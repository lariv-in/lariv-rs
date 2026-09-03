//! Rune sandbox bindings for customer lookup and mutation.

use crate::rune_env::{RuneEnvCapability, RuneEnvRegistrar};

/// Registers customer helpers onto the assistant Rune environment.
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
        "create_customer",
        "create_customer(#{ name: string, customer_type?: \"business\"|\"individual\", address_line_1?: string, address_line_2?: string, city?: string, pincode?: string, state?: string, gstin?: string, cin?: string, pan?: string, phone?: string, email?: string, website?: string }) -> int  // new customer id",
        |_ctx| NativeBinding::Function(Arc::new(create_customer)),
    );
    rune_env.register_contextual(
        "update_customer",
        "update_customer(#{ id: int, name: string, customer_type?: \"business\"|\"individual\", address_line_1?: string, address_line_2?: string, city?: string, pincode?: string, state?: string, gstin?: string, cin?: string, pan?: string, phone?: string, email?: string, website?: string }) -> int  // updated customer id (full replace)",
        |_ctx| NativeBinding::Function(Arc::new(update_customer)),
    );
    rune_env.register_contextual(
        "search_customers",
        "search_customers(#{ query: string, limit?: int }) -> #{ results: [#{ id: int, name: string, email: string|null, gstin: string|null, city: string|null, phone: string|null }] }",
        |_ctx| NativeBinding::Function(Arc::new(search_customers)),
    );
}

#[cfg(feature = "cap-llm")]
fn create_customer(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let input = parse_create_args(args)?;
    let db = ctx.db.clone();
    let saved = crate::rune_env::block_on_async(async move { insert_customer(&db, input).await })?;
    Ok(rune::Value::from(saved.id))
}

#[cfg(feature = "cap-llm")]
fn update_customer(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (id, input) = parse_update_args(args)?;
    let db = ctx.db.clone();
    crate::rune_env::block_on_async(async move { replace_customer(&db, id, input).await })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn search_customers(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use serde::Deserialize;
    use serde_json::json;

    use crate::db::trigram;
    use crate::plugins::customer::entities::customer::{self, Entity as CustomerEntity};
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
        .ok_or_else(|| "search_customers requires an object argument".to_string())?;
    let parsed: SearchArgs = serde_json::from_value(rune_to_json(value)?)
        .map_err(|e| format!("invalid search_customers arguments: {e}"))?;
    let query = parsed.query.trim().to_string();
    if query.is_empty() {
        return Err("search_customers requires query".into());
    }
    let limit = trigram::clamp_search_limit(parsed.limit);
    let db = ctx.db.clone();
    let rows = block_on_async(async move {
        trigram::search::<CustomerEntity, _>(
            &db,
            &[
                customer::Column::Name,
                customer::Column::Email,
                customer::Column::Gstin,
                customer::Column::City,
            ],
            &query,
            limit,
        )
        .await
    })
    .map_err(|e| e.to_string())?;
    json_to_rune(json!({
        "results": rows.into_iter().map(|c| json!({
            "id": c.id,
            "name": c.name,
            "email": c.email,
            "gstin": c.gstin,
            "city": c.city,
            "phone": c.phone,
        })).collect::<Vec<_>>(),
    }))
}

#[cfg(feature = "cap-llm")]
mod args {
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
    use serde::Deserialize;

    use crate::plugins::customer::customer_type::CustomerType;
    use crate::plugins::customer::entities::customer::{self, Entity as CustomerEntity};

    #[derive(Debug, Deserialize)]
    pub(super) struct CustomerFields {
        #[serde(default)]
        customer_type: String,
        name: String,
        #[serde(default)]
        address_line_1: String,
        #[serde(default)]
        address_line_2: String,
        #[serde(default)]
        city: String,
        #[serde(default)]
        pincode: String,
        #[serde(default)]
        state: String,
        #[serde(default)]
        gstin: String,
        #[serde(default)]
        cin: String,
        #[serde(default)]
        pan: String,
        #[serde(default)]
        phone: String,
        #[serde(default)]
        email: String,
        #[serde(default)]
        website: String,
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct UpdateCustomerArgs {
        id: i64,
        #[serde(flatten)]
        fields: CustomerFields,
    }

    pub(super) struct CustomerInput {
        pub customer_type: CustomerType,
        pub name: String,
        pub address_line_1: Option<String>,
        pub address_line_2: Option<String>,
        pub city: Option<String>,
        pub pincode: Option<String>,
        pub state: Option<String>,
        pub gstin: Option<String>,
        pub cin: Option<String>,
        pub pan: Option<String>,
        pub phone: Option<String>,
        pub email: Option<String>,
        pub website: Option<String>,
    }

    fn opt_string(s: String) -> Option<String> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    fn parse_customer_type(raw: &str) -> Result<CustomerType, String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(CustomerType::default());
        }
        CustomerType::parse(trimmed).ok_or_else(|| format!("invalid customer_type: {raw}"))
    }

    fn parse_fields(parsed: CustomerFields, op: &str) -> Result<CustomerInput, String> {
        let name = parsed.name.trim().to_string();
        if name.is_empty() {
            return Err(format!("{op} requires name"));
        }
        Ok(CustomerInput {
            customer_type: parse_customer_type(&parsed.customer_type)?,
            name,
            address_line_1: opt_string(parsed.address_line_1),
            address_line_2: opt_string(parsed.address_line_2),
            city: opt_string(parsed.city),
            pincode: opt_string(parsed.pincode),
            state: opt_string(parsed.state),
            gstin: opt_string(parsed.gstin),
            cin: opt_string(parsed.cin),
            pan: opt_string(parsed.pan),
            phone: opt_string(parsed.phone),
            email: opt_string(parsed.email),
            website: opt_string(parsed.website),
        })
    }

    pub(super) fn parse_create_args(args: &[rune::Value]) -> Result<CustomerInput, String> {
        let value = args
            .first()
            .ok_or_else(|| "create_customer requires an object argument".to_string())?;
        let parsed: CustomerFields = serde_json::from_value(
            crate::rune_env::rune_to_json(value)
                .map_err(|e| format!("invalid create_customer arguments: {e}"))?,
        )
        .map_err(|e| format!("invalid create_customer arguments: {e}"))?;
        parse_fields(parsed, "create_customer")
    }

    pub(super) fn parse_update_args(args: &[rune::Value]) -> Result<(i64, CustomerInput), String> {
        let value = args
            .first()
            .ok_or_else(|| "update_customer requires an object argument".to_string())?;
        let parsed: UpdateCustomerArgs = serde_json::from_value(
            crate::rune_env::rune_to_json(value)
                .map_err(|e| format!("invalid update_customer arguments: {e}"))?,
        )
        .map_err(|e| format!("invalid update_customer arguments: {e}"))?;
        if parsed.id <= 0 {
            return Err("update_customer requires a positive customer id".to_string());
        }
        Ok((parsed.id, parse_fields(parsed.fields, "update_customer")?))
    }

    pub(super) async fn insert_customer(
        db: &sea_orm::DatabaseConnection,
        input: CustomerInput,
    ) -> Result<customer::Model, String> {
        let now = Utc::now();
        let model = customer::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            customer_type: Set(input.customer_type),
            name: Set(input.name),
            address_line_1: Set(input.address_line_1),
            address_line_2: Set(input.address_line_2),
            city: Set(input.city),
            pincode: Set(input.pincode),
            state: Set(input.state),
            gstin: Set(input.gstin),
            cin: Set(input.cin),
            pan: Set(input.pan),
            phone: Set(input.phone),
            email: Set(input.email),
            website: Set(input.website),
        };
        model.insert(db).await.map_err(|e| e.to_string())
    }

    pub(super) async fn replace_customer(
        db: &sea_orm::DatabaseConnection,
        id: i64,
        input: CustomerInput,
    ) -> Result<(), String> {
        let existing = CustomerEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("customer {id} not found"))?;
        let now = Utc::now();
        let model = customer::ActiveModel {
            id: Set(existing.id),
            updated_at: Set(Some(now)),
            customer_type: Set(input.customer_type),
            name: Set(input.name),
            address_line_1: Set(input.address_line_1),
            address_line_2: Set(input.address_line_2),
            city: Set(input.city),
            pincode: Set(input.pincode),
            state: Set(input.state),
            gstin: Set(input.gstin),
            cin: Set(input.cin),
            pan: Set(input.pan),
            phone: Set(input.phone),
            email: Set(input.email),
            website: Set(input.website),
            ..Default::default()
        };
        model.update(db).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(feature = "cap-llm")]
use args::{insert_customer, parse_create_args, parse_update_args, replace_customer};

#[cfg(all(test, feature = "cap-llm"))]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::plugins::customer::customer_type::CustomerType;
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
            session_id: None,
        }
    }

    fn registered_env() -> RuneEnvCapability {
        let mut cap = RuneEnvCapability::new();
        Hook.register_rune_env(&mut cap);
        cap
    }

    #[test]
    fn registers_create_update_and_search_customer_bindings() {
        let names = registered_env().all_names();
        assert!(
            names.iter().any(|name| name == "create_customer"),
            "expected create_customer in {names:?}"
        );
        assert!(
            names.iter().any(|name| name == "update_customer"),
            "expected update_customer in {names:?}"
        );
        assert!(
            names.iter().any(|name| name == "search_customers"),
            "expected search_customers in {names:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_customer_via_rune_rejects_missing_name() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "create_customer(#{})", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("invalid create_customer arguments") || error.contains("name"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_customer_via_rune_rejects_empty_name() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_customer(#{ name: "  " })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("create_customer requires name"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_customer_via_rune_rejects_missing_argument() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "create_customer(())", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("create_customer requires an object argument")
                || error.contains("unsupported"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_customer_via_rune_rejects_invalid_type() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_customer(#{ name: "Acme", customer_type: "corp" })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("invalid customer_type"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_customer_via_rune_accepts_object_built_from_lets() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"
let customer_name = "Acme Corp";
let city = "Mumbai";
let bad_type = "corp";
create_customer(#{
    name: customer_name,
    customer_type: bad_type,
    city: city
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
            error.contains("invalid customer_type"),
            "object-from-lets conversion failed: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_customer_via_rune_rejects_missing_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_customer(#{ name: "Acme" })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("invalid update_customer arguments") || error.contains("id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_customer_via_rune_rejects_non_positive_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_customer(#{ id: 0, name: "Acme" })"#,
            &[],
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

    #[tokio::test(flavor = "multi_thread")]
    async fn search_customers_rejects_missing_query() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "search_customers(#{})", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(error.contains("query"), "unexpected error payload: {out}");
    }

    #[test]
    fn update_customer_args_accept_object_fields() {
        use crate::rune_env::json_to_rune;
        use serde_json::json;

        let value = json_to_rune(json!({
            "id": 42,
            "name": "Acme Corp",
            "customer_type": "business",
            "city": "Mumbai",
            "email": "billing@acme.test"
        }))
        .expect("json to rune");
        let (id, input) = parse_update_args(&[value]).expect("parse update args");
        assert_eq!(id, 42);
        assert_eq!(input.name, "Acme Corp");
        assert_eq!(input.customer_type, CustomerType::Business);
        assert_eq!(input.city.as_deref(), Some("Mumbai"));
        assert_eq!(input.email.as_deref(), Some("billing@acme.test"));
    }
}
