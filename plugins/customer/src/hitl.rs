//! HITL-gated Rune bindings for customer deletes.

use lariv_plugin_llm_assistant::hitl::{HitlCapability, HitlRegistrar};

/// Registers HITL customer helpers onto the assistant HITL capability.
#[derive(Clone, Copy, Default)]
pub struct Hook;

impl HitlRegistrar for Hook {
    fn register_hitl(self, hitl: &mut HitlCapability) {
        register(hitl);
    }
}

fn register(hitl: &mut HitlCapability) {
    use std::sync::Arc;

    use lariv_core::rune_env::NativeBinding;

    hitl.register(
        "delete_customer",
        "delete_customer(#{ id: int }) -> ()  // requires human approval",
        |_ctx| NativeBinding::Function(Arc::new(delete_customer)),
    );
}

fn delete_customer(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = parse_id(args)?;
    let db = ctx.db.clone();
    lariv_core::rune_env::block_on_async(
        async move { crate::logic::delete_customer(&db, id).await },
    )?;
    Ok(rune::Value::from(()))
}

fn parse_id(args: &[rune::Value]) -> Result<i64, String> {
    #[derive(serde::Deserialize)]
    struct IdArgs {
        #[serde(default)]
        id: i64,
    }
    let value = args
        .first()
        .ok_or_else(|| "delete_customer requires an object argument".to_string())?;
    let parsed: IdArgs = serde_json::from_value(
        lariv_core::rune_env::rune_to_json(value)
            .map_err(|e| format!("invalid delete_customer arguments: {e}"))?,
    )
    .map_err(|e| format!("invalid delete_customer arguments: {e}"))?;
    if parsed.id <= 0 {
        return Err("delete_customer requires a positive customer id".to_string());
    }
    Ok(parsed.id)
}

#[cfg(all(test, feature = "plugin-llm-assistant"))]
mod tests {
    use super::*;
    use std::sync::Arc;

    use lariv_core::rune_env::{RuneEnvCapability, RuneEnvCtx};
    use lariv_plugin_llm_assistant::hitl::{HitlCapability, approve_all_gate};
    use lariv_plugin_llm_assistant::rune_engine::{self, CompileOpts};

    fn test_env_ctx(db: &sea_orm::DatabaseConnection) -> RuneEnvCtx<'_> {
        RuneEnvCtx {
            db,
            store: Arc::new(()) as Arc<dyn std::any::Any + Send + Sync>,
            session_id: None,
        }
    }

    fn registered_hitl() -> HitlCapability {
        let mut cap = HitlCapability::new();
        Hook.register_hitl(&mut cap);
        cap
    }

    #[test]
    fn registers_delete_customer() {
        let names = registered_hitl().all_names();
        assert!(
            names.iter().any(|name| name == "delete_customer"),
            "expected delete_customer in {names:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn delete_rejects_missing_id() {
        let hitl = registered_hitl();
        let rune = RuneEnvCapability::new();
        let gate = approve_all_gate();
        let db = sea_orm::DatabaseConnection::default();
        let env_ctx = test_env_ctx(&db);
        let out = rune_engine::compile_and_run_with(
            &rune,
            &env_ctx,
            r#"delete_customer(#{ })"#,
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
            error.contains("positive customer id"),
            "unexpected error payload: {out}"
        );
    }
}
