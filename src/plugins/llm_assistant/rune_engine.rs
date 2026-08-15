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
    plugins::filesystem::storage::DynFilestore,
    rune_env::{NativeFn, ResolvedRuneEnv, RuneEnvCapability, RuneEnvCtx},
};

const MAX_SOURCE_BYTES: usize = 64 * 1024;
const RUN_TIMEOUT: Duration = Duration::from_secs(5);

struct InvokeState {
    db: DatabaseConnection,
    store: Arc<DynFilestore>,
    functions: HashMap<String, NativeFn>,
}

/// Build a Rune [`Context`] with std (no stdio) and plugin bindings on `lariv`.
pub fn build_context(
    resolved: &ResolvedRuneEnv,
    env_ctx: &RuneEnvCtx<'_>,
) -> Result<Context, String> {
    let mut context = Context::with_config(false).map_err(|e| e.to_string())?;
    install_lariv_module(&mut context, resolved, env_ctx)?;
    Ok(context)
}

fn install_lariv_module(
    context: &mut Context,
    resolved: &ResolvedRuneEnv,
    env_ctx: &RuneEnvCtx<'_>,
) -> Result<(), String> {
    let mut functions = HashMap::new();
    for (name, f) in &resolved.functions {
        functions.insert(name.clone(), f.clone());
    }
    let state = Arc::new(InvokeState {
        db: env_ctx.db.clone(),
        store: Arc::clone(&env_ctx.store),
        functions,
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

fn invoke(state: &InvokeState, name: &str, args: Value) -> Result<Value, String> {
    let f = state
        .functions
        .get(name)
        .ok_or_else(|| format!("unknown function {name:?}"))?;

    let arg_list = match args.borrow_tuple_ref() {
        Ok(tuple) if tuple.is_empty() => vec![],
        Ok(tuple) => tuple.iter().cloned().collect(),
        Err(_) => vec![args],
    };

    let env_ctx = RuneEnvCtx {
        db: &state.db,
        store: Arc::clone(&state.store),
    };
    f(&env_ctx, &arg_list).map_err(|e| format!("{name}: {e}"))
}

/// Compile and run Rune source; returns `{result}` or `{error}` JSON objects.
pub async fn compile_and_run(
    rune_env: &RuneEnvCapability,
    env_ctx: &RuneEnvCtx<'_>,
    source: &str,
    extra_lets: &[(String, JsonValue)],
) -> JsonValue {
    if source.len() > MAX_SOURCE_BYTES {
        return json!({ "error": "source exceeds maximum size" });
    }

    let resolved = rune_env.resolve(env_ctx);
    let context = match build_context(&resolved, env_ctx) {
        Ok(c) => c,
        Err(e) => return json!({ "error": e }),
    };

    let full_source = wrap_source(source, &resolved, extra_lets);

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

    match tokio::time::timeout(RUN_TIMEOUT, run).await {
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
) -> String {
    let mut prelude = String::new();
    if !resolved.functions.is_empty() {
        prelude.push_str("use lariv::invoke;\n\n");
        let mut names: Vec<_> = resolved
            .functions
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
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
        cap.register_contextual("double", |_ctx| {
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
        cap.register_contextual("boom", |_ctx| {
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
}
