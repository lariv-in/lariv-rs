//! Rune VM helper — build context, compile scripts, run `main` with limits.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use rune::module::Module;
use rune::runtime::VmError;
use rune::termcolor::NoColor;
use rune::{Context, Diagnostics, Source, Sources, Value, Vm};
use sea_orm::DatabaseConnection;
use serde_json::{Value as JsonValue, json};

use crate::{
    llm_tools::{HitlGate, HitlSource},
    plugins::filesystem::storage::DynFilestore,
    rune_env::{NativeFn, ResolvedRuneEnv, RuneEnvCapability, RuneEnvCtx},
};

use super::hitl::args_to_json;

const MAX_SOURCE_BYTES: usize = 64 * 1024;
const RUN_TIMEOUT: Duration = Duration::from_secs(5);
const HITL_RUN_TIMEOUT: Duration = Duration::from_secs(600);

/// Optional HITL registry and approval gate for one script run.
#[derive(Default)]
pub struct CompileOpts<'a> {
    pub hitl: Option<&'a dyn HitlSource>,
    pub hitl_gate: Option<&'a HitlGate>,
}

struct InvokeState {
    db: DatabaseConnection,
    store: Arc<DynFilestore>,
    session_id: Option<i64>,
    functions: HashMap<String, NativeFn>,
    hitl_functions: HashMap<String, NativeFn>,
    hitl_gate: Option<HitlGate>,
}

/// Build a Rune [`Context`] with std (no stdio) and plugin bindings on `lariv`.
pub fn build_context(
    resolved: &ResolvedRuneEnv,
    env_ctx: &RuneEnvCtx<'_>,
) -> Result<Context, String> {
    build_context_with(resolved, env_ctx, &CompileOpts::default())
}

fn build_context_with(
    resolved: &ResolvedRuneEnv,
    env_ctx: &RuneEnvCtx<'_>,
    opts: &CompileOpts<'_>,
) -> Result<Context, String> {
    let mut context = Context::with_config(false).map_err(|e| e.to_string())?;
    install_lariv_module(&mut context, resolved, env_ctx, opts)?;
    Ok(context)
}

fn install_lariv_module(
    context: &mut Context,
    resolved: &ResolvedRuneEnv,
    env_ctx: &RuneEnvCtx<'_>,
    opts: &CompileOpts<'_>,
) -> Result<(), String> {
    let mut functions = HashMap::new();
    for (name, f) in &resolved.functions {
        functions.insert(name.clone(), f.clone());
    }
    let mut hitl_functions = HashMap::new();
    if let Some(hitl) = opts.hitl {
        for (name, f) in hitl.resolve(env_ctx) {
            hitl_functions.insert(name, f);
        }
    }
    let state = Arc::new(InvokeState {
        db: env_ctx.db.clone(),
        store: Arc::clone(&env_ctx.store),
        session_id: env_ctx.session_id,
        functions,
        hitl_functions,
        hitl_gate: opts.hitl_gate.cloned(),
    });

    let mut module = Module::with_item(["lariv"]).map_err(|e| e.to_string())?;

    let state_for_fn = state.clone();
    module
        .function(
            ["invoke"],
            move |name: &str, args: Value| -> Result<Value, String> {
                invoke(&state_for_fn, name, args)
            },
        )
        .build()
        .map_err(|e| e.to_string())?;

    context.install(module).map_err(|e| e.to_string())?;
    Ok(())
}

fn unpack_args(args: Value) -> Vec<Value> {
    match args.borrow_tuple_ref() {
        Ok(tuple) if tuple.is_empty() => vec![],
        Ok(tuple) => tuple.iter().cloned().collect(),
        Err(_) => vec![args],
    }
}

fn invoke(state: &InvokeState, name: &str, args: Value) -> Result<Value, String> {
    let arg_list = unpack_args(args);
    let env_ctx = RuneEnvCtx {
        db: &state.db,
        store: Arc::clone(&state.store),
        session_id: state.session_id,
    };

    if let Some(f) = state.hitl_functions.get(name) {
        let json_args = args_to_json(&arg_list)?;
        match &state.hitl_gate {
            None => {
                return Err(format!("{name}: requires human approval"));
            }
            Some(gate) => gate(name, &json_args).map_err(|e| format!("{name}: {e}"))?,
        }
        return f(&env_ctx, &arg_list).map_err(|e| format!("{name}: {e}"));
    }

    let f = state
        .functions
        .get(name)
        .ok_or_else(|| format!("unknown function {name:?}"))?;
    f(&env_ctx, &arg_list).map_err(|e| format!("{name}: {e}"))
}

/// Compile and run Rune source; returns `{result}` or `{error}` JSON objects.
pub async fn compile_and_run(
    rune_env: &RuneEnvCapability,
    env_ctx: &RuneEnvCtx<'_>,
    source: &str,
    extra_lets: &[(String, JsonValue)],
) -> JsonValue {
    compile_and_run_with(
        rune_env,
        env_ctx,
        source,
        extra_lets,
        CompileOpts::default(),
    )
    .await
}

/// Compile and run with optional HITL bindings and approval gate.
pub async fn compile_and_run_with(
    rune_env: &RuneEnvCapability,
    env_ctx: &RuneEnvCtx<'_>,
    source: &str,
    extra_lets: &[(String, JsonValue)],
    opts: CompileOpts<'_>,
) -> JsonValue {
    if source.len() > MAX_SOURCE_BYTES {
        return json!({ "error": "source exceeds maximum size" });
    }

    let resolved = rune_env.resolve(env_ctx);
    let context = match build_context_with(&resolved, env_ctx, &opts) {
        Ok(c) => c,
        Err(e) => return json!({ "error": e }),
    };

    let hitl_names: Vec<String> = opts.hitl.map(|h| h.all_names()).unwrap_or_default();
    let full_source = wrap_source(source, &resolved, extra_lets, &hitl_names);
    let timeout = if opts.hitl_gate.is_some() {
        HITL_RUN_TIMEOUT
    } else {
        RUN_TIMEOUT
    };

    let run = async move {
        let runtime = Arc::new(context.runtime().map_err(|e| e.to_string())?);
        let mut sources = Sources::new();
        sources
            .insert(Source::new("script", &full_source).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;

        let mut diagnostics = Diagnostics::new();
        let result = rune::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .build();

        if diagnostics.has_error() {
            return Err(format_diagnostics(&diagnostics, &sources));
        }

        let unit = result.map_err(|e| e.to_string())?;
        let mut vm = Vm::new(runtime, Arc::new(unit));
        let output = match vm.call(["main"], ()) {
            Ok(v) => v,
            Err(e) => return Err(format_vm_error(&e, &sources)),
        };
        crate::rune_env::rune_to_json(&output)
    };

    match tokio::time::timeout(timeout, run).await {
        Ok(Ok(v)) => encode_result(v),
        Ok(Err(e)) => json!({ "error": e }),
        Err(_) => json!({ "error": "execution timed out" }),
    }
}

fn format_diagnostics(diagnostics: &Diagnostics, sources: &Sources) -> String {
    match emit_to_string(|out| diagnostics.emit(out, sources)) {
        Ok(text) if !text.trim().is_empty() => text,
        Ok(_) | Err(_) => "compilation failed".into(),
    }
}

fn format_vm_error(error: &VmError, sources: &Sources) -> String {
    match emit_to_string(|out| error.emit(out, sources)) {
        Ok(text) if !text.trim().is_empty() => text,
        Ok(_) | Err(_) => error.to_string(),
    }
}

fn emit_to_string<F>(emit: F) -> Result<String, String>
where
    F: FnOnce(&mut NoColor<Vec<u8>>) -> Result<(), rune::diagnostics::EmitError>,
{
    let mut buf = NoColor::new(Vec::new());
    emit(&mut buf).map_err(|e| e.to_string())?;
    String::from_utf8(buf.into_inner()).map_err(|e| e.to_string())
}

fn encode_result(v: JsonValue) -> JsonValue {
    json!({ "result": v })
}

fn wrap_source(
    source: &str,
    resolved: &ResolvedRuneEnv,
    extra_lets: &[(String, JsonValue)],
    hitl_names: &[String],
) -> String {
    let mut prelude = String::new();
    let mut names: Vec<&str> = resolved
        .functions
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    for name in hitl_names {
        if !names.contains(&name.as_str()) {
            names.push(name.as_str());
        }
    }
    if !names.is_empty() {
        prelude.push_str("use lariv::invoke;\n\n");
        names.sort_unstable();
        for name in names {
            prelude.push_str(&format!(
                "fn {name}(a) {{\n    match invoke({name:?}, a) {{\n        Ok(v) => v,\n        Err(e) => panic(e),\n    }}\n}}\n"
            ));
        }
        prelude.push('\n');
    }

    let mut lets = String::new();
    for (name, value) in resolved.statics.iter().chain(extra_lets.iter()) {
        lets.push_str(&format!("let {name} = {};\n", json_to_rune_literal(value)));
    }

    if source.contains("pub fn main") {
        if lets.is_empty() {
            prelude.push_str(source);
            prelude
        } else {
            prelude.push_str("pub fn main() {\n");
            prelude.push_str(&lets);
            prelude.push_str("inner_main()\n}\n");
            prelude.push_str(&source.replace("pub fn main", "fn inner_main"));
            prelude
        }
    } else {
        prelude.push_str("pub fn main() {\n");
        prelude.push_str(&lets);
        prelude.push_str(source);
        prelude.push_str("\n}\n");
        prelude
    }
}

fn json_to_rune_literal(v: &JsonValue) -> String {
    match v {
        JsonValue::Null => "()".into(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => format!("{s:?}"),
        JsonValue::Array(items) => {
            let inner: Vec<_> = items.iter().map(json_to_rune_literal).collect();
            format!("[{}]", inner.join(", "))
        }
        JsonValue::Object(map) => {
            let inner: Vec<_> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k, json_to_rune_literal(v)))
                .collect();
            format!("#{{ {} }}", inner.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::plugins::filesystem::storage::UnimplementedFilestore;

    fn test_env_ctx<'a>(
        db: &'a DatabaseConnection,
        store: &'a Arc<DynFilestore>,
    ) -> RuneEnvCtx<'a> {
        RuneEnvCtx {
            db,
            store: Arc::clone(store),
            session_id: None,
        }
    }

    #[tokio::test]
    async fn eval_snippet_returns_result() {
        let cap = RuneEnvCapability::new();
        let db = DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = compile_and_run(&cap, &env_ctx, "1 + 2", &[]).await;
        assert_eq!(out["result"], json!(3));
    }

    #[tokio::test]
    async fn compile_error_returns_error_payload() {
        let cap = RuneEnvCapability::new();
        let db = DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = compile_and_run(&cap, &env_ctx, "let x: int = \"nope\";", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("error")
                && (error.contains("mismatched types")
                    || error.contains("expected")
                    || error.contains("int")),
            "expected rust-style compile diagnostic, got: {out}"
        );
    }

    #[tokio::test]
    async fn registered_function_is_callable() {
        use crate::rune_env::NativeBinding;

        let mut cap = RuneEnvCapability::new();
        cap.register_contextual("double", "double(n: int) -> int", |_ctx| {
            NativeBinding::Function(Arc::new(|_ctx, args| {
                let n = rune::from_value::<i64>(
                    args.first()
                        .cloned()
                        .ok_or_else(|| "missing arg".to_string())?,
                )
                .map_err(|e| e.to_string())?;
                Ok(rune::Value::from(n.saturating_mul(2)))
            }))
        });
        let db = DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = compile_and_run(&cap, &env_ctx, "double(21)", &[]).await;
        assert_eq!(out["result"], json!(42), "{out}");
    }

    #[tokio::test]
    async fn native_error_includes_function_name() {
        use crate::rune_env::NativeBinding;

        let mut cap = RuneEnvCapability::new();
        cap.register_contextual("boom", "boom() -> !", |_ctx| {
            NativeBinding::Function(Arc::new(|_ctx, _args| Err("nope".into())))
        });
        let db = DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = compile_and_run(&cap, &env_ctx, "boom(1)", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("boom: nope"),
            "expected function-prefixed native error, got: {out}"
        );
    }

    #[tokio::test]
    async fn object_literal_returns_json_object() {
        let cap = RuneEnvCapability::new();
        let db = DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = compile_and_run(&cap, &env_ctx, "#{ site_id: 1, name: \"Aayush\" }", &[]).await;
        assert_eq!(
            out["result"],
            json!({ "site_id": 1, "name": "Aayush" }),
            "{out}"
        );
    }

    fn hitl_counting_cap(
        calls: Arc<std::sync::atomic::AtomicUsize>,
    ) -> crate::plugins::llm_assistant::hitl::HitlCapability {
        use crate::plugins::llm_assistant::hitl::HitlCapability;
        use crate::rune_env::NativeBinding;
        use std::sync::atomic::Ordering;

        let mut cap = HitlCapability::new();
        cap.register(
            "double_hitl",
            "double_hitl(n: int) -> int  // requires approval",
            move |_ctx| {
                let calls = Arc::clone(&calls);
                NativeBinding::Function(Arc::new(move |_ctx, args| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    let n = rune::from_value::<i64>(
                        args.first()
                            .cloned()
                            .ok_or_else(|| "missing arg".to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                    Ok(rune::Value::from(n.saturating_mul(2)))
                }))
            },
        );
        cap
    }

    #[tokio::test]
    async fn hitl_without_gate_does_not_run() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls = Arc::new(AtomicUsize::new(0));
        let rune = RuneEnvCapability::new();
        let hitl = hitl_counting_cap(Arc::clone(&calls));
        let db = DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = compile_and_run_with(
            &rune,
            &env_ctx,
            "double_hitl(21)",
            &[],
            CompileOpts {
                hitl: Some(&hitl),
                hitl_gate: None,
            },
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("requires human approval"),
            "expected fail-closed HITL, got: {out}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn hitl_deny_does_not_run() {
        use crate::plugins::llm_assistant::hitl::deny_all_gate;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls = Arc::new(AtomicUsize::new(0));
        let rune = RuneEnvCapability::new();
        let hitl = hitl_counting_cap(Arc::clone(&calls));
        let gate = deny_all_gate();
        let db = DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = compile_and_run_with(
            &rune,
            &env_ctx,
            "double_hitl(21)",
            &[],
            CompileOpts {
                hitl: Some(&hitl),
                hitl_gate: Some(&gate),
            },
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("double_hitl: denied"),
            "expected denied HITL, got: {out}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn hitl_approve_runs_native_fn() {
        use crate::plugins::llm_assistant::hitl::approve_all_gate;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls = Arc::new(AtomicUsize::new(0));
        let rune = RuneEnvCapability::new();
        let hitl = hitl_counting_cap(Arc::clone(&calls));
        let gate = approve_all_gate();
        let db = DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = compile_and_run_with(
            &rune,
            &env_ctx,
            "double_hitl(21)",
            &[],
            CompileOpts {
                hitl: Some(&hitl),
                hitl_gate: Some(&gate),
            },
        )
        .await;
        assert_eq!(out["result"], json!(42), "{out}");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn wrap_source_includes_hitl_names_in_prelude() {
        let resolved = ResolvedRuneEnv {
            statics: vec![],
            functions: vec![],
        };
        let source = wrap_source(
            "delete_draft_invoice(#{ id: 1 })",
            &resolved,
            &[],
            &["delete_draft_invoice".to_string()],
        );
        assert!(
            source.contains("fn delete_draft_invoice(a)"),
            "expected HITL wrapper in prelude, got:\n{source}"
        );
        assert!(source.contains(r#"invoke("delete_draft_invoice""#));
    }
}
