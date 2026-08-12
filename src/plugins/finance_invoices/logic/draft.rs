//! Draft invoice create/update with line editor.

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, TransactionTrait,
};
use serde::Deserialize;

use crate::plugins::finance_common::decimal::{self, parse_decimal};
use crate::plugins::finance_products::{
    entities::product::Entity as ProductEntity,
    preferences::load_product_tax_ids,
};
use crate::plugins::finance_taxes::scope::load_taxes_by_ids;

use crate::plugins::finance_invoices::entities::{
    draft_invoice, draft_invoice_line,
    posted_invoice::Entity as PostedInvoiceEntity,
};
use crate::plugins::finance_invoices::logic::draft_payment_term::{
    upsert_draft_payment_term, DraftPaymentTermLineInput,
};
use crate::plugins::finance_invoices::logic::tax_assoc::{set_draft_invoice_taxes, set_draft_line_taxes};

#[derive(Debug, Deserialize, Clone)]
pub struct DraftLinePending {
    pub product_id: i64,
    pub rate: Option<String>,
    pub quantity: String,
    #[serde(default)]
    pub tax_ids: Option<Vec<i64>>,
}

/// Format an invoice datetime as a calendar date (`YYYY-MM-DD`) in `tz`.
pub fn format_invoice_date(dt: DateTime<Utc>, tz: &str) -> String {
    dt.with_timezone(&crate::datetime::parse_timezone(tz))
        .format("%Y-%m-%d")
        .to_string()
}

/// Parse an invoice date/datetime string into UTC.
///
/// Prefers HTML `type="date"` (`YYYY-MM-DD`) as start-of-day in `tz`, then
/// `datetime-local`, then a few legacy formats.
pub fn parse_invoice_datetime(s: &str, tz: &str) -> DateTime<Utc> {
    let s = s.trim();
    if s.is_empty() {
        return Utc::now();
    }
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        if let Some(naive) = date.and_hms_opt(0, 0, 0) {
            if let Some(dt) = crate::datetime::parse_timezone(tz)
                .from_local_datetime(&naive)
                .single()
            {
                return dt.with_timezone(&Utc);
            }
        }
    }
    crate::datetime::DatetimeLocalInput::from_raw(s)
        .to_stored(tz)
        .unwrap_or_else(|| {
            NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .map(|ndt| ndt.and_utc())
                .unwrap_or_else(|_| Utc::now())
        })
}

pub async fn err_if_draft_sealed(db: &DatabaseConnection, draft_id: i64) -> Result<(), String> {
    if draft_id == 0 {
        return Ok(());
    }
    let n = PostedInvoiceEntity::find()
        .filter(crate::plugins::finance_invoices::entities::posted_invoice::Column::DraftInvoiceId.eq(draft_id))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if n > 0 {
        return Err("draft invoice is posted and cannot be changed".to_string());
    }
    Ok(())
}

fn merge_tax_ids(header: &[i64], product: &[i64], line: Option<&[i64]>) -> Vec<i64> {
    if let Some(ids) = line {
        return ids.to_vec();
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for id in header.iter().chain(product.iter()) {
        if *id != 0 && seen.insert(*id) {
            out.push(*id);
        }
    }
    out
}

async fn build_line<C: ConnectionTrait>(
    db: &DatabaseConnection,
    txn: &C,
    draft_id: i64,
    row: &DraftLinePending,
    header_tax_ids: &[i64],
) -> Result<(draft_invoice_line::Model, Vec<i64>), String> {
    if row.product_id == 0 {
        return Err("choose a product for each line".to_string());
    }
    let qty = parse_decimal(&row.quantity)
        .filter(|d| *d > Decimal::ZERO)
        .ok_or_else(|| "quantity must be positive".to_string())?;
    let prod = ProductEntity::find_by_id(row.product_id)
        .one(txn)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("unknown product #{}", row.product_id))?;
    let rate = if let Some(r) = row.rate.as_ref().filter(|s| !s.trim().is_empty()) {
        let rate = parse_decimal(r).ok_or_else(|| "invalid rate".to_string())?;
        if rate < Decimal::ZERO {
            return Err("rate must be non-negative".to_string());
        }
        rate
    } else {
        prod.sales_price
    };
    let product_tax_ids = load_product_tax_ids(db, prod.id).await;
    let tax_ids = merge_tax_ids(header_tax_ids, &product_tax_ids, row.tax_ids.as_deref());
    if !tax_ids.is_empty() {
        let loaded = load_taxes_by_ids(db, &tax_ids).await.map_err(|e| e.to_string())?;
        if loaded.len() != tax_ids.len() {
            return Err("one or more line tax ids are invalid".to_string());
        }
    }
    let now = Utc::now();
    let line = draft_invoice_line::ActiveModel {
        draft_invoice_id: Set(draft_id),
        product_id: Set(row.product_id),
        rate: Set(decimal::normalize(rate)),
        quantity: Set(decimal::normalize(qty)),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(txn)
    .await
    .map_err(|e| e.to_string())?;
    Ok((line, tax_ids))
}

pub fn optional_trimmed_text(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn optional_display(opt: &Option<String>) -> String {
    opt.as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("—")
        .to_string()
}

pub struct CreateDraftInput {
    pub number: Option<String>,
    pub reference: Option<String>,
    pub payment_reference: Option<String>,
    pub bank_account: Option<String>,
    pub datetime: DateTime<Utc>,
    pub customer_id: i64,
    pub payment_term_lines: Vec<DraftPaymentTermLineInput>,
    pub header_tax_ids: Vec<i64>,
    pub lines: Vec<DraftLinePending>,
}

pub async fn create_draft_invoice(
    db: &DatabaseConnection,
    input: CreateDraftInput,
    tz: &str,
) -> Result<draft_invoice::Model, String> {
    if input.lines.is_empty() {
        return Err("add at least one invoice line".to_string());
    }
    if input.customer_id == 0 {
        return Err("customer is required".to_string());
    }

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now();
    let number = input
        .number
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());
    let draft = draft_invoice::ActiveModel {
        number: Set(number),
        reference: Set(input.reference),
        payment_reference: Set(input.payment_reference),
        bank_account: Set(input.bank_account),
        datetime: Set(input.datetime),
        customer_id: Set(input.customer_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;

    upsert_draft_payment_term(&txn, draft.id, &input.payment_term_lines, tz).await?;

    set_draft_invoice_taxes(&txn, draft.id, &input.header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    for row in &input.lines {
        let (line, tax_ids) = build_line(db, &txn, draft.id, row, &input.header_tax_ids).await?;
        set_draft_line_taxes(&txn, line.id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(draft)
}

pub struct UpdateDraftInput {
    pub number: Option<String>,
    pub reference: Option<String>,
    pub payment_reference: Option<String>,
    pub bank_account: Option<String>,
    pub datetime: DateTime<Utc>,
    pub customer_id: i64,
    pub payment_term_lines: Vec<DraftPaymentTermLineInput>,
    pub header_tax_ids: Vec<i64>,
    pub lines: Vec<DraftLinePending>,
}

pub async fn update_draft_invoice(
    db: &DatabaseConnection,
    draft_id: i64,
    input: UpdateDraftInput,
    tz: &str,
) -> Result<draft_invoice::Model, String> {
    err_if_draft_sealed(db, draft_id).await?;
    if input.lines.is_empty() {
        return Err("add at least one invoice line".to_string());
    }

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now();
    let number = input
        .number
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());

    let mut am: draft_invoice::ActiveModel = draft_invoice::Entity::find_by_id(draft_id)
        .one(&txn)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "draft not found".to_string())?
        .into();
    am.number = Set(number);
    am.reference = Set(input.reference);
    am.payment_reference = Set(input.payment_reference);
    am.bank_account = Set(input.bank_account);
    am.datetime = Set(input.datetime);
    am.customer_id = Set(input.customer_id);
    am.updated_at = Set(Some(now));
    let draft = am.update(&txn).await.map_err(|e| e.to_string())?;

    upsert_draft_payment_term(&txn, draft.id, &input.payment_term_lines, tz).await?;

    set_draft_invoice_taxes(&txn, draft.id, &input.header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    draft_invoice_line::Entity::delete_many()
        .filter(draft_invoice_line::Column::DraftInvoiceId.eq(draft_id))
        .exec(&txn)
        .await
        .map_err(|e| e.to_string())?;

    for row in &input.lines {
        let (line, tax_ids) = build_line(db, &txn, draft.id, row, &input.header_tax_ids).await?;
        set_draft_line_taxes(&txn, line.id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(draft)
}

pub async fn delete_draft(db: &DatabaseConnection, draft_id: i64) -> Result<(), String> {
    err_if_draft_sealed(db, draft_id).await?;
    draft_invoice::Entity::delete_by_id(draft_id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn parse_header_tax_ids(s: &str) -> Vec<i64> {
    s.split(',')
        .filter_map(|p| p.trim().parse().ok())
        .filter(|id| *id > 0)
        .collect()
}

pub fn parse_lines_json(raw: &str) -> Result<Vec<DraftLinePending>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("add at least one invoice line".to_string());
    }
    serde_json::from_str(raw).map_err(|e| format!("invalid lines data: {e}"))
}
