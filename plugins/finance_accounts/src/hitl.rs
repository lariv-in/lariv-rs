//! HITL-gated confirmation for deleting a journal entry.

use lariv_plugin_llm_assistant::hitl::{HitlCapability, HitlRegistrar};

/// Registers the journal-entry delete confirmation.
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
        "confirm_delete_journal_entry",
        "confirm_delete_journal_entry(#{ id: int }) -> ()  // requires human approval. Permanently deletes the journal entry and the records listed by request_delete_journal_entry. The journal must be mutable.",
        |_ctx| NativeBinding::Function(Arc::new(confirm_delete_journal_entry)),
    );
}

fn confirm_delete_journal_entry(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = parse_id(args, "confirm_delete_journal_entry")?;
    let db = ctx.db.clone();
    lariv_core::rune_env::block_on_async(async move {
        crate::logic::journal::delete_mutable_journal_entry(&db, id).await
    })?;
    Ok(rune::Value::from(()))
}

fn parse_id(args: &[rune::Value], op: &str) -> Result<i64, String> {
    #[derive(serde::Deserialize)]
    struct IdArgs {
        id: i64,
    }

    let value = args
        .first()
        .ok_or_else(|| format!("{op} requires an object argument"))?;
    let parsed: IdArgs = serde_json::from_value(
        lariv_core::rune_env::rune_to_json(value)
            .map_err(|error| format!("invalid {op} arguments: {error}"))?,
    )
    .map_err(|error| format!("invalid {op} arguments: {error}"))?;
    if parsed.id <= 0 {
        return Err(format!("{op} requires a positive journal entry id"));
    }
    Ok(parsed.id)
}

#[cfg(all(test, feature = "plugin-llm-assistant"))]
mod tests {
    use std::sync::Arc;

    use lariv_core::rune_env::{RuneEnvCapability, RuneEnvCtx};
    use lariv_plugin_filesystem::storage::{DynFilestore, UnimplementedFilestore};
    use lariv_plugin_llm_assistant::hitl::{HitlCapability, approve_all_gate};
    use lariv_plugin_llm_assistant::rune_engine::{self, CompileOpts};

    use super::Hook;

    fn registered_hitl() -> HitlCapability {
        let mut cap = HitlCapability::new();
        Hook.register_hitl(&mut cap);
        cap
    }

    #[test]
    fn registers_confirm_delete_journal_entry() {
        let names = registered_hitl().all_names();
        assert!(
            names
                .iter()
                .any(|name| name == "confirm_delete_journal_entry"),
            "expected confirm_delete_journal_entry in {names:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn confirm_rejects_non_positive_id() {
        let hitl = registered_hitl();
        let rune = RuneEnvCapability::new();
        let gate = approve_all_gate();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = RuneEnvCtx {
            db: &db,
            store: lariv_plugin_filesystem::storage::erase_store(store),
            session_id: None,
        };
        let out = rune_engine::compile_and_run_with(
            &rune,
            &env_ctx,
            r#"confirm_delete_journal_entry(#{ id: 0 })"#,
            &[],
            CompileOpts {
                hitl: Some(&hitl),
                hitl_gate: Some(&gate),
            },
        )
        .await;
        let error = out
            .get("error")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("positive journal entry id"),
            "unexpected error payload: {out}"
        );
    }
}
