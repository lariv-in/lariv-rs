//! Rune sandbox bindings for purchase scripts run by the LLM assistant.

use lariv_core::rune_env::{RuneEnvCapability, RuneEnvRegistrar};

/// Registers purchase helpers onto the assistant Rune environment.
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
        "create_purchase",
        "create_purchase(#{ vendor_is_individual?: bool, vendor_contact_id?: int, vendor_company_id?: int, lines: [#{ product_id: int, quantity?: number|string, rate?: number|string, variables?: object, tax_ids?: [int], remarks?: string }], number?: string, reference?: string, payment_reference?: string, bank_account?: string, remarks?: string, datetime?: string, date?: string, delivery_date?: string, timezone?: string, payment_term_lines?: [#{ date_kind: \"absolute\"|\"relative\"|\"relative_delivery\", amount_kind: \"absolute\"|\"relative\", due_date?: string, due_duration?: string, amount?: number|string, amount_percentage?: number|string }], header_tax_ids?: [int] }) -> int  // new draft purchase id. Quantity multiplies the price of one product (the sales formula, or rate). When the product has variables, pass them and that formula is the price of one product. Omitted quantity is 1. Omitted line remarks are filled from the product remarks.",
        |_ctx| NativeBinding::Function(Arc::new(create_purchase)),
    );
    rune_env.register_contextual(
        "update_purchase",
        "update_purchase(#{ id: int, vendor_is_individual?: bool, vendor_contact_id?: int, vendor_company_id?: int, lines: [#{ product_id: int, quantity?: number|string, rate?: number|string, variables?: object, tax_ids?: [int], remarks?: string }], number?: string, reference?: string, payment_reference?: string, bank_account?: string, remarks?: string, datetime?: string, date?: string, delivery_date?: string, timezone?: string, payment_term_lines?: [#{ date_kind: \"absolute\"|\"relative\"|\"relative_delivery\", amount_kind: \"absolute\"|\"relative\", due_date?: string, due_duration?: string, amount?: number|string, amount_percentage?: number|string }], header_tax_ids?: [int] }) -> int  // updated draft purchase id (full replace; purchase must be in draft state). Quantity multiplies the price of one product. Omitted quantity is 1. Omitted line remarks are filled from the product remarks.",
        |_ctx| NativeBinding::Function(Arc::new(update_purchase)),
    );
    rune_env.register_contextual(
        "search_purchases",
        "search_purchases(#{ query: string, limit?: int }) -> #{ drafts: [#{ id: int, number: string|null, reference: string|null, vendor_is_individual: bool, vendor_contact_id: int|null, vendor_company_id: int|null, datetime: string }], posted: [#{ id: int, draft_purchase_id: int, number: string, reference: string|null, vendor_is_individual: bool, vendor_contact_id: int|null, vendor_company_id: int|null, datetime: string }] }",
        |_ctx| NativeBinding::Function(Arc::new(search_purchases)),
    );
}

#[cfg(feature = "cap-llm")]
fn create_purchase(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (input, tz) = parse_create_args(args)?;
    let db = ctx.db.clone();
    let draft = lariv_core::rune_env::block_on_async(async move {
        crate::logic::create_draft_purchase(&db, input, &tz).await
    })?;
    Ok(rune::Value::from(draft.id))
}

#[cfg(feature = "cap-llm")]
fn update_purchase(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (draft_id, input, tz) = parse_update_args(args)?;
    // Fail before any DB round-trip so empty-line scripts get a clear error
    // (and unit tests don't panic on a disconnected default connection).
    if input.lines.is_empty() {
        return Err("add at least one purchase line".to_string());
    }
    let db = ctx.db.clone();
    let draft = lariv_core::rune_env::block_on_async(async move {
        crate::logic::update_draft_purchase(&db, draft_id, input, &tz).await
    })?;
    Ok(rune::Value::from(draft.id))
}

#[cfg(feature = "cap-llm")]
fn search_purchases(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use serde::Deserialize;
    use serde_json::json;

    use crate::entities::draft_purchase::{self, Entity as DraftPurchaseEntity};
    use crate::entities::posted_purchase::{self, Entity as PostedPurchaseEntity};
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
        .ok_or_else(|| "search_purchases requires an object argument".to_string())?;
    let parsed: SearchArgs = serde_json::from_value(rune_to_json(value)?)
        .map_err(|e| format!("invalid search_purchases arguments: {e}"))?;
    let query = parsed.query.trim().to_string();
    if query.is_empty() {
        return Err("search_purchases requires query".into());
    }
    let limit = trigram::clamp_search_limit(parsed.limit);
    let db = ctx.db.clone();
    let (drafts, posted) = block_on_async(async move {
        let drafts = trigram::search::<DraftPurchaseEntity, _>(
            &db,
            &[
                draft_purchase::Column::Number,
                draft_purchase::Column::Reference,
            ],
            &query,
            limit,
        )
        .await
        .map_err(|e| e.to_string())?;
        let posted = trigram::search::<PostedPurchaseEntity, _>(
            &db,
            &[
                posted_purchase::Column::Number,
                posted_purchase::Column::Reference,
            ],
            &query,
            limit,
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok::<_, String>((drafts, posted))
    })?;
    json_to_rune(json!({
        "drafts": drafts.into_iter().map(|d| json!({
            "id": d.id,
            "number": d.number,
            "reference": d.reference,
            "vendor_is_individual": d.vendor_is_individual,
            "vendor_contact_id": d.vendor_contact_id,
            "vendor_company_id": d.vendor_company_id,
            "datetime": d.datetime,
        })).collect::<Vec<_>>(),
        "posted": posted.into_iter().map(|p| json!({
            "id": p.id,
            "draft_purchase_id": p.draft_purchase_id,
            "number": p.number,
            "reference": p.reference,
            "vendor_is_individual": p.vendor_is_individual,
            "vendor_contact_id": p.vendor_contact_id,
            "vendor_company_id": p.vendor_company_id,
            "datetime": p.datetime,
        })).collect::<Vec<_>>(),
    }))
}

#[cfg(feature = "cap-llm")]
mod args {
    use chrono::Utc;
    use serde::Deserialize;

    use crate::logic::draft::DraftLinePending;
    use crate::logic::{
        CreateDraftInput, DraftPaymentTermLineInput, UpdateDraftInput,
        default_payment_term_lines_json, parse_delivery_date, parse_purchase_datetime,
        parse_payment_term_lines_json,
    };
    use crate::{PaymentTermAmountKind, PaymentTermDateKind};

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
    pub(super) struct LineArg {
        product_id: i64,
        #[serde(default)]
        rate: Option<NumberOrString>,
        #[serde(default)]
        quantity: Option<NumberOrString>,
        #[serde(default)]
        variables: Option<serde_json::Value>,
        #[serde(default)]
        tax_ids: Option<Vec<i64>>,
        #[serde(default)]
        remarks: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct PaymentTermArg {
        date_kind: PaymentTermDateKind,
        #[serde(default)]
        due_date: Option<NumberOrString>,
        #[serde(default)]
        due_duration: Option<NumberOrString>,
        amount_kind: PaymentTermAmountKind,
        #[serde(default)]
        amount: Option<NumberOrString>,
        #[serde(default)]
        amount_percentage: Option<NumberOrString>,
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct PurchaseFields {
        #[serde(default)]
        number: Option<String>,
        #[serde(default)]
        reference: Option<String>,
        #[serde(default)]
        payment_reference: Option<String>,
        #[serde(default)]
        bank_account: Option<String>,
        #[serde(default)]
        remarks: Option<String>,
        #[serde(default)]
        datetime: Option<String>,
        #[serde(default)]
        date: Option<String>,
        #[serde(default)]
        delivery_date: Option<String>,
        #[serde(default)]
        timezone: Option<String>,
        #[serde(default)]
        vendor_is_individual: bool,
        #[serde(default)]
        vendor_contact_id: Option<i64>,
        #[serde(default)]
        vendor_company_id: Option<i64>,
        #[serde(default)]
        payment_term_lines: Option<Vec<PaymentTermArg>>,
        #[serde(default)]
        header_tax_ids: Vec<i64>,
        #[serde(default)]
        lines: Vec<LineArg>,
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct UpdatePurchaseArgs {
        id: i64,
        #[serde(flatten)]
        fields: PurchaseFields,
    }

    fn payment_term_lines(
        lines: Option<Vec<PaymentTermArg>>,
    ) -> Result<Vec<DraftPaymentTermLineInput>, String> {
        match lines {
            Some(lines) => Ok(lines
                .into_iter()
                .map(|line| DraftPaymentTermLineInput {
                    date_kind: line.date_kind,
                    due_date: line.due_date.map(NumberOrString::into_string),
                    due_duration: line.due_duration.map(NumberOrString::into_string),
                    amount_kind: line.amount_kind,
                    amount: line.amount.map(NumberOrString::into_string),
                    amount_percentage: line.amount_percentage.map(NumberOrString::into_string),
                })
                .collect()),
            None => parse_payment_term_lines_json(&default_payment_term_lines_json()),
        }
    }

    fn draft_lines(lines: Vec<LineArg>) -> Vec<DraftLinePending> {
        lines
            .into_iter()
            .map(|line| DraftLinePending {
                product_id: line.product_id,
                rate: line.rate.map(NumberOrString::into_string),
                quantity: line
                    .quantity
                    .map(NumberOrString::into_string)
                    .unwrap_or_else(|| "1".into()),
                variables: line.variables,
                tax_ids: line.tax_ids,
                remarks: line.remarks,
            })
            .collect()
    }

    fn parse_fields(parsed: PurchaseFields) -> Result<(CreateDraftInput, String), String> {
        let tz = parsed.timezone.unwrap_or_else(|| "UTC".to_string());
        let datetime = match parsed.datetime.as_deref().or(parsed.date.as_deref()) {
            Some(raw) if !raw.trim().is_empty() => parse_purchase_datetime(raw, &tz),
            _ => Utc::now(),
        };
        let delivery_date = match parsed.delivery_date.as_deref() {
            Some(raw) => parse_delivery_date(raw)?,
            None => None,
        };
        Ok((
            CreateDraftInput {
                number: parsed.number,
                reference: parsed.reference,
                payment_reference: parsed.payment_reference,
                bank_account: parsed.bank_account,
                remarks: parsed.remarks,
                datetime,
                delivery_date,
                bill_to: crate::logic::bill_to::require_bill_to(
                    parsed.vendor_is_individual,
                    parsed.vendor_contact_id.unwrap_or(0),
                    parsed.vendor_company_id.unwrap_or(0),
                )?,
                payment_term_lines: payment_term_lines(parsed.payment_term_lines)?,
                header_tax_ids: parsed.header_tax_ids,
                lines: draft_lines(parsed.lines),
            },
            tz,
        ))
    }

    pub(super) fn parse_create_args(
        args: &[rune::Value],
    ) -> Result<(CreateDraftInput, String), String> {
        let value = args
            .first()
            .ok_or_else(|| "create_purchase requires an object argument".to_string())?;
        let parsed: PurchaseFields = serde_json::from_value(
            lariv_core::rune_env::rune_to_json(value)
                .map_err(|e| format!("invalid create_purchase arguments: {e}"))?,
        )
        .map_err(|e| format!("invalid create_purchase arguments: {e}"))?;
        parse_fields(parsed)
    }

    pub(super) fn parse_update_args(
        args: &[rune::Value],
    ) -> Result<(i64, UpdateDraftInput, String), String> {
        let value = args
            .first()
            .ok_or_else(|| "update_purchase requires an object argument".to_string())?;
        let parsed: UpdatePurchaseArgs = serde_json::from_value(
            lariv_core::rune_env::rune_to_json(value)
                .map_err(|e| format!("invalid update_purchase arguments: {e}"))?,
        )
        .map_err(|e| format!("invalid update_purchase arguments: {e}"))?;
        if parsed.id <= 0 {
            return Err("update_purchase requires a positive draft purchase id".to_string());
        }
        let (create, tz) = parse_fields(parsed.fields)?;
        Ok((
            parsed.id,
            UpdateDraftInput {
                number: create.number,
                reference: create.reference,
                payment_reference: create.payment_reference,
                bank_account: create.bank_account,
                remarks: create.remarks,
                datetime: create.datetime,
                delivery_date: create.delivery_date,
                bill_to: create.bill_to,
                payment_term_lines: create.payment_term_lines,
                header_tax_ids: create.header_tax_ids,
                lines: create.lines,
            },
            tz,
        ))
    }
}

#[cfg(feature = "cap-llm")]
use args::{parse_create_args, parse_update_args};

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
    fn registers_create_and_update_purchase_bindings() {
        let names = registered_env().all_names();
        assert!(
            names.iter().any(|name| name == "create_purchase"),
            "expected create_purchase in {names:?}"
        );
        assert!(
            names.iter().any(|name| name == "update_purchase"),
            "expected update_purchase in {names:?}"
        );
        assert!(
            names.iter().any(|name| name == "search_purchases"),
            "expected search_purchases in {names:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_purchase_via_rune_rejects_missing_vendor() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_purchase(#{ lines: [#{ product_id: 1, quantity: "1" }] })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("select a company")
                || error.contains("invalid create_purchase arguments"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_purchase_via_rune_rejects_empty_lines() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            "create_purchase(#{ vendor_company_id: 1, lines: [] })",
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("add at least one purchase line"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_purchase_via_rune_rejects_missing_argument() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "create_purchase(())", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("create_purchase requires an object argument")
                || error.contains("unsupported"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_purchase_via_rune_accepts_object_built_from_lets() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"
let purchase_number = "P26RIN100294";
let vendor_company_id = 5;
let purchase_date = "27/04/2026";
let product_id = 1;
let quantity = 1;
let rate = 162000;
create_purchase(#{
    number: purchase_number,
    reference: purchase_number,
    vendor_company_id: vendor_company_id,
    date: purchase_date,
    lines: [#{ product_id: product_id, quantity: quantity, rate: rate }]
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
            !error.contains("unsupported create_purchase argument type"),
            "argument conversion failed: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_purchase_via_rune_rejects_missing_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_purchase(#{ vendor_company_id: 1, lines: [#{ product_id: 1, quantity: "1" }] })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("invalid update_purchase arguments") || error.contains("id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_purchase_via_rune_rejects_non_positive_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_purchase(#{ id: 0, vendor_company_id: 1, lines: [#{ product_id: 1, quantity: "1" }] })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("positive draft purchase id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_purchase_via_rune_rejects_empty_lines() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            "update_purchase(#{ id: 1, vendor_company_id: 1, lines: [] })",
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("add at least one purchase line"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn search_purchases_rejects_missing_query() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "search_purchases(#{})", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(error.contains("query"), "unexpected error payload: {out}");
    }

    #[test]
    fn update_purchase_args_accept_object_fields() {
        use lariv_core::rune_env::json_to_rune;
        use serde_json::json;

        let value = json_to_rune(json!({
            "id": 42,
            "vendor_company_id": 5,
            "number": "P26RIN100294",
            "date": "27/04/2026",
            "lines": [{ "product_id": 1, "quantity": 2, "rate": 100 }]
        }))
        .expect("json to rune");
        let (id, input, tz) = parse_update_args(&[value]).expect("parse update args");
        assert_eq!(id, 42);
        assert!(!input.bill_to.vendor_is_individual);
        assert_eq!(input.bill_to.vendor_company_id, Some(5));
        assert_eq!(input.bill_to.vendor_contact_id, None);
        assert_eq!(input.number.as_deref(), Some("P26RIN100294"));
        assert_eq!(input.lines.len(), 1);
        assert_eq!(input.lines[0].product_id, 1);
        assert_eq!(tz, "UTC");
    }
}
