//! Rune VM helper — build context, compile scripts, run `main` with limits.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use rune::module::Module;
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
pub fn build_context(resolved: &ResolvedRuneEnv, env_ctx: &RuneEnvCtx<'_>) -> Result<Context, String> {
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
        .function(["invoke"], move |name: &str, args: Value| -> Result<Value, String> {
            invoke(&state_for_fn, name, args)
        })
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
    f(&env_ctx, &arg_list)
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
            .insert(Source::memory(&full_source).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;

        let mut diagnostics = Diagnostics::new();
        let result = rune::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .build();

        if diagnostics.has_error() {
            return Err("compilation failed".into());
        }

        let unit = result.map_err(|e| e.to_string())?;
        let mut vm = Vm::new(runtime, Arc::new(unit));
        let output = vm
            .call(["main"], ())
            .map_err(|e| e.to_string())?;
        value_to_json(output)
    };

    match tokio::time::timeout(RUN_TIMEOUT, run).await {
        Ok(Ok(v)) => encode_result(v),
        Ok(Err(e)) => json!({ "error": e }),
        Err(_) => json!({ "error": "execution timed out" }),
    }
}

fn encode_result(v: JsonValue) -> JsonValue {
    match serde_json::to_string(&v) {
        Ok(s) => json!({ "result": s }),
        Err(_) => json!({ "result": v.to_string() }),
    }
}

fn value_to_json(v: Value) -> Result<JsonValue, String> {
    if let Ok(x) = rune::from_value::<i64>(v.clone()) {
        return Ok(json!(x));
    }
    if let Ok(x) = rune::from_value::<f64>(v.clone()) {
        return Ok(json!(x));
    }
    if let Ok(x) = rune::from_value::<bool>(v.clone()) {
        return Ok(json!(x));
    }
    if let Ok(x) = rune::from_value::<String>(v.clone()) {
        return Ok(json!(x));
    }
    if let Ok(tuple) = v.borrow_tuple_ref() {
        if tuple.is_empty() {
            return Ok(JsonValue::Null);
        }
        let items: Result<Vec<_>, _> = tuple.iter().map(|x| value_to_json(x.clone())).collect();
        return Ok(JsonValue::Array(items?));
    }
    Err("unsupported result type".into())
}

fn wrap_source(
    source: &str,
    resolved: &ResolvedRuneEnv,
    extra_lets: &[(String, JsonValue)],
) -> String {
    let mut out = String::new();
    let has_functions = !resolved.functions.is_empty();
    if has_functions {
        out.push_str("use lariv::invoke;\n\n");
    }
    for (name, value) in resolved.statics.iter().chain(extra_lets.iter()) {
        out.push_str(&format!("let {name} = {};\n", json_to_rune_literal(value)));
    }
    if has_functions {
        for (name, _) in &resolved.functions {
            out.push_str(&format!("let {name} = |a| invoke({name:?}, a);\n"));
        }
        out.push('\n');
    }
    if source.contains("pub fn main") {
        out.push_str(source);
    } else {
        out.push_str("pub fn main() {\n");
        out.push_str(source);
        out.push_str("\n}\n");
    }
    out
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
            format!("{{ {} }}", inner.join(", "))
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
        assert_eq!(out["result"], json!("3"));
    }

    #[tokio::test]
    async fn compile_error_returns_error_payload() {
        let cap = RuneEnvCapability::new();
        let db = DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = compile_and_run(&cap, &env_ctx, "let x: int = \"nope\";", &[]).await;
        assert!(out.get("error").is_some());
    }
}
