//! Rune sandbox bindings for product lookup and mutation.

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

    rune_env.register_contextual(
        "create_product",
        "create_product(#{ name: string, base_cost: number|string, sales_price: number|string, product_type?: \"Goods\"|\"Services\"|\"Both\", reference?: string, remarks?: string, hsn_code?: int, tax_ids?: [int] }) -> int  // new product id",
        |_ctx| NativeBinding::Function(Arc::new(create_product)),
    );
    rune_env.register_contextual(
        "update_product",
        "update_product(#{ id: int, name: string, base_cost: number|string, sales_price: number|string, product_type?: \"Goods\"|\"Services\"|\"Both\", reference?: string, remarks?: string, hsn_code?: int, tax_ids?: [int] }) -> int  // updated product id (full replace)",
        |_ctx| NativeBinding::Function(Arc::new(update_product)),
    );
    rune_env.register_contextual(
        "search_products",
        "search_products(#{ query: string, limit?: int }) -> #{ results: [#{ id: int, name: string, reference: string|null, product_type: \"Goods\"|\"Services\"|\"Both\" }] }",
        |_ctx| NativeBinding::Function(Arc::new(search_products)),
    );
}

#[cfg(feature = "cap-llm")]
fn create_product(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let input = parse_create_args(args)?;
    let db = ctx.db.clone();
    let id = crate::rune_env::block_on_async(async move { insert_product(&db, input).await })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn update_product(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (id, input) = parse_update_args(args)?;
    let db = ctx.db.clone();
    crate::rune_env::block_on_async(async move { replace_product(&db, id, input).await })?;
    Ok(rune::Value::from(id))
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

#[cfg(feature = "cap-llm")]
mod args {
    use chrono::Utc;
    use rust_decimal::Decimal;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
    use serde::Deserialize;

    use crate::plugins::finance_common::decimal::{self, parse_decimal};
    use crate::plugins::finance_products::entities::product::{
        self, Entity as ProductEntity, ProductType,
    };
    use crate::plugins::finance_products::preferences::set_product_tax_ids;

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    pub(super) enum NumberOrString {
        Number(serde_json::Number),
        String(String),
    }

    impl NumberOrString {
        pub(super) fn into_string(self) -> String {
            match self {
                Self::Number(n) => n.to_string(),
                Self::String(s) => s,
            }
        }
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct ProductFields {
        name: String,
        #[serde(default)]
        product_type: String,
        #[serde(default)]
        reference: String,
        #[serde(default)]
        remarks: String,
        base_cost: NumberOrString,
        sales_price: NumberOrString,
        #[serde(default)]
        hsn_code: i64,
        #[serde(default)]
        tax_ids: Vec<i64>,
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct UpdateProductArgs {
        id: i64,
        #[serde(flatten)]
        fields: ProductFields,
    }

    pub(super) struct ProductInput {
        pub name: String,
        pub product_type: ProductType,
        pub reference: Option<String>,
        pub remarks: Option<String>,
        pub base_cost: Decimal,
        pub sales_price: Decimal,
        pub hsn_code: i64,
        pub tax_ids: Vec<i64>,
    }

    fn opt_string(s: String) -> Option<String> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    fn parse_product_type(raw: &str) -> Result<ProductType, String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(ProductType::default());
        }
        ProductType::parse(trimmed).ok_or_else(|| format!("invalid product_type: {raw}"))
    }

    fn parse_money(raw: NumberOrString, field: &str) -> Result<Decimal, String> {
        parse_decimal(&raw.into_string()).ok_or_else(|| format!("invalid {field}"))
    }

    fn parse_fields(parsed: ProductFields, op: &str) -> Result<ProductInput, String> {
        let name = parsed.name.trim().to_string();
        if name.is_empty() {
            return Err(format!("{op} requires name"));
        }
        Ok(ProductInput {
            name,
            product_type: parse_product_type(&parsed.product_type)?,
            reference: opt_string(parsed.reference),
            remarks: opt_string(parsed.remarks),
            base_cost: parse_money(parsed.base_cost, "base_cost")?,
            sales_price: parse_money(parsed.sales_price, "sales_price")?,
            hsn_code: parsed.hsn_code,
            tax_ids: parsed.tax_ids,
        })
    }

    pub(super) fn parse_create_args(args: &[rune::Value]) -> Result<ProductInput, String> {
        let value = args
            .first()
            .ok_or_else(|| "create_product requires an object argument".to_string())?;
        let parsed: ProductFields = serde_json::from_value(
            crate::rune_env::rune_to_json(value)
                .map_err(|e| format!("invalid create_product arguments: {e}"))?,
        )
        .map_err(|e| format!("invalid create_product arguments: {e}"))?;
        parse_fields(parsed, "create_product")
    }

    pub(super) fn parse_update_args(args: &[rune::Value]) -> Result<(i64, ProductInput), String> {
        let value = args
            .first()
            .ok_or_else(|| "update_product requires an object argument".to_string())?;
        let parsed: UpdateProductArgs = serde_json::from_value(
            crate::rune_env::rune_to_json(value)
                .map_err(|e| format!("invalid update_product arguments: {e}"))?,
        )
        .map_err(|e| format!("invalid update_product arguments: {e}"))?;
        if parsed.id <= 0 {
            return Err("update_product requires a positive product id".to_string());
        }
        Ok((parsed.id, parse_fields(parsed.fields, "update_product")?))
    }

    pub(super) async fn insert_product(
        db: &sea_orm::DatabaseConnection,
        input: ProductInput,
    ) -> Result<i64, String> {
        let now = Utc::now();
        let am = product::ActiveModel {
            name: Set(input.name),
            product_type: Set(input.product_type),
            reference: Set(input.reference),
            remarks: Set(input.remarks),
            base_cost: Set(decimal::normalize(input.base_cost)),
            sales_price: Set(decimal::normalize(input.sales_price)),
            hsn_code: Set(input.hsn_code),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        };
        let m = am.insert(db).await.map_err(|e| e.to_string())?;
        set_product_tax_ids(db, m.id, &input.tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        Ok(m.id)
    }

    pub(super) async fn replace_product(
        db: &sea_orm::DatabaseConnection,
        id: i64,
        input: ProductInput,
    ) -> Result<(), String> {
        let mut am: product::ActiveModel = ProductEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("product {id} not found"))?
            .into();
        let now = Utc::now();
        am.name = Set(input.name);
        am.product_type = Set(input.product_type);
        am.reference = Set(input.reference);
        am.remarks = Set(input.remarks);
        am.base_cost = Set(decimal::normalize(input.base_cost));
        am.sales_price = Set(decimal::normalize(input.sales_price));
        am.hsn_code = Set(input.hsn_code);
        am.updated_at = Set(Some(now));
        am.update(db).await.map_err(|e| e.to_string())?;
        set_product_tax_ids(db, id, &input.tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(feature = "cap-llm")]
use args::{insert_product, parse_create_args, parse_update_args, replace_product};

#[cfg(all(test, feature = "cap-llm"))]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::plugins::filesystem::storage::{DynFilestore, UnimplementedFilestore};
    use crate::plugins::finance_products::entities::product::ProductType;
    use crate::plugins::llm_assistant::rune_engine;
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

    fn registered_env() -> RuneEnvCapability {
        let mut cap = RuneEnvCapability::new();
        Hook.register_rune_env(&mut cap);
        cap
    }

    #[test]
    fn registers_create_update_and_search_product_bindings() {
        let names = registered_env().all_names();
        assert!(
            names.iter().any(|name| name == "create_product"),
            "expected create_product in {names:?}"
        );
        assert!(
            names.iter().any(|name| name == "update_product"),
            "expected update_product in {names:?}"
        );
        assert!(
            names.iter().any(|name| name == "search_products"),
            "expected search_products in {names:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_product_via_rune_rejects_missing_name() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_product(#{ base_cost: 1, sales_price: 2 })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("invalid create_product arguments") || error.contains("name"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_product_via_rune_rejects_empty_name() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_product(#{ name: "  ", base_cost: 1, sales_price: 2 })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("create_product requires name"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_product_via_rune_rejects_missing_argument() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "create_product(())", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("create_product requires an object argument")
                || error.contains("unsupported"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_product_via_rune_rejects_invalid_type() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_product(#{ name: "Widget", base_cost: 1, sales_price: 2, product_type: "Part" })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("invalid product_type"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_product_via_rune_rejects_invalid_base_cost() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_product(#{ name: "Widget", base_cost: "nope", sales_price: 2 })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("invalid base_cost"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_product_via_rune_accepts_object_built_from_lets() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"
let product_name = "Widget";
let cost = "nope";
let price = 25;
create_product(#{
    name: product_name,
    product_type: "Goods",
    base_cost: cost,
    sales_price: price,
    hsn_code: 1234
})
"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("invalid base_cost"),
            "object-from-lets conversion failed: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_product_via_rune_rejects_missing_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_product(#{ name: "Widget", base_cost: 1, sales_price: 2 })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("invalid update_product arguments") || error.contains("id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_product_via_rune_rejects_non_positive_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_product(#{ id: 0, name: "Widget", base_cost: 1, sales_price: 2 })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("positive product id"),
            "unexpected error payload: {out}"
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

    #[test]
    fn update_product_args_accept_object_fields() {
        use crate::rune_env::json_to_rune;
        use rust_decimal::Decimal;
        use serde_json::json;
        use std::str::FromStr;

        let value = json_to_rune(json!({
            "id": 42,
            "name": "Widget",
            "product_type": "Services",
            "base_cost": 10,
            "sales_price": "25.50",
            "hsn_code": 9983,
            "tax_ids": [1, 2]
        }))
        .expect("json to rune");
        let (id, input) = parse_update_args(&[value]).expect("parse update args");
        assert_eq!(id, 42);
        assert_eq!(input.name, "Widget");
        assert_eq!(input.product_type, ProductType::Services);
        assert_eq!(input.base_cost, Decimal::from_str("10").unwrap());
        assert_eq!(input.sales_price, Decimal::from_str("25.50").unwrap());
        assert_eq!(input.hsn_code, 9983);
        assert_eq!(input.tax_ids, vec![1, 2]);
    }
}
