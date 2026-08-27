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
fn parse_create_args(
    args: &[rune::Value],
) -> Result<
    (
        crate::plugins::finance_invoices::logic::CreateDraftInput,
        String,
    ),
    String,
> {
    use chrono::Utc;
    use serde::Deserialize;

    use crate::plugins::finance_invoices::logic::draft::DraftLinePending;
    use crate::plugins::finance_invoices::logic::{
        CreateDraftInput, DraftPaymentTermLineInput, default_payment_term_lines_json,
        parse_delivery_date, parse_invoice_datetime, parse_payment_term_lines_json,
    };
    use crate::plugins::finance_invoices::{PaymentTermAmountKind, PaymentTermDateKind};

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum NumberOrString {
        Number(serde_json::Number),
        String(String),
    }

    impl NumberOrString {
        fn into_string(self) -> String {
            match self {
                Self::Number(n) => n.to_string(),
                Self::String(s) => s,
            }
        }
    }

    #[derive(Debug, Deserialize)]
    struct LineArg {
        product_id: i64,
        #[serde(default)]
        rate: Option<NumberOrString>,
        quantity: NumberOrString,
        #[serde(default)]
        tax_ids: Option<Vec<i64>>,
    }

    #[derive(Debug, Deserialize)]
    struct PaymentTermArg {
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
    struct CreateInvoiceArgs {
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

    let value = args
        .first()
        .ok_or_else(|| "create_invoice requires an object argument".to_string())?;
    let parsed: CreateInvoiceArgs = serde_json::from_value(
        crate::rune_env::rune_to_json(value)
            .map_err(|e| format!("invalid create_invoice arguments: {e}"))?,
    )
    .map_err(|e| format!("invalid create_invoice arguments: {e}"))?;
    let tz = parsed.timezone.unwrap_or_else(|| "UTC".to_string());
    let datetime = match parsed.datetime.as_deref().or(parsed.date.as_deref()) {
        Some(raw) if !raw.trim().is_empty() => parse_invoice_datetime(raw, &tz),
        _ => Utc::now(),
    };
    let delivery_date = match parsed.delivery_date.as_deref() {
        Some(raw) => parse_delivery_date(raw)?,
        None => None,
    };
    let payment_term_lines = match parsed.payment_term_lines {
        Some(lines) => lines
            .into_iter()
            .map(|line| DraftPaymentTermLineInput {
                date_kind: line.date_kind,
                due_date: line.due_date.map(NumberOrString::into_string),
                due_duration: line.due_duration.map(NumberOrString::into_string),
                amount_kind: line.amount_kind,
                amount: line.amount.map(NumberOrString::into_string),
                amount_percentage: line.amount_percentage.map(NumberOrString::into_string),
            })
            .collect(),
        None => parse_payment_term_lines_json(&default_payment_term_lines_json())?,
    };
    let lines = parsed
        .lines
        .into_iter()
        .map(|line| DraftLinePending {
            product_id: line.product_id,
            rate: line.rate.map(NumberOrString::into_string),
            quantity: line.quantity.into_string(),
            tax_ids: line.tax_ids,
        })
        .collect();

    Ok((
        CreateDraftInput {
            number: parsed.number,
            reference: parsed.reference,
            payment_reference: parsed.payment_reference,
            bank_account: parsed.bank_account,
            datetime,
            delivery_date,
            customer_id: parsed.customer_id,
            payment_term_lines,
            header_tax_ids: parsed.header_tax_ids,
            lines,
        },
        tz,
    ))
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
    fn registers_create_invoice_binding() {
        let cap = registered_env();
        assert!(
            cap.all_names().iter().any(|name| name == "create_invoice"),
            "expected create_invoice in {:?}",
            cap.all_names()
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
}
