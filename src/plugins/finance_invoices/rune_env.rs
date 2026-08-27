//! Rune sandbox bindings for invoice scripts run by the LLM assistant.

use crate::rune_env::{RuneEnvCapability, RuneEnvRegistrar};

/// Registers invoice helpers onto the assistant Rune environment.
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
        "create_invoice",
        "create_invoice(#{ customer_id: int, lines: [#{ product_id: int, quantity: number|string, rate?: number|string, tax_ids?: [int] }], number?: string, reference?: string, payment_reference?: string, bank_account?: string, datetime?: string, date?: string, delivery_date?: string, timezone?: string, payment_term_lines?: [#{ date_kind: \"absolute\"|\"relative\"|\"relative_delivery\", amount_kind: \"absolute\"|\"relative\", due_date?: string, due_duration?: string, amount?: number|string, amount_percentage?: number|string }], header_tax_ids?: [int] }) -> int  // new draft invoice id",
        |_ctx| NativeBinding::Function(Arc::new(create_invoice)),
    );
    rune_env.register_contextual(
        "update_invoice",
        "update_invoice(#{ id: int, customer_id: int, lines: [#{ product_id: int, quantity: number|string, rate?: number|string, tax_ids?: [int] }], number?: string, reference?: string, payment_reference?: string, bank_account?: string, datetime?: string, date?: string, delivery_date?: string, timezone?: string, payment_term_lines?: [#{ date_kind: \"absolute\"|\"relative\"|\"relative_delivery\", amount_kind: \"absolute\"|\"relative\", due_date?: string, due_duration?: string, amount?: number|string, amount_percentage?: number|string }], header_tax_ids?: [int] }) -> int  // updated draft invoice id (full replace; draft must not be posted)",
        |_ctx| NativeBinding::Function(Arc::new(update_invoice)),
    );
}

#[cfg(feature = "cap-llm")]
fn create_invoice(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (input, tz) = parse_create_args(args)?;
    let db = ctx.db.clone();
    let draft = crate::rune_env::block_on_async(async move {
        crate::plugins::finance_invoices::logic::create_draft_invoice(&db, input, &tz).await
    })?;
    Ok(rune::Value::from(draft.id))
}

#[cfg(feature = "cap-llm")]
fn update_invoice(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (draft_id, input, tz) = parse_update_args(args)?;
    // Fail before any DB round-trip so empty-line scripts get a clear error
    // (and unit tests don't panic on a disconnected default connection).
    if input.lines.is_empty() {
        return Err("add at least one invoice line".to_string());
    }
    let db = ctx.db.clone();
    let draft = crate::rune_env::block_on_async(async move {
        crate::plugins::finance_invoices::logic::update_draft_invoice(&db, draft_id, input, &tz)
            .await
    })?;
    Ok(rune::Value::from(draft.id))
}

#[cfg(feature = "cap-llm")]
mod args {
    use chrono::Utc;
    use serde::Deserialize;

    use crate::plugins::finance_invoices::logic::draft::DraftLinePending;
    use crate::plugins::finance_invoices::logic::{
        CreateDraftInput, DraftPaymentTermLineInput, UpdateDraftInput,
        default_payment_term_lines_json, parse_delivery_date, parse_invoice_datetime,
        parse_payment_term_lines_json,
    };
    use crate::plugins::finance_invoices::{PaymentTermAmountKind, PaymentTermDateKind};

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
        quantity: NumberOrString,
        #[serde(default)]
        tax_ids: Option<Vec<i64>>,
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
    pub(super) struct InvoiceFields {
        #[serde(default)]
        number: Option<String>,
        #[serde(default)]
        reference: Option<String>,
        #[serde(default)]
        payment_reference: Option<String>,
        #[serde(default)]
        bank_account: Option<String>,
        #[serde(default)]
        datetime: Option<String>,
        #[serde(default)]
        date: Option<String>,
        #[serde(default)]
        delivery_date: Option<String>,
        #[serde(default)]
        timezone: Option<String>,
        customer_id: i64,
        #[serde(default)]
        payment_term_lines: Option<Vec<PaymentTermArg>>,
        #[serde(default)]
        header_tax_ids: Vec<i64>,
        #[serde(default)]
        lines: Vec<LineArg>,
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct UpdateInvoiceArgs {
        id: i64,
        #[serde(flatten)]
        fields: InvoiceFields,
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
                quantity: line.quantity.into_string(),
                tax_ids: line.tax_ids,
            })
            .collect()
    }

    fn parse_fields(parsed: InvoiceFields) -> Result<(CreateDraftInput, String), String> {
        let tz = parsed.timezone.unwrap_or_else(|| "UTC".to_string());
        let datetime = match parsed.datetime.as_deref().or(parsed.date.as_deref()) {
            Some(raw) if !raw.trim().is_empty() => parse_invoice_datetime(raw, &tz),
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
                datetime,
                delivery_date,
                customer_id: parsed.customer_id,
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
            .ok_or_else(|| "create_invoice requires an object argument".to_string())?;
        let parsed: InvoiceFields = serde_json::from_value(
            crate::rune_env::rune_to_json(value)
                .map_err(|e| format!("invalid create_invoice arguments: {e}"))?,
        )
        .map_err(|e| format!("invalid create_invoice arguments: {e}"))?;
        parse_fields(parsed)
    }

    pub(super) fn parse_update_args(
        args: &[rune::Value],
    ) -> Result<(i64, UpdateDraftInput, String), String> {
        let value = args
            .first()
            .ok_or_else(|| "update_invoice requires an object argument".to_string())?;
        let parsed: UpdateInvoiceArgs = serde_json::from_value(
            crate::rune_env::rune_to_json(value)
                .map_err(|e| format!("invalid update_invoice arguments: {e}"))?,
        )
        .map_err(|e| format!("invalid update_invoice arguments: {e}"))?;
        if parsed.id <= 0 {
            return Err("update_invoice requires a positive draft invoice id".to_string());
        }
        let (create, tz) = parse_fields(parsed.fields)?;
        Ok((
            parsed.id,
            UpdateDraftInput {
                number: create.number,
                reference: create.reference,
                payment_reference: create.payment_reference,
                bank_account: create.bank_account,
                datetime: create.datetime,
                delivery_date: create.delivery_date,
                customer_id: create.customer_id,
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
    fn registers_create_and_update_invoice_bindings() {
        let names = registered_env().all_names();
        assert!(
            names.iter().any(|name| name == "create_invoice"),
            "expected create_invoice in {names:?}"
        );
        assert!(
            names.iter().any(|name| name == "update_invoice"),
            "expected update_invoice in {names:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_invoice_via_rune_rejects_missing_customer() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_invoice(#{ lines: [#{ product_id: 1, quantity: "1" }] })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("invalid create_invoice arguments") || error.contains("customer_id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_invoice_via_rune_rejects_empty_lines() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            "create_invoice(#{ customer_id: 1, lines: [] })",
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("add at least one invoice line"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_invoice_via_rune_rejects_missing_argument() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "create_invoice(())", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("create_invoice requires an object argument")
                || error.contains("unsupported"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_invoice_via_rune_accepts_object_built_from_lets() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"
let invoice_number = "P26RIN100294";
let customer_id = 5;
let invoice_date = "27/04/2026";
let product_id = 1;
let quantity = 1;
let rate = 162000;
create_invoice(#{
    number: invoice_number,
    reference: invoice_number,
    customer_id: customer_id,
    date: invoice_date,
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
            !error.contains("unsupported create_invoice argument type"),
            "argument conversion failed: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_invoice_via_rune_rejects_missing_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_invoice(#{ customer_id: 1, lines: [#{ product_id: 1, quantity: "1" }] })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("invalid update_invoice arguments") || error.contains("id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_invoice_via_rune_rejects_non_positive_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_invoice(#{ id: 0, customer_id: 1, lines: [#{ product_id: 1, quantity: "1" }] })"#,
            &[],
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
    async fn update_invoice_via_rune_rejects_empty_lines() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            "update_invoice(#{ id: 1, customer_id: 1, lines: [] })",
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("add at least one invoice line"),
            "unexpected error payload: {out}"
        );
    }

    #[test]
    fn update_invoice_args_accept_object_fields() {
        use crate::rune_env::json_to_rune;
        use serde_json::json;

        let value = json_to_rune(json!({
            "id": 42,
            "customer_id": 5,
            "number": "P26RIN100294",
            "date": "27/04/2026",
            "lines": [{ "product_id": 1, "quantity": 2, "rate": 100 }]
        }))
        .expect("json to rune");
        let (id, input, tz) = parse_update_args(&[value]).expect("parse update args");
        assert_eq!(id, 42);
        assert_eq!(input.customer_id, 5);
        assert_eq!(input.number.as_deref(), Some("P26RIN100294"));
        assert_eq!(input.lines.len(), 1);
        assert_eq!(input.lines[0].product_id, 1);
        assert_eq!(tz, "UTC");
    }
}
