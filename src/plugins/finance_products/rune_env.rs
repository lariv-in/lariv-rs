//! Rune sandbox bindings for product lookup.

use crate::rune_env::{RuneEnvCapability, RuneEnvRegistrar};

/// Registers product helpers onto the assistant Rune environment.
#[derive(Clone, Copy, Default)]
pub struct Hook;

impl RuneEnvRegistrar for Hook {
    fn register_rune_env(self, rune_env: &mut RuneEnvCapability) {
        #[cfg(feature = "cap-llm")]
        register(rune_env);
        #[cfg(not(feature = "cap-llm"))]
        let _ = rune_env;
    }
}

#[cfg(feature = "cap-llm")]
fn register(rune_env: &mut RuneEnvCapability) {
    use std::sync::Arc;

    use crate::rune_env::NativeBinding;

    rune_env.register_contextual("search_products", |_ctx| {
        NativeBinding::Function(Arc::new(search_products))
    });
}

#[cfg(feature = "cap-llm")]
fn search_products(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use serde::Deserialize;
    use serde_json::json;

    use crate::db::trigram;
    use crate::plugins::finance_products::entities::product::{self, Entity as ProductEntity};
    use crate::rune_env::{block_on_async, json_to_rune, rune_to_json};

    #[derive(Debug, Deserialize, Default)]
    struct SearchArgs {
        #[serde(default)]
        query: String,
        #[serde(default)]
        limit: u64,
    }

    let value = args
        .first()
        .ok_or_else(|| "search_products requires an object argument".to_string())?;
    let parsed: SearchArgs = serde_json::from_value(rune_to_json(value)?)
        .map_err(|e| format!("invalid search_products arguments: {e}"))?;
    let query = parsed.query.trim().to_string();
    if query.is_empty() {
        return Err("search_products requires query".into());
    }
    let limit = trigram::clamp_search_limit(parsed.limit);
    let db = ctx.db.clone();
    let rows = block_on_async(async move {
        trigram::search::<ProductEntity, _>(
            &db,
            &[product::Column::Name, product::Column::Reference],
            &query,
            limit,
        )
        .await
    })
    .map_err(|e| e.to_string())?;
    json_to_rune(json!({
        "results": rows.into_iter().map(|p| json!({
            "id": p.id,
            "name": p.name,
            "reference": p.reference,
            "product_type": p.product_type.as_str(),
        })).collect::<Vec<_>>(),
    }))
}

#[cfg(all(test, feature = "cap-llm"))]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::plugins::filesystem::storage::{DynFilestore, UnimplementedFilestore};
    use crate::plugins::llm_assistant::rune_engine;
    use crate::rune_env::{RuneEnvCapability, RuneEnvCtx};

    fn test_env_ctx<'a>(
        db: &'a sea_orm::DatabaseConnection,
        store: &'a Arc<DynFilestore>,
    ) -> RuneEnvCtx<'a> {
        RuneEnvCtx {
            db,
            store: Arc::clone(store),
        }
    }

    fn registered_env() -> RuneEnvCapability {
        let mut cap = RuneEnvCapability::new();
        Hook.register_rune_env(&mut cap);
        cap
    }

    #[test]
    fn registers_search_products_binding() {
        let names = registered_env().all_names();
        assert!(
            names.iter().any(|name| name == "search_products"),
            "expected search_products in {names:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn search_products_rejects_missing_query() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "search_products(#{})", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(error.contains("query"), "unexpected error payload: {out}");
    }
}
