//! HITL-gated Rune bindings for invoice scripts (require human approval).

use crate::plugins::llm_assistant::hitl::{HitlCapability, HitlRegistrar};

/// Registers HITL invoice helpers onto the assistant HITL capability.
#[derive(Clone, Copy, Default)]
pub struct Hook;

impl HitlRegistrar for Hook {
    fn register_hitl(self, hitl: &mut HitlCapability) {
        register(hitl);
    }
}

fn register(hitl: &mut HitlCapability) {
    use std::sync::Arc;

    use crate::rune_env::NativeBinding;

    hitl.register(
        "delete_draft_invoice",
        "delete_draft_invoice(#{ id: int }) -> ()  // requires human approval; invoice must still be draft",
        |_ctx| NativeBinding::Function(Arc::new(delete_draft_invoice)),
    );
}

fn delete_draft_invoice(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let draft_id = parse_delete_args(args)?;
    let db = ctx.db.clone();
    crate::rune_env::block_on_async(async move {
        use sea_orm::EntityTrait;
        let found =
            crate::plugins::finance_invoices::entities::DraftInvoiceEntity::find_by_id(draft_id)
                .one(&db)
                .await
                .map_err(|e| e.to_string())?;
        if found.is_none() {
            return Err(format!("draft invoice {draft_id} not found"));
        }
        crate::plugins::finance_invoices::logic::delete_draft(&db, draft_id).await
    })?;
    Ok(rune::Value::from(()))
}

fn parse_delete_args(args: &[rune::Value]) -> Result<i64, String> {
    let value = args
        .first()
        .ok_or_else(|| "delete_draft_invoice requires an object argument".to_string())?;
    let parsed: DeleteArgs = serde_json::from_value(
        crate::rune_env::rune_to_json(value)
            .map_err(|e| format!("invalid delete_draft_invoice arguments: {e}"))?,
    )
    .map_err(|e| format!("invalid delete_draft_invoice arguments: {e}"))?;
    if parsed.id <= 0 {
        return Err("delete_draft_invoice requires a positive draft invoice id".to_string());
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

    use crate::plugins::filesystem::storage::{DynFilestore, UnimplementedFilestore};
    use crate::plugins::llm_assistant::hitl::{HitlCapability, approve_all_gate};
    use crate::plugins::llm_assistant::rune_engine::{self, CompileOpts};
    use crate::rune_env::{RuneEnvCapability, RuneEnvCtx};

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
    fn registers_delete_draft_invoice() {
        let names = registered_hitl().all_names();
        assert!(
            names.iter().any(|name| name == "delete_draft_invoice"),
            "expected delete_draft_invoice in {names:?}"
        );
    }

    // Sealed-state copy ("invoice is {state} and cannot be changed") is owned by
    // `logic::delete_draft` → `err_if_draft_sealed`. See `err_if_not_draft_state_*`
    // tests in `logic/draft.rs`.

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
            r#"delete_draft_invoice(#{ })"#,
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
            error.contains("invalid delete_draft_invoice arguments") || error.contains("id"),
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
            r#"delete_draft_invoice(#{ id: 0 })"#,
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
            error.contains("positive draft invoice id"),
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
            "delete_draft_invoice(())",
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
            error.contains("delete_draft_invoice requires an object argument")
                || error.contains("unsupported")
                || error.contains("invalid delete_draft_invoice arguments"),
            "unexpected error payload: {out}"
        );
    }
}
