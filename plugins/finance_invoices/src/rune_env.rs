//! Rune sandbox bindings for invoice scripts run by the LLM assistant.
//!
//! Drafts, posted invoices, and cancelled invoices are separate functions.
//! Posting a draft creates a posted invoice. Cancelling a posted invoice
//! creates a cancelled invoice. Those records are not edited or deleted.
//! Invoice payments are read-only.

use lariv_core::rune_env::{RuneEnvCapability, RuneEnvRegistrar};

/// Registers invoice helpers onto the assistant Rune environment.
#[derive(Clone, Copy, Default)]
pub struct Hook;

impl RuneEnvRegistrar for Hook {
    fn register_rune_env(self, rune_env: &mut RuneEnvCapability) {
        mount(rune_env);
    }
}

/// Register invoice script bindings. Later plugins call this again so their doc suffix is included.
pub fn mount(rune_env: &mut RuneEnvCapability) {
    #[cfg(feature = "cap-llm")]
    register(rune_env);
    #[cfg(not(feature = "cap-llm"))]
    let _ = rune_env;
}

#[cfg(feature = "cap-llm")]
fn documented(base: &str) -> String {
    format!("{base}{}", crate::script_addon::doc_suffix())
}

#[cfg(feature = "cap-llm")]
fn script_args(args: &[rune::Value]) -> serde_json::Value {
    args.first()
        .and_then(|value| lariv_core::rune_env::rune_to_json(value).ok())
        .unwrap_or(serde_json::Value::Null)
}

#[cfg(feature = "cap-llm")]
fn attach_script_fields(
    mut mapped: std::collections::HashMap<String, rune::Value>,
    extras: Vec<(String, serde_json::Value)>,
) -> Result<rune::Value, String> {
    for (key, value) in extras {
        mapped.insert(key, lariv_core::rune_env::json_to_rune(value)?);
    }
    rune::to_value(mapped).map_err(|e| e.to_string())
}

#[cfg(feature = "cap-llm")]
fn register(rune_env: &mut RuneEnvCapability) {
    use std::sync::Arc;

    use lariv_core::rune_env::NativeBinding;

    rune_env.register_contextual(
        "create_draft_invoice",
        documented("create_draft_invoice(#{ customer_id?: int, lines?: [#{ product_id: int, quantity?: number|string, qty_kind?: string, qty_unit?: string, rate?: number|string, variables?: object, tax_ids?: [int], remarks?: string }], number?: string, reference?: string, eway_bill_number?: string, payment_reference?: string, bank_account?: string, remarks?: string, datetime?: string, date?: string, delivery_date?: string, timezone?: string, payment_term_lines?: [#{ date_kind: \"absolute\"|\"relative\"|\"relative_delivery\", amount_kind: \"absolute\"|\"relative\", due_date?: string, due_duration?: string, amount?: number|string, amount_percentage?: number|string }], header_tax_ids?: [int] }) -> int  // new draft invoice id. Provide at least one line (lines). Quantity is a measure: qty_kind is its kind (length, weight, quantity, …) and qty_unit is the unit for that kind. The magnitude multiplies the price of one product. Omitted kind is quantity and omitted unit is that kind's default (Number for a count). When the catalog item has variables, pass them; an omitted rate uses the sales formula, and a given rate is the price of one unit. Omitted quantity is 1. Omitted line remarks are filled from the product remarks."),
        |_ctx| NativeBinding::Function(Arc::new(create_draft_invoice)),
    );
    rune_env.register_contextual(
        "update_draft_invoice",
        documented("update_draft_invoice(#{ id: int, customer_id?: int, lines?: [#{ product_id: int, quantity?: number|string, qty_kind?: string, qty_unit?: string, rate?: number|string, variables?: object, tax_ids?: [int], remarks?: string }], number?: string, reference?: string, eway_bill_number?: string, payment_reference?: string, bank_account?: string, remarks?: string, datetime?: string, date?: string, delivery_date?: string, timezone?: string, payment_term_lines?: [#{ date_kind: \"absolute\"|\"relative\"|\"relative_delivery\", amount_kind: \"absolute\"|\"relative\", due_date?: string, due_duration?: string, amount?: number|string, amount_percentage?: number|string }], header_tax_ids?: [int] }) -> int  // updated draft invoice id (full replace; invoice must be in draft state). Provide at least one line (lines). Quantity is a measure: qty_kind is its kind and qty_unit is the unit for that kind. The magnitude multiplies the price of one product. Omitted kind is quantity. When the catalog item has variables, an omitted rate uses the sales formula, and a given rate is the price of one unit. Omitted quantity is 1. Omitted line remarks are filled from the product remarks."),
        |_ctx| NativeBinding::Function(Arc::new(update_draft_invoice)),
    );
    rune_env.register_contextual(
        "get_draft_invoice",
        documented(concat!(
            "get_draft_invoice(#{ id: int }) -> #{ id: int, number: string|null, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null, pre_tax_amount: string, tax_levied: string, total_amount: string, ",
            "lines: [#{ id: int, product_id: int, product_name: string, quantity: string, qty_kind: string, qty_unit: string, rate: string, variables: object, tax_ids: [int], remarks: string|null, pre_tax_amount: string, tax_levied: string, total_amount: string }], ",
            "header_tax_ids: [int], ",
            "payment_term_lines: [#{ date_kind: \"absolute\"|\"relative\"|\"relative_delivery\", due_date: string|null, due_duration: string|null, amount_kind: \"absolute\"|\"relative\", amount: string|null, amount_percentage: string|null }] }",
        )),
        |_ctx| NativeBinding::Function(Arc::new(get_draft_invoice)),
    );
    rune_env.register_contextual(
        "search_draft_invoices",
        "search_draft_invoices(#{ query: string, limit?: int }) -> #{ results: [#{ id: int, number: string|null, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null, pre_tax_amount: string, tax_levied: string, total_amount: string }] }  // pre_tax_amount is the untaxed subtotal. tax_levied excludes withholding. total_amount is the amount due.",
        |_ctx| NativeBinding::Function(Arc::new(search_draft_invoices)),
    );
    rune_env.register_contextual(
        "delete_draft_invoice",
        "delete_draft_invoice(#{ id: int }) ->  // permanently deletes the draft invoice",
        |_ctx| NativeBinding::Function(Arc::new(delete_draft_invoice)),
    );
    rune_env.register_contextual(
        "create_posted_invoice",
        "create_posted_invoice(#{ draft_id: int, posted_at?: string, timezone?: string }) -> int  // posts the draft and returns the posted invoice id. The draft is removed.",
        |_ctx| NativeBinding::Function(Arc::new(create_posted_invoice)),
    );
    rune_env.register_contextual(
        "get_posted_invoice",
        documented(concat!(
            "get_posted_invoice(#{ id: int }) -> #{ id: int, number: string, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null, posted_at: string|null, settlement: \"open\"|\"partial\"|\"paid\", pre_tax_amount: string, tax_levied: string, total_amount: string, ",
            "lines: [#{ id: int, product_id: int|null, product_name: string, quantity: string, qty_kind: string, qty_unit: string, rate: string, variables: object, tax_ids: [int], remarks: string|null, pre_tax_amount: string, tax_levied: string, total_amount: string }], ",
            "header_tax_ids: [int], ",
            "payment_term_lines: [#{ due_date: string, amount: string }] }  // cancelled invoices are not returned. product_id is null after the product is deleted. payment_term_lines are the resolved due dates and amounts.",
        )),
        |_ctx| NativeBinding::Function(Arc::new(get_posted_invoice)),
    );
    rune_env.register_contextual(
        "search_posted_invoices",
        "search_posted_invoices(#{ query: string, limit?: int }) -> #{ results: [#{ id: int, number: string, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null, posted_at: string|null, settlement: \"open\"|\"partial\"|\"paid\", pre_tax_amount: string, tax_levied: string, total_amount: string }] }  // excludes cancelled invoices. settlement is open, partial, or paid. pre_tax_amount is the untaxed subtotal. tax_levied excludes withholding. total_amount is the amount due.",
        |_ctx| NativeBinding::Function(Arc::new(search_posted_invoices)),
    );
    rune_env.register_contextual(
        "create_cancelled_invoice",
        "create_cancelled_invoice(#{ posted_invoice_id: int, reason: string, cancelled_at?: string, timezone?: string }) -> int  // cancels the posted invoice and returns the cancelled invoice id",
        |_ctx| NativeBinding::Function(Arc::new(create_cancelled_invoice)),
    );
    rune_env.register_contextual(
        "get_cancelled_invoice",
        documented(concat!(
            "get_cancelled_invoice(#{ id: int }) -> #{ id: int, posted_invoice_id: int, number: string, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null, posted_at: string|null, cancelled_at: string|null, pre_tax_amount: string, tax_levied: string, total_amount: string, ",
            "lines: [#{ id: int, product_id: int|null, product_name: string, quantity: string, qty_kind: string, qty_unit: string, rate: string, variables: object, tax_ids: [int], remarks: string|null, pre_tax_amount: string, tax_levied: string, total_amount: string }], ",
            "header_tax_ids: [int], ",
            "payment_term_lines: [#{ due_date: string, amount: string }] }",
        )),
        |_ctx| NativeBinding::Function(Arc::new(get_cancelled_invoice)),
    );
    rune_env.register_contextual(
        "search_cancelled_invoices",
        "search_cancelled_invoices(#{ query: string, limit?: int }) -> #{ results: [#{ id: int, posted_invoice_id: int, number: string, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null, posted_at: string|null, cancelled_at: string|null, pre_tax_amount: string, tax_levied: string, total_amount: string }] }",
        |_ctx| NativeBinding::Function(Arc::new(search_cancelled_invoices)),
    );
    rune_env.register_contextual(
        "get_invoice_payment",
        "get_invoice_payment(#{ id: int }) -> #{ id: int, number: string, amount: string, customer_id: int, datetime: string, posted_invoice_id: int, settlement: \"paid\"|\"partial\" }  // id is the payment id",
        |_ctx| NativeBinding::Function(Arc::new(get_invoice_payment)),
    );
    rune_env.register_contextual(
        "list_invoice_payments",
        "list_invoice_payments(#{ posted_invoice_id: int }) -> #{ results: [#{ id: int, number: string, amount: string, customer_id: int, datetime: string, posted_invoice_id: int, settlement: \"paid\"|\"partial\" }] }",
        |_ctx| NativeBinding::Function(Arc::new(list_invoice_payments)),
    );
    rune_env.register_contextual(
        "search_invoice_payments",
        "search_invoice_payments(#{ query: string, limit?: int }) -> #{ results: [#{ id: int, number: string, amount: string, customer_id: int, datetime: string, posted_invoice_id: int, settlement: \"paid\"|\"partial\" }] }  // matches payment number. Only payments that settle an invoice are returned.",
        |_ctx| NativeBinding::Function(Arc::new(search_invoice_payments)),
    );
    rune_env.register_contextual(
        "create_quotation",
        documented("create_quotation(#{ customer_id?: int, lines?: [#{ product_id: int, quantity?: number|string, qty_kind?: string, qty_unit?: string, rate?: number|string, variables?: object, tax_ids?: [int], remarks?: string }], number?: string, reference?: string, eway_bill_number?: string, payment_reference?: string, bank_account?: string, remarks?: string, datetime?: string, date?: string, delivery_date?: string, timezone?: string, payment_term_lines?: [#{ date_kind: \"absolute\"|\"relative\"|\"relative_delivery\", amount_kind: \"absolute\"|\"relative\", due_date?: string, due_duration?: string, amount?: number|string, amount_percentage?: number|string }], header_tax_ids?: [int] }) -> int  // new quotation id. Provide at least one line (lines). Quantity is a measure: qty_kind is its kind (length, weight, quantity, …) and qty_unit is the unit for that kind. The magnitude multiplies the price of one product. Omitted kind is quantity and omitted unit is that kind's default (Number for a count). When the catalog item has variables, pass them; an omitted rate uses the sales formula, and a given rate is the price of one unit. Omitted quantity is 1. Omitted line remarks are filled from the product remarks."),
        |_ctx| NativeBinding::Function(Arc::new(create_quotation)),
    );
    rune_env.register_contextual(
        "update_quotation",
        documented("update_quotation(#{ id: int, customer_id?: int, lines?: [#{ product_id: int, quantity?: number|string, qty_kind?: string, qty_unit?: string, rate?: number|string, variables?: object, tax_ids?: [int], remarks?: string }], number?: string, reference?: string, eway_bill_number?: string, payment_reference?: string, bank_account?: string, remarks?: string, datetime?: string, date?: string, delivery_date?: string, timezone?: string, payment_term_lines?: [#{ date_kind: \"absolute\"|\"relative\"|\"relative_delivery\", amount_kind: \"absolute\"|\"relative\", due_date?: string, due_duration?: string, amount?: number|string, amount_percentage?: number|string }], header_tax_ids?: [int] }) -> int  // updated quotation id (full replace). Provide at least one line (lines). Quantity is a measure: qty_kind is its kind and qty_unit is the unit for that kind. The magnitude multiplies the price of one product. Omitted kind is quantity. When the catalog item has variables, an omitted rate uses the sales formula, and a given rate is the price of one unit. Omitted quantity is 1. Omitted line remarks are filled from the product remarks."),
        |_ctx| NativeBinding::Function(Arc::new(update_quotation)),
    );
    rune_env.register_contextual(
        "get_quotation",
        documented(concat!(
            "get_quotation(#{ id: int }) -> #{ id: int, number: string|null, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null, pre_tax_amount: string, tax_levied: string, total_amount: string, ",
            "lines: [#{ id: int, product_id: int, product_name: string, quantity: string, qty_kind: string, qty_unit: string, rate: string, variables: object, tax_ids: [int], remarks: string|null, pre_tax_amount: string, tax_levied: string, total_amount: string }], ",
            "header_tax_ids: [int], ",
            "payment_term_lines: [#{ date_kind: \"absolute\"|\"relative\"|\"relative_delivery\", due_date: string|null, due_duration: string|null, amount_kind: \"absolute\"|\"relative\", amount: string|null, amount_percentage: string|null }] }",
        )),
        |_ctx| NativeBinding::Function(Arc::new(get_quotation)),
    );
    rune_env.register_contextual(
        "search_quotations",
        "search_quotations(#{ query: string, limit?: int }) -> #{ results: [#{ id: int, number: string|null, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null, pre_tax_amount: string, tax_levied: string, total_amount: string }] }",
        |_ctx| NativeBinding::Function(Arc::new(search_quotations)),
    );
    rune_env.register_contextual(
        "delete_quotation",
        "delete_quotation(#{ id: int }) ->  // permanently deletes the quotation",
        |_ctx| NativeBinding::Function(Arc::new(delete_quotation)),
    );
}

#[cfg(feature = "cap-llm")]
fn create_draft_invoice(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let raw = script_args(args);
    let (input, tz) = parse_create_args(args, "create_draft_invoice")?;
    if input.customer_id <= 0 {
        return Err("select a customer".to_string());
    }
    let db = ctx.db.clone();
    let draft = lariv_core::rune_env::block_on_async(async move {
        let draft = crate::logic::create_draft_invoice(&db, input, &tz).await?;
        crate::script_addon::after_invoice_script_write(
            &db,
            crate::script_addon::InvoiceScriptRecord::Draft,
            draft.id,
            &raw,
        )
        .await?;
        Ok::<_, String>(draft)
    })?;
    Ok(rune::Value::from(draft.id))
}

#[cfg(feature = "cap-llm")]
fn update_draft_invoice(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let raw = script_args(args);
    let (draft_id, input, tz) = parse_update_args(args, "update_draft_invoice", "draft invoice")?;
    // Fail before any DB round-trip so empty-line scripts get a clear error
    // (and unit tests don't panic on a disconnected default connection).
    if input.lines.is_empty() {
        return Err("add at least one line".to_string());
    }
    if input.customer_id <= 0 {
        return Err("select a customer".to_string());
    }
    let db = ctx.db.clone();
    let draft = lariv_core::rune_env::block_on_async(async move {
        let draft = crate::logic::update_draft_invoice(&db, draft_id, input, &tz).await?;
        crate::script_addon::after_invoice_script_write(
            &db,
            crate::script_addon::InvoiceScriptRecord::Draft,
            draft.id,
            &raw,
        )
        .await?;
        Ok::<_, String>(draft)
    })?;
    Ok(rune::Value::from(draft.id))
}

#[cfg(feature = "cap-llm")]
fn create_quotation(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let raw = script_args(args);
    let (input, tz) = parse_create_args(args, "create_quotation")?;
    if input.customer_id <= 0 {
        return Err("select a customer".to_string());
    }
    let db = ctx.db.clone();
    let quotation = lariv_core::rune_env::block_on_async(async move {
        let quotation = crate::logic::create_quotation_invoice(&db, input, &tz).await?;
        crate::script_addon::after_invoice_script_write(
            &db,
            crate::script_addon::InvoiceScriptRecord::Quotation,
            quotation.id,
            &raw,
        )
        .await?;
        Ok::<_, String>(quotation)
    })?;
    Ok(rune::Value::from(quotation.id))
}

#[cfg(feature = "cap-llm")]
fn update_quotation(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let raw = script_args(args);
    let (quotation_id, input, tz) = parse_update_args(args, "update_quotation", "quotation")?;
    if input.lines.is_empty() {
        return Err("add at least one line".to_string());
    }
    if input.customer_id <= 0 {
        return Err("select a customer".to_string());
    }
    let db = ctx.db.clone();
    let quotation = lariv_core::rune_env::block_on_async(async move {
        let quotation =
            crate::logic::update_quotation_invoice(&db, quotation_id, input, &tz).await?;
        crate::script_addon::after_invoice_script_write(
            &db,
            crate::script_addon::InvoiceScriptRecord::Quotation,
            quotation.id,
            &raw,
        )
        .await?;
        Ok::<_, String>(quotation)
    })?;
    Ok(rune::Value::from(quotation.id))
}

#[cfg(feature = "cap-llm")]
fn delete_quotation(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use serde::Deserialize;

    use lariv_core::rune_env::{block_on_async, rune_to_json};

    #[derive(Debug, Deserialize)]
    struct DeleteArgs {
        id: i64,
    }

    let value = args
        .first()
        .ok_or_else(|| "delete_quotation requires an object argument".to_string())?;
    let parsed: DeleteArgs = serde_json::from_value(rune_to_json(value)?)
        .map_err(|e| format!("invalid delete_quotation arguments: {e}"))?;
    if parsed.id <= 0 {
        return Err("delete_quotation requires a positive quotation id".to_string());
    }
    let db = ctx.db.clone();
    block_on_async(async move { crate::logic::delete_quotation(&db, parsed.id).await })?;
    Ok(rune::Value::from(()))
}

#[cfg(feature = "cap-llm")]
mod rows {
    use std::collections::HashMap;

    use chrono::{DateTime, NaiveDate, Utc};
    use rust_decimal::Decimal;
    use sea_orm::{
        ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait,
        QueryFilter, QueryOrder, Statement,
    };
    use serde::Deserialize;

    use crate::entities::{
        cancelled_invoice, draft_invoice, draft_invoice_line, paid_invoice, partially_paid_invoice,
        posted_invoice, posted_invoice_line, quotation_invoice, quotation_invoice_line,
    };
    use lariv_core::rune_env::rune_to_json;

    pub(super) fn object_arg<'a>(
        args: &'a [rune::Value],
        op: &str,
    ) -> Result<&'a rune::Value, String> {
        args.first()
            .ok_or_else(|| format!("{op} requires an object argument"))
    }

    pub(super) fn parse_json<T: serde::de::DeserializeOwned>(
        value: &rune::Value,
        op: &str,
    ) -> Result<T, String> {
        serde_json::from_value(
            rune_to_json(value).map_err(|e| format!("invalid {op} arguments: {e}"))?,
        )
        .map_err(|e| format!("invalid {op} arguments: {e}"))
    }

    pub(super) fn require_id(id: i64, op: &str, label: &str) -> Result<(), String> {
        if id <= 0 {
            Err(format!("{op} requires a positive {label} id"))
        } else {
            Ok(())
        }
    }

    #[derive(Debug, Deserialize)]
    struct IdArgs {
        id: i64,
    }

    pub(super) fn parse_id(args: &[rune::Value], op: &str, label: &str) -> Result<i64, String> {
        let parsed: IdArgs = parse_json(object_arg(args, op)?, op)?;
        require_id(parsed.id, op, label)?;
        Ok(parsed.id)
    }

    #[derive(Debug, Deserialize, Default)]
    struct SearchArgs {
        #[serde(default)]
        query: String,
        #[serde(default)]
        limit: u64,
    }

    pub(super) fn parse_search(args: &[rune::Value], op: &str) -> Result<(String, u64), String> {
        let parsed: SearchArgs = parse_json(object_arg(args, op)?, op)?;
        let query = parsed.query.trim().to_string();
        if query.is_empty() {
            return Err(format!("{op} requires query"));
        }
        Ok((
            query,
            lariv_core::db::trigram::clamp_search_limit(parsed.limit),
        ))
    }

    fn opt_string(value: Option<String>) -> Result<rune::Value, String> {
        match value {
            Some(value) => rune::to_value(value).map_err(|e| e.to_string()),
            None => Ok(rune::Value::from(())),
        }
    }

    fn opt_datetime(value: Option<DateTime<Utc>>) -> Result<rune::Value, String> {
        opt_string(value.map(|datetime| datetime.to_rfc3339()))
    }

    pub(super) fn results(rows: Vec<rune::Value>) -> Result<rune::Value, String> {
        let mut out: HashMap<String, rune::Value> = HashMap::new();
        out.insert(
            "results".into(),
            rune::to_value(rows).map_err(|e| e.to_string())?,
        );
        rune::to_value(out).map_err(|e| e.to_string())
    }

    struct Header {
        id: i64,
        number: Option<String>,
        reference: Option<String>,
        customer_id: i64,
        datetime: DateTime<Utc>,
        delivery_date: Option<NaiveDate>,
        remarks: Option<String>,
        eway_bill_number: Option<String>,
        payment_reference: Option<String>,
        bank_account: Option<String>,
    }

    fn header_map(header: Header) -> Result<HashMap<String, rune::Value>, String> {
        let mut row = HashMap::new();
        row.insert("id".into(), rune::Value::from(header.id));
        row.insert("number".into(), opt_string(header.number)?);
        row.insert("reference".into(), opt_string(header.reference)?);
        row.insert("customer_id".into(), rune::Value::from(header.customer_id));
        row.insert(
            "datetime".into(),
            rune::to_value(header.datetime.to_rfc3339()).map_err(|e| e.to_string())?,
        );
        row.insert(
            "delivery_date".into(),
            opt_string(header.delivery_date.map(|date| date.to_string()))?,
        );
        row.insert("remarks".into(), opt_string(header.remarks)?);
        row.insert(
            "eway_bill_number".into(),
            opt_string(header.eway_bill_number)?,
        );
        row.insert(
            "payment_reference".into(),
            opt_string(header.payment_reference)?,
        );
        row.insert("bank_account".into(), opt_string(header.bank_account)?);
        Ok(row)
    }

    pub(super) fn draft_map(
        row: &draft_invoice::Model,
    ) -> Result<HashMap<String, rune::Value>, String> {
        header_map(Header {
            id: row.id,
            number: row.number.clone(),
            reference: row.reference.clone(),
            customer_id: row.customer_id,
            datetime: row.datetime,
            delivery_date: row.delivery_date,
            remarks: row.remarks.clone(),
            eway_bill_number: row.eway_bill_number.clone(),
            payment_reference: row.payment_reference.clone(),
            bank_account: row.bank_account.clone(),
        })
    }

    pub(super) struct InvoiceAmounts {
        pub pre_tax_amount: String,
        pub tax_levied: String,
        pub total_amount: String,
    }

    pub(super) fn invoice_amounts(metrics: &crate::logic::InvoiceListMetrics) -> InvoiceAmounts {
        InvoiceAmounts {
            pre_tax_amount: money(metrics.untaxed),
            tax_levied: money(metrics.tax_levied),
            total_amount: money(metrics.total),
        }
    }

    pub(super) fn insert_amounts(
        mapped: &mut HashMap<String, rune::Value>,
        amounts: &InvoiceAmounts,
    ) -> Result<(), String> {
        mapped.insert(
            "pre_tax_amount".into(),
            text_value(&amounts.pre_tax_amount)?,
        );
        mapped.insert("tax_levied".into(), text_value(&amounts.tax_levied)?);
        mapped.insert("total_amount".into(), text_value(&amounts.total_amount)?);
        Ok(())
    }

    pub(super) async fn draft_amounts(db: &DatabaseConnection, id: i64) -> InvoiceAmounts {
        invoice_amounts(&crate::logic::draft_invoice_list_metrics(db, id, "UTC").await)
    }

    pub(super) async fn quotation_amounts(db: &DatabaseConnection, id: i64) -> InvoiceAmounts {
        invoice_amounts(&crate::logic::quotation_invoice_list_metrics(db, id, "UTC").await)
    }

    pub(super) async fn posted_amounts(db: &DatabaseConnection, id: i64) -> InvoiceAmounts {
        invoice_amounts(&crate::logic::posted_invoice_list_metrics(db, id).await)
    }

    pub(super) async fn cancelled_amounts(db: &DatabaseConnection, id: i64) -> InvoiceAmounts {
        invoice_amounts(&crate::logic::cancelled_invoice_list_metrics(db, id).await)
    }

    fn value_with_amounts(
        mut mapped: HashMap<String, rune::Value>,
        amounts: &InvoiceAmounts,
    ) -> Result<rune::Value, String> {
        insert_amounts(&mut mapped, amounts)?;
        rune::to_value(mapped).map_err(|e| e.to_string())
    }

    pub(super) fn draft_value(
        row: &draft_invoice::Model,
        amounts: &InvoiceAmounts,
    ) -> Result<rune::Value, String> {
        value_with_amounts(draft_map(row)?, amounts)
    }

    pub(super) fn quotation_map(
        row: &quotation_invoice::Model,
    ) -> Result<HashMap<String, rune::Value>, String> {
        header_map(Header {
            id: row.id,
            number: row.number.clone(),
            reference: row.reference.clone(),
            customer_id: row.customer_id,
            datetime: row.datetime,
            delivery_date: row.delivery_date,
            remarks: row.remarks.clone(),
            eway_bill_number: row.eway_bill_number.clone(),
            payment_reference: row.payment_reference.clone(),
            bank_account: row.bank_account.clone(),
        })
    }

    pub(super) fn quotation_value(
        row: &quotation_invoice::Model,
        amounts: &InvoiceAmounts,
    ) -> Result<rune::Value, String> {
        value_with_amounts(quotation_map(row)?, amounts)
    }

    pub(super) fn posted_map(
        row: &posted_invoice::Model,
        settlement: &str,
    ) -> Result<HashMap<String, rune::Value>, String> {
        let mut mapped = header_map(Header {
            id: row.id,
            number: Some(row.number.clone()),
            reference: row.reference.clone(),
            customer_id: row.customer_id,
            datetime: row.datetime,
            delivery_date: row.delivery_date,
            remarks: row.remarks.clone(),
            eway_bill_number: row.eway_bill_number.clone(),
            payment_reference: row.payment_reference.clone(),
            bank_account: row.bank_account.clone(),
        })?;
        mapped.insert("posted_at".into(), opt_datetime(row.posted_at)?);
        mapped.insert(
            "settlement".into(),
            rune::to_value(settlement).map_err(|e| e.to_string())?,
        );
        Ok(mapped)
    }

    pub(super) fn posted_value(
        row: &posted_invoice::Model,
        settlement: &str,
        amounts: &InvoiceAmounts,
    ) -> Result<rune::Value, String> {
        value_with_amounts(posted_map(row, settlement)?, amounts)
    }

    pub(super) fn cancelled_map(
        row: &cancelled_invoice::Model,
    ) -> Result<HashMap<String, rune::Value>, String> {
        let mut mapped = header_map(Header {
            id: row.id,
            number: Some(row.number.clone()),
            reference: row.reference.clone(),
            customer_id: row.customer_id,
            datetime: row.datetime,
            delivery_date: row.delivery_date,
            remarks: row.remarks.clone(),
            eway_bill_number: row.eway_bill_number.clone(),
            payment_reference: row.payment_reference.clone(),
            bank_account: row.bank_account.clone(),
        })?;
        mapped.insert(
            "posted_invoice_id".into(),
            rune::Value::from(row.posted_invoice_id),
        );
        mapped.insert("posted_at".into(), opt_datetime(row.posted_at)?);
        mapped.insert("cancelled_at".into(), opt_datetime(row.cancelled_at)?);
        Ok(mapped)
    }

    pub(super) fn cancelled_value(
        row: &cancelled_invoice::Model,
        amounts: &InvoiceAmounts,
    ) -> Result<rune::Value, String> {
        value_with_amounts(cancelled_map(row)?, amounts)
    }

    pub(super) struct PaymentHit {
        pub id: i64,
        pub number: String,
        pub amount: String,
        pub customer_id: i64,
        pub datetime: DateTime<Utc>,
        pub posted_invoice_id: i64,
        pub settlement: &'static str,
    }

    pub(super) fn payment_value(hit: PaymentHit) -> Result<rune::Value, String> {
        let mut row: HashMap<String, rune::Value> = HashMap::new();
        row.insert("id".into(), rune::Value::from(hit.id));
        row.insert(
            "number".into(),
            rune::to_value(hit.number).map_err(|e| e.to_string())?,
        );
        row.insert(
            "amount".into(),
            rune::to_value(hit.amount).map_err(|e| e.to_string())?,
        );
        row.insert("customer_id".into(), rune::Value::from(hit.customer_id));
        row.insert(
            "datetime".into(),
            rune::to_value(hit.datetime.to_rfc3339()).map_err(|e| e.to_string())?,
        );
        row.insert(
            "posted_invoice_id".into(),
            rune::Value::from(hit.posted_invoice_id),
        );
        row.insert(
            "settlement".into(),
            rune::to_value(hit.settlement).map_err(|e| e.to_string())?,
        );
        rune::to_value(row).map_err(|e| e.to_string())
    }

    pub(super) async fn find_one<E>(
        db: &DatabaseConnection,
        id: i64,
        missing: &str,
    ) -> Result<E::Model, String>
    where
        E: EntityTrait,
        <<E as EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType: From<i64>,
    {
        E::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| missing.to_string())
    }

    /// `paid` replaces `partial` when both settlement rows exist for one invoice.
    pub(super) async fn posted_settlements(
        db: &DatabaseConnection,
        posted_ids: &[i64],
    ) -> Result<HashMap<i64, &'static str>, String> {
        let mut labels = HashMap::new();
        if posted_ids.is_empty() {
            return Ok(labels);
        }
        let ids = posted_ids.to_vec();
        let partials = partially_paid_invoice::Entity::find()
            .filter(partially_paid_invoice::Column::PostedInvoiceId.is_in(ids.clone()))
            .all(db)
            .await
            .map_err(|e| e.to_string())?;
        for row in partials {
            labels.insert(row.posted_invoice_id, "partial");
        }
        let paid = paid_invoice::Entity::find()
            .filter(paid_invoice::Column::PostedInvoiceId.is_in(ids))
            .all(db)
            .await
            .map_err(|e| e.to_string())?;
        for row in paid {
            labels.insert(row.posted_invoice_id, "paid");
        }
        Ok(labels)
    }

    /// Payment id → (posted invoice id, `paid` or `partial`).
    pub(super) async fn payment_settlements(
        db: &DatabaseConnection,
        payment_ids: &[i64],
    ) -> Result<HashMap<i64, (i64, &'static str)>, String> {
        let mut labels = HashMap::new();
        if payment_ids.is_empty() {
            return Ok(labels);
        }
        let ids = payment_ids.to_vec();
        let partials = partially_paid_invoice::Entity::find()
            .filter(partially_paid_invoice::Column::PaymentId.is_in(ids.clone()))
            .all(db)
            .await
            .map_err(|e| e.to_string())?;
        for row in partials {
            labels.insert(row.payment_id, (row.posted_invoice_id, "partial"));
        }
        let paid = paid_invoice::Entity::find()
            .filter(paid_invoice::Column::PaymentId.is_in(ids))
            .all(db)
            .await
            .map_err(|e| e.to_string())?;
        for row in paid {
            labels.insert(row.payment_id, (row.posted_invoice_id, "paid"));
        }
        Ok(labels)
    }

    pub(super) fn when(raw: Option<String>, tz: &str) -> DateTime<Utc> {
        match raw.as_deref() {
            Some(raw) if !raw.trim().is_empty() => crate::logic::parse_invoice_datetime(raw, tz),
            _ => Utc::now(),
        }
    }

    struct ScriptLine {
        id: i64,
        product_id: Option<i64>,
        product_name: String,
        quantity: String,
        qty_kind: String,
        qty_unit: String,
        rate: String,
        variables: serde_json::Value,
        tax_ids: Vec<i64>,
        remarks: Option<String>,
        pre_tax: Decimal,
        pre_tax_amount: String,
        tax_levied: String,
        total_amount: String,
        extras: Vec<(String, String)>,
    }

    struct LineParts {
        id: i64,
        product_id: Option<i64>,
        product_name: String,
        quantity: Decimal,
        qty_kind: String,
        qty_unit: String,
        rate: Decimal,
        variable_values: String,
        remarks: Option<String>,
        pre_tax_amount: Decimal,
    }

    impl LineParts {
        fn into_script(self) -> ScriptLine {
            ScriptLine {
                id: self.id,
                product_id: self.product_id,
                product_name: self.product_name,
                quantity: money(self.quantity),
                qty_kind: self.qty_kind,
                qty_unit: self.qty_unit,
                rate: money(self.rate),
                variables: variables_json(&self.variable_values),
                tax_ids: Vec::new(),
                remarks: self.remarks,
                pre_tax: self.pre_tax_amount,
                pre_tax_amount: money(self.pre_tax_amount),
                tax_levied: money(Decimal::ZERO),
                total_amount: money(self.pre_tax_amount),
                extras: Vec::new(),
            }
        }
    }

    enum TermLine {
        Editable {
            date_kind: &'static str,
            due_date: Option<String>,
            due_duration: Option<String>,
            amount_kind: &'static str,
            amount: Option<String>,
            amount_percentage: Option<String>,
        },
        Resolved {
            due_date: String,
            amount: String,
        },
    }

    pub(super) struct Related {
        lines: Vec<ScriptLine>,
        header_tax_ids: Vec<i64>,
        terms: Vec<TermLine>,
    }

    fn money(amount: Decimal) -> String {
        lariv_formula::decimal::normalize(amount).to_string()
    }

    fn variables_json(raw: &str) -> serde_json::Value {
        match serde_json::from_str::<serde_json::Value>(raw) {
            Ok(value) if value.is_object() => value,
            _ => serde_json::json!({}),
        }
    }

    fn opt_id(id: Option<i64>) -> rune::Value {
        match id {
            Some(id) => rune::Value::from(id),
            None => rune::Value::from(()),
        }
    }

    fn text_value(value: &str) -> Result<rune::Value, String> {
        rune::to_value(value).map_err(|e| e.to_string())
    }

    fn line_value(line: &ScriptLine) -> Result<rune::Value, String> {
        let mut row = HashMap::new();
        row.insert("id".into(), rune::Value::from(line.id));
        row.insert("product_id".into(), opt_id(line.product_id));
        row.insert("product_name".into(), text_value(&line.product_name)?);
        row.insert("quantity".into(), text_value(&line.quantity)?);
        row.insert("qty_kind".into(), text_value(&line.qty_kind)?);
        row.insert("qty_unit".into(), text_value(&line.qty_unit)?);
        row.insert("rate".into(), text_value(&line.rate)?);
        row.insert(
            "variables".into(),
            lariv_core::rune_env::json_to_rune(line.variables.clone())?,
        );
        row.insert(
            "tax_ids".into(),
            rune::to_value(line.tax_ids.clone()).map_err(|e| e.to_string())?,
        );
        row.insert("remarks".into(), opt_string(line.remarks.clone())?);
        row.insert("pre_tax_amount".into(), text_value(&line.pre_tax_amount)?);
        row.insert("tax_levied".into(), text_value(&line.tax_levied)?);
        row.insert("total_amount".into(), text_value(&line.total_amount)?);
        for (key, value) in &line.extras {
            row.insert(key.clone(), text_value(value)?);
        }
        rune::to_value(row).map_err(|e| e.to_string())
    }

    fn term_value(line: &TermLine) -> Result<rune::Value, String> {
        let mut row: HashMap<String, rune::Value> = HashMap::new();
        match line {
            TermLine::Editable {
                date_kind,
                due_date,
                due_duration,
                amount_kind,
                amount,
                amount_percentage,
            } => {
                row.insert("date_kind".into(), text_value(date_kind)?);
                row.insert("due_date".into(), opt_string(due_date.clone())?);
                row.insert("due_duration".into(), opt_string(due_duration.clone())?);
                row.insert("amount_kind".into(), text_value(amount_kind)?);
                row.insert("amount".into(), opt_string(amount.clone())?);
                row.insert(
                    "amount_percentage".into(),
                    opt_string(amount_percentage.clone())?,
                );
            }
            TermLine::Resolved { due_date, amount } => {
                row.insert("due_date".into(), text_value(due_date)?);
                row.insert("amount".into(), text_value(amount)?);
            }
        }
        rune::to_value(row).map_err(|e| e.to_string())
    }

    pub(super) fn insert_related(
        mapped: &mut HashMap<String, rune::Value>,
        related: Related,
    ) -> Result<(), String> {
        let lines = related
            .lines
            .iter()
            .map(line_value)
            .collect::<Result<Vec<_>, _>>()?;
        let terms = related
            .terms
            .iter()
            .map(term_value)
            .collect::<Result<Vec<_>, _>>()?;
        mapped.insert(
            "lines".into(),
            rune::to_value(lines).map_err(|e| e.to_string())?,
        );
        mapped.insert(
            "header_tax_ids".into(),
            rune::to_value(related.header_tax_ids).map_err(|e| e.to_string())?,
        );
        mapped.insert(
            "payment_term_lines".into(),
            rune::to_value(terms).map_err(|e| e.to_string())?,
        );
        Ok(())
    }

    async fn taxes_by_line(
        db: &DatabaseConnection,
        table: &str,
        line_column: &str,
        line_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<i64>>, String> {
        let mut out: HashMap<i64, Vec<i64>> = HashMap::new();
        if line_ids.is_empty() {
            return Ok(out);
        }
        let list = line_ids
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let rows = db
            .query_all_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                format!(
                    "SELECT {line_column} AS line_id, tax_id FROM {table} WHERE {line_column} IN ({list}) ORDER BY tax_id"
                ),
            ))
            .await
            .map_err(|e| e.to_string())?;
        for row in rows {
            let line_id = row
                .try_get::<i64>("", "line_id")
                .map_err(|e| e.to_string())?;
            let tax_id = row
                .try_get::<i64>("", "tax_id")
                .map_err(|e| e.to_string())?;
            out.entry(line_id).or_default().push(tax_id);
        }
        Ok(out)
    }

    async fn finish_lines(
        db: &DatabaseConnection,
        table: &str,
        tax_table: &str,
        tax_column: &str,
        mut lines: Vec<ScriptLine>,
    ) -> Result<Vec<ScriptLine>, String> {
        let ids: Vec<i64> = lines.iter().map(|line| line.id).collect();
        let taxes = taxes_by_line(db, tax_table, tax_column, &ids).await?;
        for line in &mut lines {
            if let Some(tax_ids) = taxes.get(&line.id) {
                line.tax_ids.clone_from(tax_ids);
            }
            let tax_rows = lariv_plugin_finance_taxes::scope::load_taxes_by_ids(db, &line.tax_ids)
                .await
                .map_err(|e| e.to_string())?;
            let (_, levied, _, net) =
                crate::logic::tax_calculations::invoice_line_amounts(line.pre_tax, &tax_rows);
            line.tax_levied = money(levied);
            line.total_amount = money(net);
            line.extras = crate::line_editor_addon::line_editor_fields(db, table, line.id).await;
        }
        Ok(lines)
    }

    fn editable_term(
        date_kind: crate::PaymentTermDateKind,
        due_date: Option<NaiveDate>,
        due_duration: Option<i64>,
        amount_kind: crate::PaymentTermAmountKind,
        amount: Option<Decimal>,
        amount_percentage: Option<Decimal>,
    ) -> TermLine {
        TermLine::Editable {
            date_kind: date_kind.as_str(),
            due_date: due_date.map(|date| date.to_string()),
            due_duration: due_duration.map(lariv_core::duration::format_duration),
            amount_kind: amount_kind.as_str(),
            amount: amount.map(money),
            amount_percentage: amount_percentage.map(money),
        }
    }

    pub(super) async fn load_draft_related(
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<Related, String> {
        let rows = draft_invoice_line::Entity::find()
            .filter(draft_invoice_line::Column::DraftInvoiceId.eq(id))
            .order_by_asc(draft_invoice_line::Column::Id)
            .all(db)
            .await
            .map_err(|e| e.to_string())?;
        let lines = rows
            .into_iter()
            .map(|line| {
                LineParts {
                    id: line.id,
                    product_id: Some(line.product_id),
                    product_name: line.product_name,
                    quantity: line.quantity,
                    qty_kind: line.qty_kind,
                    qty_unit: line.qty_unit,
                    rate: line.rate,
                    variable_values: line.variable_values,
                    remarks: line.remarks,
                    pre_tax_amount: line.pre_tax_amount,
                }
                .into_script()
            })
            .collect();
        let lines = finish_lines(
            db,
            "draft_invoice_lines",
            "draft_invoice_line_taxes",
            "draft_invoice_line_id",
            lines,
        )
        .await?;
        let header_tax_ids = crate::logic::tax_assoc::load_draft_invoice_tax_ids(db, id)
            .await
            .map_err(|e| e.to_string())?;
        let terms = crate::logic::draft_payment_term::load_draft_payment_term_lines(db, id)
            .await?
            .into_iter()
            .map(|line| {
                editable_term(
                    line.date_kind,
                    line.due_date,
                    line.due_duration,
                    line.amount_kind,
                    line.amount,
                    line.amount_percentage,
                )
            })
            .collect();
        Ok(Related {
            lines,
            header_tax_ids,
            terms,
        })
    }

    pub(super) async fn load_quotation_related(
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<Related, String> {
        let rows = quotation_invoice_line::Entity::find()
            .filter(quotation_invoice_line::Column::QuotationInvoiceId.eq(id))
            .order_by_asc(quotation_invoice_line::Column::Id)
            .all(db)
            .await
            .map_err(|e| e.to_string())?;
        let lines = rows
            .into_iter()
            .map(|line| {
                LineParts {
                    id: line.id,
                    product_id: Some(line.product_id),
                    product_name: line.product_name,
                    quantity: line.quantity,
                    qty_kind: line.qty_kind,
                    qty_unit: line.qty_unit,
                    rate: line.rate,
                    variable_values: line.variable_values,
                    remarks: line.remarks,
                    pre_tax_amount: line.pre_tax_amount,
                }
                .into_script()
            })
            .collect();
        let lines = finish_lines(
            db,
            "quotation_invoice_lines",
            "quotation_invoice_line_taxes",
            "quotation_invoice_line_id",
            lines,
        )
        .await?;
        let header_tax_ids = crate::logic::tax_assoc::load_quotation_invoice_tax_ids(db, id)
            .await
            .map_err(|e| e.to_string())?;
        let terms = crate::logic::quotation::load_quotation_payment_term_lines(db, id)
            .await?
            .into_iter()
            .map(|line| {
                editable_term(
                    line.date_kind,
                    line.due_date,
                    line.due_duration,
                    line.amount_kind,
                    line.amount,
                    line.amount_percentage,
                )
            })
            .collect();
        Ok(Related {
            lines,
            header_tax_ids,
            terms,
        })
    }

    fn resolved_terms(
        lines: Vec<crate::entities::posted_payment_term_line::Model>,
    ) -> Vec<TermLine> {
        lines
            .into_iter()
            .map(|line| TermLine::Resolved {
                due_date: line.due_date.to_string(),
                amount: money(line.amount),
            })
            .collect()
    }

    pub(super) async fn load_posted_related(
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<Related, String> {
        let rows = posted_invoice_line::Entity::find()
            .filter(posted_invoice_line::Column::PostedInvoiceId.eq(id))
            .order_by_asc(posted_invoice_line::Column::Id)
            .all(db)
            .await
            .map_err(|e| e.to_string())?;
        let lines = rows
            .into_iter()
            .map(|line| {
                LineParts {
                    id: line.id,
                    product_id: line.product_id,
                    product_name: line.product_name,
                    quantity: line.quantity,
                    qty_kind: line.qty_kind,
                    qty_unit: line.qty_unit,
                    rate: line.rate,
                    variable_values: line.variable_values,
                    remarks: line.remarks,
                    pre_tax_amount: line.pre_tax_amount,
                }
                .into_script()
            })
            .collect();
        let lines = finish_lines(
            db,
            "posted_invoice_lines",
            "posted_invoice_line_taxes",
            "posted_invoice_line_id",
            lines,
        )
        .await?;
        let header_tax_ids = crate::logic::tax_assoc::load_posted_invoice_tax_ids(db, id)
            .await
            .map_err(|e| e.to_string())?;
        let terms =
            match crate::logic::draft_payment_term::load_posted_payment_term_for_posted(db, id)
                .await?
            {
                Some((_, lines)) => resolved_terms(lines),
                None => Vec::new(),
            };
        Ok(Related {
            lines,
            header_tax_ids,
            terms,
        })
    }

    pub(super) async fn load_cancelled_related(
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<Related, String> {
        let rows = db
            .query_all_raw(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT id, product_id, product_name, rate, quantity, qty_kind, qty_unit, \
                 variable_values, pre_tax_amount, remarks \
                 FROM cancelled_invoice_lines \
                 WHERE cancelled_invoice_id = $1 ORDER BY id ASC",
                [id.into()],
            ))
            .await
            .map_err(|e| e.to_string())?;
        let mut lines = Vec::new();
        for row in rows {
            lines.push(
                LineParts {
                    id: row.try_get("", "id").map_err(|e| e.to_string())?,
                    product_id: row.try_get("", "product_id").map_err(|e| e.to_string())?,
                    product_name: row.try_get("", "product_name").map_err(|e| e.to_string())?,
                    quantity: row.try_get("", "quantity").map_err(|e| e.to_string())?,
                    qty_kind: row.try_get("", "qty_kind").map_err(|e| e.to_string())?,
                    qty_unit: row.try_get("", "qty_unit").map_err(|e| e.to_string())?,
                    rate: row.try_get("", "rate").map_err(|e| e.to_string())?,
                    variable_values: row
                        .try_get("", "variable_values")
                        .map_err(|e| e.to_string())?,
                    remarks: row.try_get("", "remarks").map_err(|e| e.to_string())?,
                    pre_tax_amount: row
                        .try_get("", "pre_tax_amount")
                        .map_err(|e| e.to_string())?,
                }
                .into_script(),
            );
        }
        let lines = finish_lines(
            db,
            "cancelled_invoice_lines",
            "cancelled_invoice_line_taxes",
            "cancelled_invoice_line_id",
            lines,
        )
        .await?;
        let header_tax_ids = crate::logic::tax_assoc::load_cancelled_invoice_tax_ids(db, id)
            .await
            .map_err(|e| e.to_string())?;
        let terms =
            match crate::logic::draft_payment_term::load_posted_payment_term_for_cancelled(db, id)
                .await?
            {
                Some((_, lines)) => resolved_terms(lines),
                None => Vec::new(),
            };
        Ok(Related {
            lines,
            header_tax_ids,
            terms,
        })
    }

    #[cfg(test)]
    mod tests {
        use super::{Related, ScriptLine, TermLine, insert_related, invoice_amounts, line_value};

        fn field<'a>(
            json: &'a serde_json::Value,
            key: &str,
        ) -> Result<&'a serde_json::Value, String> {
            json.get(key).ok_or_else(|| format!("missing {key}"))
        }

        #[test]
        fn line_value_keeps_taxes_variables_and_addon_fields() {
            let line = ScriptLine {
                id: 3,
                product_id: None,
                product_name: "Bolt".into(),
                quantity: "2".into(),
                qty_kind: "quantity".into(),
                qty_unit: "Number".into(),
                rate: "15".into(),
                variables: serde_json::json!({"width": "2"}),
                tax_ids: vec![4, 5],
                remarks: None,
                pre_tax: rust_decimal::Decimal::ZERO,
                pre_tax_amount: "30".into(),
                tax_levied: "5".into(),
                total_amount: "35".into(),
                extras: vec![("gst_supply".into(), "exempt".into())],
            };
            let value = line_value(&line).expect("line value");
            let json = lariv_core::rune_env::rune_to_json(&value).expect("json");
            assert_eq!(field(&json, "id").expect("id"), &serde_json::json!(3));
            assert_eq!(
                field(&json, "product_id").expect("product"),
                &serde_json::Value::Null
            );
            assert_eq!(
                field(&json, "product_name").expect("name"),
                &serde_json::json!("Bolt")
            );
            assert_eq!(
                field(&json, "quantity").expect("qty"),
                &serde_json::json!("2")
            );
            assert_eq!(
                field(&json, "tax_ids").expect("taxes"),
                &serde_json::json!([4, 5])
            );
            assert_eq!(
                field(&json, "variables").expect("variables").get("width"),
                Some(&serde_json::json!("2"))
            );
            assert_eq!(
                field(&json, "remarks").expect("remarks"),
                &serde_json::Value::Null
            );
            assert_eq!(
                field(&json, "pre_tax_amount").expect("pre tax"),
                &serde_json::json!("30")
            );
            assert_eq!(
                field(&json, "tax_levied").expect("levied"),
                &serde_json::json!("5")
            );
            assert_eq!(
                field(&json, "total_amount").expect("total"),
                &serde_json::json!("35")
            );
            assert_eq!(
                field(&json, "gst_supply").expect("gst"),
                &serde_json::json!("exempt")
            );
        }

        #[test]
        fn invoice_amounts_report_pre_tax_levied_and_total() {
            let amounts = invoice_amounts(&crate::logic::InvoiceListMetrics {
                untaxed: rust_decimal::Decimal::from(100),
                tax_levied: rust_decimal::Decimal::from(18),
                total: rust_decimal::Decimal::from(118),
                ..Default::default()
            });
            assert_eq!(amounts.pre_tax_amount, "100");
            assert_eq!(amounts.tax_levied, "18");
            assert_eq!(amounts.total_amount, "118");
        }

        #[test]
        fn insert_related_adds_lines_taxes_and_payment_terms() {
            let mut mapped = std::collections::HashMap::new();
            insert_related(
                &mut mapped,
                Related {
                    lines: vec![ScriptLine {
                        id: 1,
                        product_id: Some(9),
                        product_name: "Widget".into(),
                        quantity: "1".into(),
                        qty_kind: "quantity".into(),
                        qty_unit: "Number".into(),
                        rate: "10".into(),
                        variables: serde_json::json!({}),
                        tax_ids: vec![2],
                        remarks: Some("note".into()),
                        pre_tax: rust_decimal::Decimal::ZERO,
                        pre_tax_amount: "10".into(),
                        tax_levied: "0".into(),
                        total_amount: "10".into(),
                        extras: Vec::new(),
                    }],
                    header_tax_ids: vec![8],
                    terms: vec![TermLine::Editable {
                        date_kind: "relative",
                        due_date: None,
                        due_duration: Some("15 days".into()),
                        amount_kind: "relative",
                        amount: None,
                        amount_percentage: Some("100".into()),
                    }],
                },
            )
            .expect("insert related");
            let value = rune::to_value(mapped).expect("rune map");
            let json = lariv_core::rune_env::rune_to_json(&value).expect("json");
            let lines = field(&json, "lines").expect("lines");
            let first = lines
                .as_array()
                .and_then(|rows| rows.first())
                .expect("first line");
            assert_eq!(
                field(first, "product_id").expect("product"),
                &serde_json::json!(9)
            );
            assert_eq!(
                field(first, "tax_ids").expect("line taxes"),
                &serde_json::json!([2])
            );
            assert_eq!(
                field(&json, "header_tax_ids").expect("header"),
                &serde_json::json!([8])
            );
            let terms = field(&json, "payment_term_lines").expect("terms");
            let term = terms
                .as_array()
                .and_then(|rows| rows.first())
                .expect("first term");
            assert_eq!(
                field(term, "date_kind").expect("kind"),
                &serde_json::json!("relative")
            );
            assert_eq!(
                field(term, "due_duration").expect("duration"),
                &serde_json::json!("15 days")
            );
            assert_eq!(
                field(term, "amount").expect("amount"),
                &serde_json::Value::Null
            );
        }
    }
}

#[cfg(feature = "cap-llm")]
fn get_draft_invoice(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = rows::parse_id(args, "get_draft_invoice", "draft invoice")?;
    let db = ctx.db.clone();
    let (row, related, amounts, extras) = lariv_core::rune_env::block_on_async(async move {
        let row = rows::find_one::<crate::entities::draft_invoice::Entity>(
            &db,
            id,
            "draft invoice not found",
        )
        .await?;
        let related = rows::load_draft_related(&db, row.id).await?;
        let amounts = rows::draft_amounts(&db, row.id).await;
        let extras = crate::script_addon::enrich_invoice_script(
            &db,
            crate::script_addon::InvoiceScriptRecord::Draft,
            row.id,
        )
        .await?;
        Ok::<_, String>((row, related, amounts, extras))
    })?;
    let mut mapped = rows::draft_map(&row)?;
    rows::insert_amounts(&mut mapped, &amounts)?;
    rows::insert_related(&mut mapped, related)?;
    attach_script_fields(mapped, extras)
}

#[cfg(feature = "cap-llm")]
fn search_draft_invoices(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use crate::entities::draft_invoice::{self, Entity as DraftInvoiceEntity};

    let (query, limit) = rows::parse_search(args, "search_draft_invoices")?;
    let db = ctx.db.clone();
    let found = lariv_core::rune_env::block_on_async(async move {
        let matches = lariv_core::db::trigram::search::<DraftInvoiceEntity, _>(
            &db,
            &[
                draft_invoice::Column::Number,
                draft_invoice::Column::Reference,
            ],
            &query,
            limit,
        )
        .await
        .map_err(|e| e.to_string())?;
        let mut amounts = Vec::with_capacity(matches.len());
        for row in &matches {
            amounts.push(rows::draft_amounts(&db, row.id).await);
        }
        Ok::<_, String>((matches, amounts))
    })?;
    let results = found
        .0
        .iter()
        .zip(&found.1)
        .map(|(row, amounts)| rows::draft_value(row, amounts))
        .collect::<Result<Vec<_>, _>>()?;
    rows::results(results)
}

#[cfg(feature = "cap-llm")]
fn delete_draft_invoice(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = rows::parse_id(args, "delete_draft_invoice", "draft invoice")?;
    let db = ctx.db.clone();
    lariv_core::rune_env::block_on_async(async move { crate::logic::delete_draft(&db, id).await })?;
    Ok(rune::Value::from(()))
}

#[cfg(feature = "cap-llm")]
fn create_posted_invoice(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct PostArgs {
        draft_id: i64,
        #[serde(default)]
        posted_at: Option<String>,
        #[serde(default)]
        timezone: Option<String>,
    }

    let parsed: PostArgs = rows::parse_json(
        rows::object_arg(args, "create_posted_invoice")?,
        "create_posted_invoice",
    )?;
    rows::require_id(parsed.draft_id, "create_posted_invoice", "draft")?;
    let tz = parsed.timezone.unwrap_or_else(|| "UTC".to_string());
    let posted_at = rows::when(parsed.posted_at, &tz);
    let db = ctx.db.clone();
    let posted = lariv_core::rune_env::block_on_async(async move {
        crate::logic::draft_new_posted(&db, parsed.draft_id, posted_at, &tz).await
    })?;
    Ok(rune::Value::from(posted.id))
}

#[cfg(feature = "cap-llm")]
fn get_posted_invoice(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use sea_orm::{EntityTrait, QueryFilter};

    use crate::entities::posted_invoice::Entity as PostedInvoiceEntity;
    use crate::scope::sql_posted_not_cancelled;

    let id = rows::parse_id(args, "get_posted_invoice", "posted invoice")?;
    let db = ctx.db.clone();
    let (row, settlement, related, amounts, extras) =
        lariv_core::rune_env::block_on_async(async move {
            let row = PostedInvoiceEntity::find_by_id(id)
                .filter(sql_posted_not_cancelled())
                .one(&db)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "posted invoice not found".to_string())?;
            let labels = rows::posted_settlements(&db, &[row.id]).await?;
            let settlement = labels.get(&row.id).copied().unwrap_or("open");
            let related = rows::load_posted_related(&db, row.id).await?;
            let amounts = rows::posted_amounts(&db, row.id).await;
            let extras = crate::script_addon::enrich_invoice_script(
                &db,
                crate::script_addon::InvoiceScriptRecord::Posted,
                row.id,
            )
            .await?;
            Ok::<_, String>((row, settlement, related, amounts, extras))
        })?;
    let mut mapped = rows::posted_map(&row, settlement)?;
    rows::insert_amounts(&mut mapped, &amounts)?;
    rows::insert_related(&mut mapped, related)?;
    attach_script_fields(mapped, extras)
}

#[cfg(feature = "cap-llm")]
fn search_posted_invoices(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use sea_orm::{EntityTrait, QueryFilter, QuerySelect};

    use crate::entities::posted_invoice::{self, Entity as PostedInvoiceEntity};
    use crate::scope::sql_posted_not_cancelled;

    let (query, limit) = rows::parse_search(args, "search_posted_invoices")?;
    let db = ctx.db.clone();
    let found = lariv_core::rune_env::block_on_async(async move {
        let rows = lariv_core::db::trigram::apply_text_search(
            PostedInvoiceEntity::find().filter(sql_posted_not_cancelled()),
            db.get_database_backend(),
            &[
                posted_invoice::Column::Number,
                posted_invoice::Column::Reference,
            ],
            &query,
        )
        .limit(limit)
        .all(&db)
        .await
        .map_err(|e| e.to_string())?;
        let ids: Vec<i64> = rows.iter().map(|row| row.id).collect();
        let labels = rows::posted_settlements(&db, &ids).await?;
        let mut amounts = Vec::with_capacity(rows.len());
        for row in &rows {
            amounts.push(rows::posted_amounts(&db, row.id).await);
        }
        Ok::<_, String>((rows, labels, amounts))
    })?;
    let results = found
        .0
        .iter()
        .zip(&found.2)
        .map(|(row, amounts)| {
            let settlement = found.1.get(&row.id).copied().unwrap_or("open");
            rows::posted_value(row, settlement, amounts)
        })
        .collect::<Result<Vec<_>, _>>()?;
    rows::results(results)
}

#[cfg(feature = "cap-llm")]
fn create_cancelled_invoice(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct CancelArgs {
        posted_invoice_id: i64,
        reason: String,
        #[serde(default)]
        cancelled_at: Option<String>,
        #[serde(default)]
        timezone: Option<String>,
    }

    let parsed: CancelArgs = rows::parse_json(
        rows::object_arg(args, "create_cancelled_invoice")?,
        "create_cancelled_invoice",
    )?;
    rows::require_id(
        parsed.posted_invoice_id,
        "create_cancelled_invoice",
        "posted invoice",
    )?;
    let reason = parsed.reason.trim().to_string();
    if reason.is_empty() {
        return Err("create_cancelled_invoice requires a reason".to_string());
    }
    let tz = parsed.timezone.unwrap_or_else(|| "UTC".to_string());
    let cancelled_at = rows::when(parsed.cancelled_at, &tz);
    let db = ctx.db.clone();
    let cancelled = lariv_core::rune_env::block_on_async(async move {
        crate::logic::posted_new_cancelled(&db, parsed.posted_invoice_id, reason, cancelled_at)
            .await
    })?;
    Ok(rune::Value::from(cancelled.id))
}

#[cfg(feature = "cap-llm")]
fn get_cancelled_invoice(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = rows::parse_id(args, "get_cancelled_invoice", "cancelled invoice")?;
    let db = ctx.db.clone();
    let (row, related, amounts, extras) = lariv_core::rune_env::block_on_async(async move {
        let row = rows::find_one::<crate::entities::cancelled_invoice::Entity>(
            &db,
            id,
            "cancelled invoice not found",
        )
        .await?;
        let related = rows::load_cancelled_related(&db, row.id).await?;
        let amounts = rows::cancelled_amounts(&db, row.id).await;
        let extras = crate::script_addon::enrich_invoice_script(
            &db,
            crate::script_addon::InvoiceScriptRecord::Cancelled,
            row.id,
        )
        .await?;
        Ok::<_, String>((row, related, amounts, extras))
    })?;
    let mut mapped = rows::cancelled_map(&row)?;
    rows::insert_amounts(&mut mapped, &amounts)?;
    rows::insert_related(&mut mapped, related)?;
    attach_script_fields(mapped, extras)
}

#[cfg(feature = "cap-llm")]
fn search_cancelled_invoices(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use crate::entities::cancelled_invoice::{self, Entity as CancelledInvoiceEntity};

    let (query, limit) = rows::parse_search(args, "search_cancelled_invoices")?;
    let db = ctx.db.clone();
    let found = lariv_core::rune_env::block_on_async(async move {
        let matches = lariv_core::db::trigram::search::<CancelledInvoiceEntity, _>(
            &db,
            &[
                cancelled_invoice::Column::Number,
                cancelled_invoice::Column::Reference,
            ],
            &query,
            limit,
        )
        .await
        .map_err(|e| e.to_string())?;
        let mut amounts = Vec::with_capacity(matches.len());
        for row in &matches {
            amounts.push(rows::cancelled_amounts(&db, row.id).await);
        }
        Ok::<_, String>((matches, amounts))
    })?;
    let results = found
        .0
        .iter()
        .zip(&found.1)
        .map(|(row, amounts)| rows::cancelled_value(row, amounts))
        .collect::<Result<Vec<_>, _>>()?;
    rows::results(results)
}

#[cfg(feature = "cap-llm")]
fn payment_hit(
    payment: &lariv_plugin_finance_payments::entities::payment::Model,
    posted_invoice_id: i64,
    settlement: &'static str,
) -> rows::PaymentHit {
    rows::PaymentHit {
        id: payment.id,
        number: payment.number.clone(),
        amount: lariv_formula::decimal::normalize(payment.amount).to_string(),
        customer_id: payment.customer_id,
        datetime: payment.datetime,
        posted_invoice_id,
        settlement,
    }
}

#[cfg(feature = "cap-llm")]
fn get_invoice_payment(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = rows::parse_id(args, "get_invoice_payment", "invoice payment")?;
    let db = ctx.db.clone();
    let hit = lariv_core::rune_env::block_on_async(async move {
        let payment = rows::find_one::<lariv_plugin_finance_payments::entities::payment::Entity>(
            &db,
            id,
            "invoice payment not found",
        )
        .await?;
        let labels = rows::payment_settlements(&db, &[payment.id]).await?;
        let Some((posted_invoice_id, settlement)) = labels.get(&payment.id).copied() else {
            return Err("invoice payment not found".to_string());
        };
        Ok(payment_hit(&payment, posted_invoice_id, settlement))
    })?;
    rows::payment_value(hit)
}

#[cfg(feature = "cap-llm")]
fn list_invoice_payments(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
    use serde::Deserialize;

    use crate::entities::{paid_invoice, partially_paid_invoice, posted_invoice};
    use lariv_plugin_finance_payments::entities::payment::{self, Entity as PaymentEntity};

    #[derive(Debug, Deserialize)]
    struct ListArgs {
        posted_invoice_id: i64,
    }

    let parsed: ListArgs = rows::parse_json(
        rows::object_arg(args, "list_invoice_payments")?,
        "list_invoice_payments",
    )?;
    rows::require_id(
        parsed.posted_invoice_id,
        "list_invoice_payments",
        "posted invoice",
    )?;
    let db = ctx.db.clone();
    let hits = lariv_core::rune_env::block_on_async(async move {
        rows::find_one::<posted_invoice::Entity>(
            &db,
            parsed.posted_invoice_id,
            "posted invoice not found",
        )
        .await?;
        let mut kinds: std::collections::HashMap<i64, &'static str> =
            std::collections::HashMap::new();
        let partials = partially_paid_invoice::Entity::find()
            .filter(partially_paid_invoice::Column::PostedInvoiceId.eq(parsed.posted_invoice_id))
            .all(&db)
            .await
            .map_err(|e| e.to_string())?;
        for row in partials {
            kinds.insert(row.payment_id, "partial");
        }
        let paid = paid_invoice::Entity::find()
            .filter(paid_invoice::Column::PostedInvoiceId.eq(parsed.posted_invoice_id))
            .all(&db)
            .await
            .map_err(|e| e.to_string())?;
        for row in paid {
            kinds.insert(row.payment_id, "paid");
        }
        if kinds.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<i64> = kinds.keys().copied().collect();
        let payments = PaymentEntity::find()
            .filter(payment::Column::Id.is_in(ids))
            .order_by_asc(payment::Column::Datetime)
            .all(&db)
            .await
            .map_err(|e| e.to_string())?;
        let mut hits = Vec::new();
        for payment in payments {
            let Some(settlement) = kinds.get(&payment.id).copied() else {
                continue;
            };
            hits.push(payment_hit(&payment, parsed.posted_invoice_id, settlement));
        }
        Ok::<_, String>(hits)
    })?;
    let results = hits
        .into_iter()
        .map(rows::payment_value)
        .collect::<Result<Vec<_>, _>>()?;
    rows::results(results)
}

#[cfg(feature = "cap-llm")]
fn search_invoice_payments(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use sea_orm::{EntityTrait, QueryFilter, QuerySelect};

    use lariv_plugin_finance_payments::entities::payment::{self, Entity as PaymentEntity};

    let (query, limit) = rows::parse_search(args, "search_invoice_payments")?;
    let db = ctx.db.clone();
    let found = lariv_core::rune_env::block_on_async(async move {
        let payments = lariv_core::db::trigram::apply_text_search(
            PaymentEntity::find().filter(sea_orm::sea_query::Expr::cust(
                "payments.id IN (SELECT payment_id FROM paid_invoices UNION SELECT payment_id FROM partially_paid_invoices)",
            )),
            db.get_database_backend(),
            &[payment::Column::Number],
            &query,
        )
        .limit(limit)
        .all(&db)
        .await
        .map_err(|e| e.to_string())?;
        let ids: Vec<i64> = payments.iter().map(|payment| payment.id).collect();
        let labels = rows::payment_settlements(&db, &ids).await?;
        Ok::<_, String>((payments, labels))
    })?;
    let mut results = Vec::new();
    for payment in &found.0 {
        let Some((posted_invoice_id, settlement)) = found.1.get(&payment.id).copied() else {
            continue;
        };
        results.push(rows::payment_value(payment_hit(
            payment,
            posted_invoice_id,
            settlement,
        ))?);
    }
    rows::results(results)
}

#[cfg(feature = "cap-llm")]
fn get_quotation(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = rows::parse_id(args, "get_quotation", "quotation")?;
    let db = ctx.db.clone();
    let (row, related, amounts, extras) = lariv_core::rune_env::block_on_async(async move {
        let row = rows::find_one::<crate::entities::quotation_invoice::Entity>(
            &db,
            id,
            "quotation not found",
        )
        .await?;
        let related = rows::load_quotation_related(&db, row.id).await?;
        let amounts = rows::quotation_amounts(&db, row.id).await;
        let extras = crate::script_addon::enrich_invoice_script(
            &db,
            crate::script_addon::InvoiceScriptRecord::Quotation,
            row.id,
        )
        .await?;
        Ok::<_, String>((row, related, amounts, extras))
    })?;
    let mut mapped = rows::quotation_map(&row)?;
    rows::insert_amounts(&mut mapped, &amounts)?;
    rows::insert_related(&mut mapped, related)?;
    attach_script_fields(mapped, extras)
}

#[cfg(feature = "cap-llm")]
fn search_quotations(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use crate::entities::quotation_invoice::{self, Entity as QuotationInvoiceEntity};

    let (query, limit) = rows::parse_search(args, "search_quotations")?;
    let db = ctx.db.clone();
    let found = lariv_core::rune_env::block_on_async(async move {
        let matches = lariv_core::db::trigram::search::<QuotationInvoiceEntity, _>(
            &db,
            &[
                quotation_invoice::Column::Number,
                quotation_invoice::Column::Reference,
            ],
            &query,
            limit,
        )
        .await
        .map_err(|e| e.to_string())?;
        let mut amounts = Vec::with_capacity(matches.len());
        for row in &matches {
            amounts.push(rows::quotation_amounts(&db, row.id).await);
        }
        Ok::<_, String>((matches, amounts))
    })?;
    let results = found
        .0
        .iter()
        .zip(&found.1)
        .map(|(row, amounts)| rows::quotation_value(row, amounts))
        .collect::<Result<Vec<_>, _>>()?;
    rows::results(results)
}

#[cfg(feature = "cap-llm")]
mod args {
    use chrono::Utc;
    use serde::Deserialize;

    use crate::logic::draft::DraftLinePending;
    use crate::logic::{
        CreateDraftInput, DraftPaymentTermLineInput, UpdateDraftInput,
        default_payment_term_lines_json, parse_delivery_date, parse_invoice_datetime,
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
        qty_kind: Option<String>,
        #[serde(default)]
        qty_unit: Option<String>,
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
    pub(super) struct InvoiceFields {
        #[serde(default)]
        number: Option<String>,
        #[serde(default)]
        reference: Option<String>,
        #[serde(default)]
        eway_bill_number: Option<String>,
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
        customer_id: Option<i64>,
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
                quantity: line
                    .quantity
                    .map(NumberOrString::into_string)
                    .unwrap_or_else(|| "1".into()),
                qty_kind: line.qty_kind.unwrap_or_else(|| "quantity".into()),
                qty_unit: line.qty_unit.unwrap_or_default(),
                variables: line.variables,
                tax_ids: line.tax_ids,
                remarks: line.remarks,
                product_name: None,
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
                eway_bill_number: parsed.eway_bill_number,
                payment_reference: parsed.payment_reference,
                bank_account: parsed.bank_account,
                remarks: parsed.remarks,
                datetime,
                delivery_date,
                customer_id: parsed.customer_id.unwrap_or(0),
                payment_term_lines: payment_term_lines(parsed.payment_term_lines)?,
                header_tax_ids: parsed.header_tax_ids,
                lines: draft_lines(parsed.lines),
            },
            tz,
        ))
    }

    pub(super) fn parse_create_args(
        args: &[rune::Value],
        op: &str,
    ) -> Result<(CreateDraftInput, String), String> {
        let value = args
            .first()
            .ok_or_else(|| format!("{op} requires an object argument"))?;
        let parsed: InvoiceFields = serde_json::from_value(
            lariv_core::rune_env::rune_to_json(value)
                .map_err(|e| format!("invalid {op} arguments: {e}"))?,
        )
        .map_err(|e| format!("invalid {op} arguments: {e}"))?;
        parse_fields(parsed)
    }

    pub(super) fn parse_update_args(
        args: &[rune::Value],
        op: &str,
        id_label: &str,
    ) -> Result<(i64, UpdateDraftInput, String), String> {
        let value = args
            .first()
            .ok_or_else(|| format!("{op} requires an object argument"))?;
        let parsed: UpdateInvoiceArgs = serde_json::from_value(
            lariv_core::rune_env::rune_to_json(value)
                .map_err(|e| format!("invalid {op} arguments: {e}"))?,
        )
        .map_err(|e| format!("invalid {op} arguments: {e}"))?;
        if parsed.id <= 0 {
            return Err(format!("{op} requires a positive {id_label} id"));
        }
        let (create, tz) = parse_fields(parsed.fields)?;
        Ok((parsed.id, create.into_update(), tz))
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
            store: lariv_plugin_filesystem::storage::erase_store(Arc::clone(store)),
            session_id: None,
        }
    }

    fn registered_env() -> RuneEnvCapability {
        let mut cap = RuneEnvCapability::new();
        Hook.register_rune_env(&mut cap);
        cap
    }

    #[test]
    fn registers_invoice_bindings() {
        let names = registered_env().all_names();
        for name in [
            "create_draft_invoice",
            "get_draft_invoice",
            "search_draft_invoices",
            "update_draft_invoice",
            "delete_draft_invoice",
            "create_posted_invoice",
            "get_posted_invoice",
            "search_posted_invoices",
            "create_cancelled_invoice",
            "get_cancelled_invoice",
            "search_cancelled_invoices",
            "get_invoice_payment",
            "list_invoice_payments",
            "search_invoice_payments",
            "create_quotation",
            "get_quotation",
            "update_quotation",
            "search_quotations",
            "delete_quotation",
        ] {
            assert!(
                names.iter().any(|registered| registered == name),
                "expected {name} in {names:?}"
            );
        }
        assert!(
            !names.iter().any(|name| name == "search_invoices"),
            "search_invoices should be split by state"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_draft_invoice_via_rune_rejects_missing_customer() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_draft_invoice(#{ lines: [#{ product_id: 1, quantity: "1" }] })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("select a customer")
                || error.contains("invalid create_draft_invoice arguments"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_draft_invoice_via_rune_rejects_empty_lines() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            "create_draft_invoice(#{ customer_id: 1, lines: [] })",
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("add at least one line"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_draft_invoice_via_rune_rejects_missing_argument() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out =
            rune_engine::compile_and_run(&cap, &env_ctx, "create_draft_invoice(())", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("create_draft_invoice requires an object argument")
                || error.contains("unsupported"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_draft_invoice_via_rune_accepts_object_built_from_lets() {
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
create_draft_invoice(#{
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
            !error.contains("unsupported create_draft_invoice argument type"),
            "argument conversion failed: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_draft_invoice_via_rune_rejects_missing_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_draft_invoice(#{ customer_id: 1, lines: [#{ product_id: 1, quantity: "1" }] })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("invalid update_draft_invoice arguments") || error.contains("id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_draft_invoice_via_rune_rejects_non_positive_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_draft_invoice(#{ id: 0, customer_id: 1, lines: [#{ product_id: 1, quantity: "1" }] })"#,
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
    async fn update_draft_invoice_via_rune_rejects_empty_lines() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            "update_draft_invoice(#{ id: 1, customer_id: 1, lines: [] })",
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("add at least one line"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn state_tools_reject_missing_arguments() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let cases = [
            ("search_draft_invoices(#{})", "query"),
            ("search_posted_invoices(#{})", "query"),
            ("search_cancelled_invoices(#{})", "query"),
            ("search_invoice_payments(#{})", "query"),
            ("get_draft_invoice(#{})", "id"),
            ("get_posted_invoice(#{ id: 0 })", "positive"),
            ("get_cancelled_invoice(#{ id: 0 })", "positive"),
            ("get_quotation(#{ id: 0 })", "positive"),
            ("get_invoice_payment(#{ id: 0 })", "positive"),
            (
                "list_invoice_payments(#{ posted_invoice_id: 0 })",
                "positive",
            ),
            ("delete_draft_invoice(#{ id: 0 })", "positive"),
            ("create_posted_invoice(#{})", "draft_id"),
            (
                "create_cancelled_invoice(#{ posted_invoice_id: 1 })",
                "reason",
            ),
            (
                "create_cancelled_invoice(#{ posted_invoice_id: 0, reason: \"void\" })",
                "positive",
            ),
        ];
        for (source, needle) in cases {
            let out = rune_engine::compile_and_run(&cap, &env_ctx, source, &[]).await;
            let error = out
                .get("error")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            assert!(
                error.contains(needle),
                "{source} unexpected error payload: {out}"
            );
        }
    }

    #[test]
    fn update_draft_invoice_args_accept_object_fields() {
        use lariv_core::rune_env::json_to_rune;
        use serde_json::json;

        let value = json_to_rune(json!({
            "id": 42,
            "customer_id": 5,
            "number": "P26RIN100294",
            "date": "27/04/2026",
            "lines": [
                { "product_id": 1, "quantity": 2, "rate": 100 },
                { "product_id": 7, "quantity": 3, "rate": 50 }
            ]
        }))
        .expect("json to rune");
        let (id, input, tz) = parse_update_args(&[value], "update_draft_invoice", "draft invoice")
            .expect("parse update args");
        assert_eq!(id, 42);
        assert_eq!(input.customer_id, 5);
        assert_eq!(input.number.as_deref(), Some("P26RIN100294"));
        assert_eq!(input.lines.len(), 2);
        assert_eq!(input.lines[0].product_id, 1);
        assert_eq!(input.lines[1].product_id, 7);
        assert_eq!(input.lines[1].quantity, "3");
        assert_eq!(input.lines[1].rate.as_deref(), Some("50"));
        assert_eq!(tz, "UTC");
    }

    #[test]
    fn create_draft_invoice_args_require_at_least_one_line() {
        use lariv_core::rune_env::json_to_rune;
        use serde_json::json;

        let value = json_to_rune(json!({
            "customer_id": 5,
            "lines": [{ "product_id": 9, "quantity": "2" }]
        }))
        .expect("json to rune");
        let (input, _) =
            parse_create_args(&[value], "create_draft_invoice").expect("parse create args");
        assert_eq!(input.lines.len(), 1);
        assert_eq!(input.lines[0].product_id, 9);
        assert_eq!(input.lines[0].quantity, "2");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_quotation_via_rune_rejects_empty_lines() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            "create_quotation(#{ customer_id: 1, lines: [] })",
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("add at least one line"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_quotation_via_rune_rejects_non_positive_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_quotation(#{ id: 0, customer_id: 1, lines: [#{ product_id: 1, quantity: "1" }] })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("positive quotation id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_quotation_via_rune_rejects_empty_lines() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            "update_quotation(#{ id: 1, customer_id: 1, lines: [] })",
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("add at least one line"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn search_quotations_rejects_missing_query() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "search_quotations(#{})", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(error.contains("query"), "unexpected error payload: {out}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn delete_quotation_via_rune_rejects_non_positive_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out =
            rune_engine::compile_and_run(&cap, &env_ctx, "delete_quotation(#{ id: 0 })", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("positive quotation id"),
            "unexpected error payload: {out}"
        );
    }

    #[test]
    fn update_quotation_args_accept_object_fields() {
        use lariv_core::rune_env::json_to_rune;
        use serde_json::json;

        let value = json_to_rune(json!({
            "id": 7,
            "customer_id": 3,
            "number": "Q-100",
            "date": "27/04/2026",
            "lines": [{ "product_id": 2, "quantity": 4, "rate": 25 }]
        }))
        .expect("json to rune");
        let (id, input, tz) = parse_update_args(&[value], "update_quotation", "quotation")
            .expect("parse update args");
        assert_eq!(id, 7);
        assert_eq!(input.customer_id, 3);
        assert_eq!(input.number.as_deref(), Some("Q-100"));
        assert_eq!(input.lines.len(), 1);
        assert_eq!(input.lines[0].product_id, 2);
        assert_eq!(input.lines[0].quantity, "4");
        assert_eq!(input.lines[0].rate.as_deref(), Some("25"));
        assert_eq!(tz, "UTC");
    }
}
