//! HITL-gated Rune bindings for service scripts (require human approval).

use lariv_plugin_llm_assistant::hitl::{HitlCapability, HitlRegistrar};

/// Registers HITL service helpers onto the assistant HITL capability.
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
        "delete_service",
        "delete_service(#{ id: int }) -> ()  // requires human approval",
        |_ctx| NativeBinding::Function(Arc::new(delete_service)),
    );
}

fn delete_service(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let service_id = parse_delete_args(args)?;
    let db = ctx.db.clone();
    lariv_core::rune_env::block_on_async(async move {
        use sea_orm::EntityTrait;
        let found = crate::entities::service::Entity::find_by_id(service_id)
            .one(&db)
            .await
            .map_err(|e| e.to_string())?;
        if found.is_none() {
            return Err(format!("service {service_id} not found"));
        }
        crate::entities::service::Entity::delete_by_id(service_id)
            .exec(&db)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    })?;
    Ok(rune::Value::from(()))
}

fn parse_delete_args(args: &[rune::Value]) -> Result<i64, String> {
    let value = args
        .first()
        .ok_or_else(|| "delete_service requires an object argument".to_string())?;
    let parsed: DeleteArgs = serde_json::from_value(
        lariv_core::rune_env::rune_to_json(value)
            .map_err(|e| format!("invalid delete_service arguments: {e}"))?,
    )
    .map_err(|e| format!("invalid delete_service arguments: {e}"))?;
    if parsed.id <= 0 {
        return Err("delete_service requires a positive service id".to_string());
    }
    Ok(parsed.id)
}

#[derive(Debug, serde::Deserialize)]
struct DeleteArgs {
    id: i64,
}

#[cfg(all(test, feature = "plugin-llm-assistant"))]
mod tests {
    use super::*;
    use std::sync::Arc;

    use lariv_core::rune_env::{RuneEnvCapability, RuneEnvCtx};
    use lariv_plugin_filesystem::storage::{DynFilestore, UnimplementedFilestore};
    use lariv_plugin_llm_assistant::hitl::{HitlCapability, approve_all_gate};
    use lariv_plugin_llm_assistant::rune_engine::{self, CompileOpts};

    fn test_env_ctx<'a>(
        db: &'a sea_orm::DatabaseConnection,
        store: &'a Arc<DynFilestore>,
    ) -> RuneEnvCtx<'a> {
        RuneEnvCtx {
            db,
            store: Arc::clone(store),
            session_id: None,
        }
    }

    fn registered_hitl() -> HitlCapability {
        let mut cap = HitlCapability::new();
        Hook.register_hitl(&mut cap);
        cap
    }

    #[test]
    fn registers_delete_service() {
        let names = registered_hitl().all_names();
        assert!(
            names.iter().any(|name| name == "delete_service"),
            "expected delete_service in {names:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn delete_rejects_missing_id() {
        let hitl = registered_hitl();
        let rune = RuneEnvCapability::new();
        let gate = approve_all_gate();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run_with(
            &rune,
            &env_ctx,
            r#"delete_service(#{ })"#,
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
            error.contains("invalid delete_service arguments") || error.contains("id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn delete_rejects_non_positive_id() {
        let hitl = registered_hitl();
        let rune = RuneEnvCapability::new();
        let gate = approve_all_gate();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run_with(
            &rune,
            &env_ctx,
            r#"delete_service(#{ id: 0 })"#,
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
            error.contains("positive service id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn delete_rejects_missing_argument() {
        let hitl = registered_hitl();
        let rune = RuneEnvCapability::new();
        let gate = approve_all_gate();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run_with(
            &rune,
            &env_ctx,
            "delete_service(())",
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
            error.contains("delete_service requires an object argument")
                || error.contains("unsupported")
                || error.contains("invalid delete_service arguments"),
            "unexpected error payload: {out}"
        );
    }
}
