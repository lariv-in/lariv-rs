//! Rune sandbox bindings for product lookup and mutation.

use lariv_core::rune_env::{RuneEnvCapability, RuneEnvRegistrar};

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

    use lariv_core::rune_env::NativeBinding;

    rune_env.register_contextual(
        "create_product",
        "create_product(#{ name: string, product_kind?: string, sales_price_formula: string, remarks?: string, hsn_code?: int, tax_ids?: [int], variables?: object|[{string}], journal_id?: int, journal_lines?: [{ account_id: int, formula: string }] }) -> int  // new product id. product_kind is Goods (default) or Services. hsn_code is the HSN/SAC code (required). sales_price_formula is the price of one product; invoice quantity multiplies it. journal_lines formulas may use quantity, unit_rate, pre_tax, sales_price, and the product variables. Lines must balance.",
        |_ctx| NativeBinding::Function(Arc::new(create_product)),
    );
    rune_env.register_contextual(
        "update_product",
        "update_product(#{ id: int, name: string, product_kind?: string, sales_price_formula: string, remarks?: string, hsn_code?: int, tax_ids?: [int], variables?: object|[{string}], journal_id?: int, journal_lines?: [{ account_id: int, formula: string }] }) -> int  // updated product id (full replace). product_kind is Goods or Services. hsn_code is the HSN/SAC code (required). sales_price_formula is the price of one product; invoice quantity multiplies it. journal_lines formulas may use quantity, unit_rate, pre_tax, sales_price, and the product variables.",
        |_ctx| NativeBinding::Function(Arc::new(update_product)),
    );
    rune_env.register_contextual(
        "search_products",
        "search_products(#{ query: string, limit?: int }) -> #{ results: [#{ id: int, name: string, product_kind: string, remarks: string|null, hsn_code: int|null }] }",
        |_ctx| NativeBinding::Function(Arc::new(search_products)),
    );
}

#[cfg(feature = "cap-llm")]
fn create_product(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let input = parse_create_args(args)?;
    let db = ctx.db.clone();
    let id = lariv_core::rune_env::block_on_async(async move { insert_product(&db, input).await })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn update_product(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (id, input) = parse_update_args(args)?;
    let db = ctx.db.clone();
    lariv_core::rune_env::block_on_async(async move { replace_product(&db, id, input).await })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn search_products(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use serde::Deserialize;
    use serde_json::json;

    use crate::entities::product::{self, Entity as ProductEntity};
    use lariv_core::db::trigram;
    use lariv_core::rune_env::{block_on_async, json_to_rune, rune_to_json};

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
            &[product::Column::Name],
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
            "product_kind": p.product_kind.as_str(),
            "remarks": p.remarks,
            "hsn_code": p.hsn_code,
        })).collect::<Vec<_>>(),
    }))
}

#[cfg(feature = "cap-llm")]
mod args {
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
    use serde::Deserialize;

    use crate::entities::product::{self, Entity as ProductEntity, ProductKind};
    use crate::preferences::set_product_tax_ids;

    #[derive(Debug, Deserialize)]
    pub(super) struct ProductFields {
        name: String,
        #[serde(default)]
        product_kind: String,
        #[serde(default)]
        remarks: String,
        #[serde(default)]
        sales_price_formula: String,
        #[serde(default)]
        journal_id: i64,
        #[serde(default)]
        journal_lines: Option<serde_json::Value>,
        #[serde(default)]
        hsn_code: i64,
        #[serde(default)]
        tax_ids: Vec<i64>,
        #[serde(default)]
        variables: Option<serde_json::Value>,
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct UpdateProductArgs {
        id: i64,
        #[serde(flatten)]
        fields: ProductFields,
    }

    pub(super) struct ProductInput {
        pub name: String,
        pub product_kind: ProductKind,
        pub remarks: Option<String>,
        pub sales_price_formula: String,
        pub journal_id: String,
        pub journal_lines: String,
        pub hsn_code: Option<i64>,
        pub tax_ids: Vec<i64>,
        pub variables: String,
    }

    fn optional_code(raw: i64) -> Option<i64> {
        if raw <= 0 {
            None
        } else {
            Some(raw)
        }
    }

    fn parse_product_kind(raw: &str) -> ProductKind {
        ProductKind::parse(raw).unwrap_or(ProductKind::Goods)
    }

    fn validate_tax_code(hsn_code: Option<i64>) -> Result<(), String> {
        if hsn_code.is_none() {
            return Err("HSN/SAC code is required".into());
        }
        Ok(())
    }

    fn journal_lines_json(raw: Option<serde_json::Value>) -> String {
        match raw {
            Some(value) if !value.is_null() => value.to_string(),
            _ => crate::journal_template::empty_lines_json(),
        }
    }

    fn opt_string(s: String) -> Option<String> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    fn parse_variables(raw: Option<serde_json::Value>) -> Result<String, String> {
        let Some(raw) = raw.filter(|v| !v.is_null()) else {
            return Ok("{}".into());
        };
        let schema = match &raw {
            serde_json::Value::Array(items) => {
                let entries = items
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .map(str::to_string)
                            .ok_or_else(|| "variables list entries must be strings".to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                lariv_formula::parse_schema_list(&entries).map_err(|e| e.to_string())?
            }
            serde_json::Value::Object(_) => {
                lariv_formula::parse_schema(&raw).map_err(|e| e.to_string())?
            }
            _ => {
                return Err("variables must be an object or a list of name:type strings".into());
            }
        };
        Ok(lariv_formula::schema_to_json(&schema).to_string())
    }

    fn parse_formula(
        schema: &lariv_formula::VariableSchema,
        formula: &str,
        label: &str,
    ) -> Result<String, String> {
        let formula = formula.trim().to_string();
        crate::pricing::validate_price_formula(schema, &formula, label)?;
        Ok(formula)
    }

    fn parse_fields(parsed: ProductFields, op: &str) -> Result<ProductInput, String> {
        let name = parsed.name.trim().to_string();
        if name.is_empty() {
            return Err(format!("{op} requires name"));
        }
        let product_kind = parse_product_kind(&parsed.product_kind);
        let hsn_code = optional_code(parsed.hsn_code);
        validate_tax_code(hsn_code)?;
        let variables = parse_variables(parsed.variables.clone())?;
        let schema = crate::pricing::load_schema(&variables)?;
        crate::journal_template::ensure_schema_allows_bindings(&schema)?;
        let sales_price_formula =
            parse_formula(&schema, &parsed.sales_price_formula, "sales price formula")?;
        let journal_lines = journal_lines_json(parsed.journal_lines);
        if parsed.journal_id > 0 {
            let lines = crate::journal_template::parse_template_lines(&journal_lines)?;
            crate::journal_template::validate_parsed_lines(&schema, &lines)?;
        }
        Ok(ProductInput {
            name,
            product_kind,
            remarks: opt_string(parsed.remarks),
            sales_price_formula,
            journal_id: if parsed.journal_id > 0 {
                parsed.journal_id.to_string()
            } else {
                String::new()
            },
            journal_lines,
            hsn_code,
            tax_ids: parsed.tax_ids,
            variables,
        })
    }

    pub(super) fn parse_create_args(args: &[rune::Value]) -> Result<ProductInput, String> {
        let value = args
            .first()
            .ok_or_else(|| "create_product requires an object argument".to_string())?;
        let parsed: ProductFields = serde_json::from_value(
            lariv_core::rune_env::rune_to_json(value)
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
            lariv_core::rune_env::rune_to_json(value)
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
            product_kind: Set(input.product_kind),
            remarks: Set(input.remarks),
            variables: Set(input.variables.clone()),
            sales_price_formula: Set(input.sales_price_formula.clone()),
            hsn_code: Set(input.hsn_code),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        };
        let m = am.insert(db).await.map_err(|e| e.to_string())?;
        set_product_tax_ids(db, m.id, &input.tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        let schema = crate::pricing::load_schema(&input.variables)?;
        crate::journal_template::save_product_template(
            db,
            m.id,
            &input.journal_id,
            &input.journal_lines,
            &schema,
        )
        .await?;
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
        am.product_kind = Set(input.product_kind);
        am.remarks = Set(input.remarks);
        am.variables = Set(input.variables.clone());
        am.sales_price_formula = Set(input.sales_price_formula);
        am.hsn_code = Set(input.hsn_code);
        am.updated_at = Set(Some(now));
        am.update(db).await.map_err(|e| e.to_string())?;
        set_product_tax_ids(db, id, &input.tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        let schema = crate::pricing::load_schema(&input.variables)?;
        crate::journal_template::save_product_template(
            db,
            id,
            &input.journal_id,
            &input.journal_lines,
            &schema,
        )
        .await?;
        Ok(())
    }
}

#[cfg(feature = "cap-llm")]
use args::{insert_product, parse_create_args, parse_update_args, replace_product};

#[cfg(all(test, feature = "cap-llm"))]
mod tests {
    use super::*;
    use std::sync::Arc;

    use lariv_core::rune_env::{RuneEnvCapability, RuneEnvCtx};
    use lariv_plugin_filesystem::storage::{DynFilestore, UnimplementedFilestore};
    use lariv_plugin_llm_assistant::rune_engine;

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
    async fn create_product_via_rune_rejects_missing_sales_price() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_product(#{ name: "Widget" })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("sales price formula"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_product_via_rune_rejects_invalid_sales_price_formula() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_product(#{ name: "Widget", sales_price_formula: "nope", hsn_code: 1 })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("sales price formula"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_product_via_rune_rejects_invalid_journal_formula() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_product(#{ name: "Widget", sales_price_formula: "decimal(\"2\")", hsn_code: 1, journal_id: 1, journal_lines: [#{ account_id: 1, formula: "nope" }] })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(error.contains("formula"), "unexpected error payload: {out}");
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
create_product(#{
    name: product_name,
    sales_price_formula: "decimal(\"25\")",
    hsn_code: 1234,
    journal_id: 1,
    journal_lines: [#{ account_id: 1, formula: cost }],
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
            error.contains("formula"),
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
        use crate::entities::product::ProductKind;
        use lariv_core::rune_env::json_to_rune;
        use serde_json::json;

        let value = json_to_rune(json!({
            "id": 42,
            "name": "Widget",
            "product_kind": "Goods",
            "remarks": "  Steel widget  ",
            "sales_price_formula": "decimal(\"25.50\")",
            "journal_id": 3,
            "journal_lines": [{ "account_id": 9, "formula": "unit_rate" }, { "account_id": 10, "formula": "-unit_rate" }],
            "hsn_code": 9983,
            "tax_ids": [1, 2]
        }))
        .expect("json to rune");
        let (id, input) = parse_update_args(&[value]).expect("parse update args");
        assert_eq!(id, 42);
        assert_eq!(input.name, "Widget");
        assert_eq!(input.product_kind, ProductKind::Goods);
        assert_eq!(input.remarks.as_deref(), Some("Steel widget"));
        assert_eq!(input.sales_price_formula, "decimal(\"25.50\")");
        assert_eq!(input.journal_id, "3");
        assert!(input.journal_lines.contains("-unit_rate"));
        assert_eq!(input.hsn_code, Some(9983));
        assert_eq!(input.tax_ids, vec![1, 2]);
    }
}
