//! HITL-gated Rune bindings for invoice scripts (require human approval).

use lariv_plugin_llm_assistant::hitl::{HitlCapability, HitlRegistrar};

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

    use lariv_core::rune_env::NativeBinding;

    hitl.register(
        "delete_draft_invoice",
        "delete_draft_invoice(#{ id: int }) -> ()  // requires human approval; invoice must still be draft",
        |_ctx| NativeBinding::Function(Arc::new(delete_draft_invoice)),
    );
    hitl.register(
        "post_draft_invoice",
        "post_draft_invoice(#{ draft_id: int, posted_at?: string, timezone?: string }) -> int  // requires human approval. Posts the draft and returns the posted invoice id. The draft is removed.",
        |_ctx| NativeBinding::Function(Arc::new(post_draft_invoice)),
    );
    hitl.register(
        "confirm_reset_posted_invoice_to_draft",
        "confirm_reset_posted_invoice_to_draft(#{ id: int }) -> int  // requires human approval. Same as Reset to draft: copies the posted invoice into a new draft, then deletes the records listed by request_reset_posted_invoice_to_draft. Returns the new draft id.",
        |_ctx| NativeBinding::Function(Arc::new(confirm_reset_posted_invoice_to_draft)),
    );
}

fn post_draft_invoice(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use chrono::Utc;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct PostArgs {
        draft_id: i64,
        #[serde(default)]
        posted_at: Option<String>,
        #[serde(default)]
        timezone: Option<String>,
    }

    let value = args
        .first()
        .ok_or_else(|| "post_draft_invoice requires an object argument".to_string())?;
    let parsed: PostArgs = serde_json::from_value(
        lariv_core::rune_env::rune_to_json(value)
            .map_err(|error| format!("invalid post_draft_invoice arguments: {error}"))?,
    )
    .map_err(|error| format!("invalid post_draft_invoice arguments: {error}"))?;
    if parsed.draft_id <= 0 {
        return Err("post_draft_invoice requires a positive draft invoice id".to_string());
    }
    let tz = parsed.timezone.unwrap_or_else(|| "UTC".to_string());
    let posted_at = match parsed.posted_at.as_deref() {
        Some(raw) if !raw.trim().is_empty() => crate::logic::parse_invoice_datetime(raw, &tz),
        _ => Utc::now(),
    };
    let draft_id = parsed.draft_id;
    let db = ctx.db.clone();
    let posted_id = lariv_core::rune_env::block_on_async(async move {
        let posted = crate::logic::draft_new_posted(&db, draft_id, posted_at, &tz).await?;
        Ok::<i64, String>(posted.id)
    })?;
    Ok(rune::Value::from(posted_id))
}

fn confirm_reset_posted_invoice_to_draft(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let posted_id = parse_reset_args(args)?;
    let db = ctx.db.clone();
    let draft_id = lariv_core::rune_env::block_on_async(async move {
        crate::logic::reset_posted_invoice_to_draft(&db, posted_id).await
    })?;
    Ok(rune::Value::from(draft_id))
}

fn parse_reset_args(args: &[rune::Value]) -> Result<i64, String> {
    let value = args.first().ok_or_else(|| {
        "confirm_reset_posted_invoice_to_draft requires an object argument".to_string()
    })?;
    let parsed: DeleteArgs =
        serde_json::from_value(lariv_core::rune_env::rune_to_json(value).map_err(|e| {
            format!("invalid confirm_reset_posted_invoice_to_draft arguments: {e}")
        })?)
        .map_err(|e| format!("invalid confirm_reset_posted_invoice_to_draft arguments: {e}"))?;
    if parsed.id <= 0 {
        return Err(
            "confirm_reset_posted_invoice_to_draft requires a positive posted invoice id"
                .to_string(),
        );
    }
    Ok(parsed.id)
}

fn delete_draft_invoice(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let draft_id = parse_delete_args(args)?;
    let db = ctx.db.clone();
    lariv_core::rune_env::block_on_async(async move {
        use sea_orm::EntityTrait;
        let found = crate::entities::DraftInvoiceEntity::find_by_id(draft_id)
            .one(&db)
            .await
            .map_err(|e| e.to_string())?;
        if found.is_none() {
            return Err(format!("draft invoice {draft_id} not found"));
        }
        crate::logic::delete_draft(&db, draft_id).await
    })?;
    Ok(rune::Value::from(()))
}

fn parse_delete_args(args: &[rune::Value]) -> Result<i64, String> {
    let value = args
        .first()
        .ok_or_else(|| "delete_draft_invoice requires an object argument".to_string())?;
    let parsed: DeleteArgs = serde_json::from_value(
        lariv_core::rune_env::rune_to_json(value)
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
            store: lariv_plugin_filesystem::storage::erase_store(Arc::clone(store)),
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
        assert!(
            names.iter().any(|name| name == "post_draft_invoice"),
            "expected post_draft_invoice in {names:?}"
        );
        assert!(
            names
                .iter()
                .any(|name| name == "confirm_reset_posted_invoice_to_draft"),
            "expected confirm_reset_posted_invoice_to_draft in {names:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn post_rejects_non_positive_draft_id() {
        let hitl = registered_hitl();
        let rune = RuneEnvCapability::new();
        let gate = approve_all_gate();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run_with(
            &rune,
            &env_ctx,
            r#"post_draft_invoice(#{ draft_id: 0 })"#,
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
    async fn reset_rejects_non_positive_id() {
        let hitl = registered_hitl();
        let rune = RuneEnvCapability::new();
        let gate = approve_all_gate();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run_with(
            &rune,
            &env_ctx,
            r#"confirm_reset_posted_invoice_to_draft(#{ id: 0 })"#,
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
            error.contains("positive posted invoice id"),
            "unexpected error payload: {out}"
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
