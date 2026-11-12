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
        "create_draft_invoice",
        "create_draft_invoice(#{ customer_id?: int, lines?: [#{ product_id: int, quantity?: number|string, qty_kind?: string, qty_unit?: string, rate?: number|string, variables?: object, tax_ids?: [int], remarks?: string }], number?: string, reference?: string, eway_bill_number?: string, payment_reference?: string, bank_account?: string, remarks?: string, datetime?: string, date?: string, delivery_date?: string, timezone?: string, payment_term_lines?: [#{ date_kind: \"absolute\"|\"relative\"|\"relative_delivery\", amount_kind: \"absolute\"|\"relative\", due_date?: string, due_duration?: string, amount?: number|string, amount_percentage?: number|string }], header_tax_ids?: [int] }) -> int  // new draft invoice id. Provide at least one line (lines). Quantity is a measure: qty_kind is its kind (length, weight, quantity, …) and qty_unit is the unit for that kind. The magnitude multiplies the price of one product. Omitted kind is quantity and omitted unit is that kind's default (Number for a count). When the catalog item has variables, pass them; an omitted rate uses the sales formula, and a given rate is the price of one unit. Omitted quantity is 1. Omitted line remarks are filled from the product remarks.",
        |_ctx| NativeBinding::Function(Arc::new(create_draft_invoice)),
    );
    rune_env.register_contextual(
        "update_draft_invoice",
        "update_draft_invoice(#{ id: int, customer_id?: int, lines?: [#{ product_id: int, quantity?: number|string, qty_kind?: string, qty_unit?: string, rate?: number|string, variables?: object, tax_ids?: [int], remarks?: string }], number?: string, reference?: string, eway_bill_number?: string, payment_reference?: string, bank_account?: string, remarks?: string, datetime?: string, date?: string, delivery_date?: string, timezone?: string, payment_term_lines?: [#{ date_kind: \"absolute\"|\"relative\"|\"relative_delivery\", amount_kind: \"absolute\"|\"relative\", due_date?: string, due_duration?: string, amount?: number|string, amount_percentage?: number|string }], header_tax_ids?: [int] }) -> int  // updated draft invoice id (full replace; invoice must be in draft state). Provide at least one line (lines). Quantity is a measure: qty_kind is its kind and qty_unit is the unit for that kind. The magnitude multiplies the price of one product. Omitted kind is quantity. When the catalog item has variables, an omitted rate uses the sales formula, and a given rate is the price of one unit. Omitted quantity is 1. Omitted line remarks are filled from the product remarks.",
        |_ctx| NativeBinding::Function(Arc::new(update_draft_invoice)),
    );
    rune_env.register_contextual(
        "get_draft_invoice",
        "get_draft_invoice(#{ id: int }) -> #{ id: int, number: string|null, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null }",
        |_ctx| NativeBinding::Function(Arc::new(get_draft_invoice)),
    );
    rune_env.register_contextual(
        "search_draft_invoices",
        "search_draft_invoices(#{ query: string, limit?: int }) -> #{ results: [#{ id: int, number: string|null, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null }] }",
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
        "get_posted_invoice(#{ id: int }) -> #{ id: int, number: string, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null, posted_at: string|null, settlement: \"open\"|\"partial\"|\"paid\" }  // cancelled invoices are not returned",
        |_ctx| NativeBinding::Function(Arc::new(get_posted_invoice)),
    );
    rune_env.register_contextual(
        "search_posted_invoices",
        "search_posted_invoices(#{ query: string, limit?: int }) -> #{ results: [#{ id: int, number: string, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null, posted_at: string|null, settlement: \"open\"|\"partial\"|\"paid\" }] }  // excludes cancelled invoices. settlement is open, partial, or paid.",
        |_ctx| NativeBinding::Function(Arc::new(search_posted_invoices)),
    );
    rune_env.register_contextual(
        "create_cancelled_invoice",
        "create_cancelled_invoice(#{ posted_invoice_id: int, reason: string, cancelled_at?: string, timezone?: string }) -> int  // cancels the posted invoice and returns the cancelled invoice id",
        |_ctx| NativeBinding::Function(Arc::new(create_cancelled_invoice)),
    );
    rune_env.register_contextual(
        "get_cancelled_invoice",
        "get_cancelled_invoice(#{ id: int }) -> #{ id: int, posted_invoice_id: int, number: string, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null, posted_at: string|null, cancelled_at: string|null }",
        |_ctx| NativeBinding::Function(Arc::new(get_cancelled_invoice)),
    );
    rune_env.register_contextual(
        "search_cancelled_invoices",
        "search_cancelled_invoices(#{ query: string, limit?: int }) -> #{ results: [#{ id: int, posted_invoice_id: int, number: string, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null, posted_at: string|null, cancelled_at: string|null }] }",
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
        "create_quotation(#{ customer_id?: int, lines?: [#{ product_id: int, quantity?: number|string, qty_kind?: string, qty_unit?: string, rate?: number|string, variables?: object, tax_ids?: [int], remarks?: string }], number?: string, reference?: string, eway_bill_number?: string, payment_reference?: string, bank_account?: string, remarks?: string, datetime?: string, date?: string, delivery_date?: string, timezone?: string, payment_term_lines?: [#{ date_kind: \"absolute\"|\"relative\"|\"relative_delivery\", amount_kind: \"absolute\"|\"relative\", due_date?: string, due_duration?: string, amount?: number|string, amount_percentage?: number|string }], header_tax_ids?: [int] }) -> int  // new quotation id. Provide at least one line (lines). Quantity is a measure: qty_kind is its kind (length, weight, quantity, …) and qty_unit is the unit for that kind. The magnitude multiplies the price of one product. Omitted kind is quantity and omitted unit is that kind's default (Number for a count). When the catalog item has variables, pass them; an omitted rate uses the sales formula, and a given rate is the price of one unit. Omitted quantity is 1. Omitted line remarks are filled from the product remarks.",
        |_ctx| NativeBinding::Function(Arc::new(create_quotation)),
    );
    rune_env.register_contextual(
        "update_quotation",
        "update_quotation(#{ id: int, customer_id?: int, lines?: [#{ product_id: int, quantity?: number|string, qty_kind?: string, qty_unit?: string, rate?: number|string, variables?: object, tax_ids?: [int], remarks?: string }], number?: string, reference?: string, eway_bill_number?: string, payment_reference?: string, bank_account?: string, remarks?: string, datetime?: string, date?: string, delivery_date?: string, timezone?: string, payment_term_lines?: [#{ date_kind: \"absolute\"|\"relative\"|\"relative_delivery\", amount_kind: \"absolute\"|\"relative\", due_date?: string, due_duration?: string, amount?: number|string, amount_percentage?: number|string }], header_tax_ids?: [int] }) -> int  // updated quotation id (full replace). Provide at least one line (lines). Quantity is a measure: qty_kind is its kind and qty_unit is the unit for that kind. The magnitude multiplies the price of one product. Omitted kind is quantity. When the catalog item has variables, an omitted rate uses the sales formula, and a given rate is the price of one unit. Omitted quantity is 1. Omitted line remarks are filled from the product remarks.",
        |_ctx| NativeBinding::Function(Arc::new(update_quotation)),
    );
    rune_env.register_contextual(
        "get_quotation",
        "get_quotation(#{ id: int }) -> #{ id: int, number: string|null, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null }",
        |_ctx| NativeBinding::Function(Arc::new(get_quotation)),
    );
    rune_env.register_contextual(
        "search_quotations",
        "search_quotations(#{ query: string, limit?: int }) -> #{ results: [#{ id: int, number: string|null, reference: string|null, customer_id: int, datetime: string, delivery_date: string|null, remarks: string|null, eway_bill_number: string|null, payment_reference: string|null, bank_account: string|null }] }",
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
    let (input, tz) = parse_create_args(args, "create_draft_invoice")?;
    if input.customer_id <= 0 {
        return Err("select a customer".to_string());
    }
    let db = ctx.db.clone();
    let draft = lariv_core::rune_env::block_on_async(async move {
        crate::logic::create_draft_invoice(&db, input, &tz).await
    })?;
    Ok(rune::Value::from(draft.id))
}

#[cfg(feature = "cap-llm")]
fn update_draft_invoice(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
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
        crate::logic::update_draft_invoice(&db, draft_id, input, &tz).await
    })?;
    Ok(rune::Value::from(draft.id))
}

#[cfg(feature = "cap-llm")]
fn create_quotation(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (input, tz) = parse_create_args(args, "create_quotation")?;
    if input.customer_id <= 0 {
        return Err("select a customer".to_string());
    }
    let db = ctx.db.clone();
    let quotation = lariv_core::rune_env::block_on_async(async move {
        crate::logic::create_quotation_invoice(&db, input, &tz).await
    })?;
    Ok(rune::Value::from(quotation.id))
}

#[cfg(feature = "cap-llm")]
fn update_quotation(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (quotation_id, input, tz) = parse_update_args(args, "update_quotation", "quotation")?;
    if input.lines.is_empty() {
        return Err("add at least one line".to_string());
    }
    if input.customer_id <= 0 {
        return Err("select a customer".to_string());
    }
    let db = ctx.db.clone();
    let quotation = lariv_core::rune_env::block_on_async(async move {
        crate::logic::update_quotation_invoice(&db, quotation_id, input, &tz).await
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
    use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
    use serde::Deserialize;

    use crate::entities::{
        cancelled_invoice, draft_invoice, paid_invoice, partially_paid_invoice, posted_invoice,
        quotation_invoice,
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

    fn header_value(header: Header) -> Result<rune::Value, String> {
        rune::to_value(header_map(header)?).map_err(|e| e.to_string())
    }

    pub(super) fn draft_value(row: &draft_invoice::Model) -> Result<rune::Value, String> {
        header_value(Header {
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

    pub(super) fn quotation_value(row: &quotation_invoice::Model) -> Result<rune::Value, String> {
        header_value(Header {
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

    pub(super) fn posted_value(
        row: &posted_invoice::Model,
        settlement: &str,
    ) -> Result<rune::Value, String> {
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
        rune::to_value(mapped).map_err(|e| e.to_string())
    }

    pub(super) fn cancelled_value(row: &cancelled_invoice::Model) -> Result<rune::Value, String> {
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
        rune::to_value(mapped).map_err(|e| e.to_string())
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
}

#[cfg(feature = "cap-llm")]
fn get_draft_invoice(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = rows::parse_id(args, "get_draft_invoice", "draft invoice")?;
    let db = ctx.db.clone();
    let row = lariv_core::rune_env::block_on_async(async move {
        rows::find_one::<crate::entities::draft_invoice::Entity>(&db, id, "draft invoice not found")
            .await
    })?;
    rows::draft_value(&row)
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
        lariv_core::db::trigram::search::<DraftInvoiceEntity, _>(
            &db,
            &[
                draft_invoice::Column::Number,
                draft_invoice::Column::Reference,
            ],
            &query,
            limit,
        )
        .await
        .map_err(|e| e.to_string())
    })?;
    let results = found
        .iter()
        .map(rows::draft_value)
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
    let row = lariv_core::rune_env::block_on_async(async move {
        let row = PostedInvoiceEntity::find_by_id(id)
            .filter(sql_posted_not_cancelled())
            .one(&db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "posted invoice not found".to_string())?;
        let labels = rows::posted_settlements(&db, &[row.id]).await?;
        let settlement = labels.get(&row.id).copied().unwrap_or("open");
        Ok::<_, String>((row, settlement))
    })?;
    rows::posted_value(&row.0, row.1)
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
        Ok::<_, String>((rows, labels))
    })?;
    let results = found
        .0
        .iter()
        .map(|row| {
            let settlement = found.1.get(&row.id).copied().unwrap_or("open");
            rows::posted_value(row, settlement)
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
    let row = lariv_core::rune_env::block_on_async(async move {
        rows::find_one::<crate::entities::cancelled_invoice::Entity>(
            &db,
            id,
            "cancelled invoice not found",
        )
        .await
    })?;
    rows::cancelled_value(&row)
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
        lariv_core::db::trigram::search::<CancelledInvoiceEntity, _>(
            &db,
            &[
                cancelled_invoice::Column::Number,
                cancelled_invoice::Column::Reference,
            ],
            &query,
            limit,
        )
        .await
        .map_err(|e| e.to_string())
    })?;
    let results = found
        .iter()
        .map(rows::cancelled_value)
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
    let row = lariv_core::rune_env::block_on_async(async move {
        rows::find_one::<crate::entities::quotation_invoice::Entity>(&db, id, "quotation not found")
            .await
    })?;
    rows::quotation_value(&row)
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
        lariv_core::db::trigram::search::<QuotationInvoiceEntity, _>(
            &db,
            &[
                quotation_invoice::Column::Number,
                quotation_invoice::Column::Reference,
            ],
            &query,
            limit,
        )
        .await
        .map_err(|e| e.to_string())
    })?;
    let results = found
        .iter()
        .map(rows::quotation_value)
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
        Ok((
            parsed.id,
            UpdateDraftInput {
                number: create.number,
                reference: create.reference,
                eway_bill_number: create.eway_bill_number,
                payment_reference: create.payment_reference,
                bank_account: create.bank_account,
                remarks: create.remarks,
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
