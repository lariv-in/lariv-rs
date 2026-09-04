//! Draft invoice create/update with line editor.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, TransactionTrait,
};
use serde::Deserialize;

use crate::plugins::finance_common::decimal::{self, parse_decimal};
use crate::plugins::finance_products::{
    entities::product::Entity as ProductEntity, preferences::load_product_tax_ids,
};
use crate::plugins::finance_taxes::scope::load_taxes_by_ids;

use crate::plugins::finance_invoices::entities::{
    DraftPaymentTermEntity, cancelled_invoice, draft_invoice, draft_invoice_line, paid_invoice,
    partially_paid_invoice, posted_invoice, posted_invoice::Entity as PostedInvoiceEntity,
};
use crate::plugins::finance_invoices::logic::draft_payment_term::{
    DraftPaymentTermLineInput, upsert_draft_payment_term,
};
use crate::plugins::finance_invoices::logic::tax_assoc::{
    set_draft_invoice_taxes, set_draft_line_taxes,
};

#[derive(Debug, Deserialize, Clone)]
pub struct DraftLinePending {
    pub product_id: i64,
    pub rate: Option<String>,
    pub quantity: String,
    #[serde(default)]
    pub tax_ids: Option<Vec<i64>>,
}

/// Format an invoice datetime as a calendar date (`DD/MM/YYYY`) in `tz`.
/// For preference-driven display (hub, details, PDF), use [`super::preferences::InvoiceDateFormats`].
pub fn format_invoice_date(dt: DateTime<Utc>, tz: &str) -> String {
    crate::datetime::format_date_in_tz(dt, tz)
}

/// Parse an invoice date/datetime string into UTC.
///
/// Prefers a date-only `DD/MM/YYYY` (also ISO `YYYY-MM-DD`) as start-of-day in
/// `tz`, then a datetime text value, then a few legacy formats.
pub fn parse_invoice_datetime(s: &str, tz: &str) -> DateTime<Utc> {
    let s = s.trim();
    if s.is_empty() {
        return Utc::now();
    }
    if let Some(dt) = crate::datetime::parse_date_start_in_tz(s, tz) {
        return dt;
    }
    crate::datetime::DatetimeLocalInput::from_raw(s)
        .to_stored(tz)
        .unwrap_or_else(Utc::now)
}

/// Format an optional delivery date for form inputs (`DD/MM/YYYY`).
/// For preference-driven display, use [`super::preferences::InvoiceDateFormats`].
pub fn format_delivery_date(d: Option<NaiveDate>) -> String {
    d.map(crate::datetime::format_date).unwrap_or_default()
}

/// Parse an optional delivery date from form input.
pub fn parse_delivery_date(s: &str) -> Result<Option<NaiveDate>, String> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    crate::datetime::parse_date(s)
        .map(Some)
        .ok_or_else(|| "invalid delivery date".to_string())
}

/// Hub lifecycle of a draft invoice row (`draft_invoices` remains after posting).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvoiceState {
    Draft,
    Posted,
    PartiallyPaid,
    Paid,
    Cancelled,
}

impl InvoiceState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Posted => "posted",
            Self::PartiallyPaid => "partially paid",
            Self::Paid => "paid",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Current hub state for `draft_id` (draft / posted / partially paid / paid / cancelled).
pub async fn draft_invoice_state(
    db: &DatabaseConnection,
    draft_id: i64,
) -> Result<InvoiceState, String> {
    let posted = PostedInvoiceEntity::find()
        .filter(posted_invoice::Column::DraftInvoiceId.eq(draft_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    let Some(posted) = posted else {
        return Ok(InvoiceState::Draft);
    };

    let cancelled = cancelled_invoice::Entity::find()
        .filter(cancelled_invoice::Column::PostedInvoiceId.eq(posted.id))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if cancelled > 0 {
        return Ok(InvoiceState::Cancelled);
    }

    let paid = paid_invoice::Entity::find()
        .filter(paid_invoice::Column::PostedInvoiceId.eq(posted.id))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if paid > 0 {
        return Ok(InvoiceState::Paid);
    }

    let partial = partially_paid_invoice::Entity::find()
        .filter(partially_paid_invoice::Column::PostedInvoiceId.eq(posted.id))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if partial > 0 {
        return Ok(InvoiceState::PartiallyPaid);
    }

    Ok(InvoiceState::Posted)
}

fn err_if_not_draft_state(state: InvoiceState) -> Result<(), String> {
    if state == InvoiceState::Draft {
        Ok(())
    } else {
        Err(format!(
            "invoice is {} and cannot be changed",
            state.as_str()
        ))
    }
}

pub async fn err_if_draft_sealed(db: &DatabaseConnection, draft_id: i64) -> Result<(), String> {
    if draft_id == 0 {
        return Ok(());
    }
    err_if_not_draft_state(draft_invoice_state(db, draft_id).await?)
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
        let loaded = load_taxes_by_ids(db, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
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
    pub delivery_date: Option<NaiveDate>,
    pub customer_id: i64,
    pub payment_term_lines: Vec<DraftPaymentTermLineInput>,
    pub header_tax_ids: Vec<i64>,
    pub lines: Vec<DraftLinePending>,
}

pub async fn create_draft_invoice(
    db: &DatabaseConnection,
    input: CreateDraftInput,
    _tz: &str,
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
        delivery_date: Set(input.delivery_date),
        customer_id: Set(input.customer_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;

    upsert_draft_payment_term(&txn, draft.id, &input.payment_term_lines).await?;

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
    pub delivery_date: Option<NaiveDate>,
    pub customer_id: i64,
    pub payment_term_lines: Vec<DraftPaymentTermLineInput>,
    pub header_tax_ids: Vec<i64>,
    pub lines: Vec<DraftLinePending>,
}

pub async fn update_draft_invoice(
    db: &DatabaseConnection,
    draft_id: i64,
    input: UpdateDraftInput,
    _tz: &str,
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
    am.delivery_date = Set(input.delivery_date);
    am.customer_id = Set(input.customer_id);
    am.updated_at = Set(Some(now));
    let draft = am.update(&txn).await.map_err(|e| e.to_string())?;

    upsert_draft_payment_term(&txn, draft.id, &input.payment_term_lines).await?;

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

/// Partial draft update: only `Some` fields are written; `None` leaves the existing value.
#[derive(Clone)]
pub struct PatchDraftInput {
    pub number: Option<String>,
    pub reference: Option<String>,
    pub payment_reference: Option<String>,
    pub bank_account: Option<String>,
    pub datetime: Option<DateTime<Utc>>,
    pub delivery_date: Option<NaiveDate>,
    pub customer_id: Option<i64>,
    pub payment_term_lines: Option<Vec<DraftPaymentTermLineInput>>,
    pub header_tax_ids: Option<Vec<i64>>,
    pub lines: Option<Vec<DraftLinePending>>,
}

impl PatchDraftInput {
    pub fn is_empty(&self) -> bool {
        self.number.is_none()
            && self.reference.is_none()
            && self.payment_reference.is_none()
            && self.bank_account.is_none()
            && self.datetime.is_none()
            && self.delivery_date.is_none()
            && self.customer_id.is_none()
            && self.payment_term_lines.is_none()
            && self.header_tax_ids.is_none()
            && self.lines.is_none()
    }
}

pub async fn patch_draft_invoice(
    db: &DatabaseConnection,
    draft_id: i64,
    input: PatchDraftInput,
    _tz: &str,
) -> Result<draft_invoice::Model, String> {
    err_if_draft_sealed(db, draft_id).await?;
    if input.is_empty() {
        return Err("fill at least one field to update".to_string());
    }
    if let Some(ref lines) = input.lines {
        if lines.is_empty() {
            return Err("add at least one invoice line".to_string());
        }
    }

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now();

    let mut am: draft_invoice::ActiveModel = draft_invoice::Entity::find_by_id(draft_id)
        .one(&txn)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "draft not found".to_string())?
        .into();

    if let Some(number) = &input.number {
        let number = number.trim();
        am.number = Set(if number.is_empty() {
            None
        } else {
            Some(number.to_string())
        });
    }
    if let Some(reference) = &input.reference {
        am.reference = Set(Some(reference.clone()));
    }
    if let Some(payment_reference) = &input.payment_reference {
        am.payment_reference = Set(Some(payment_reference.clone()));
    }
    if let Some(bank_account) = &input.bank_account {
        am.bank_account = Set(Some(bank_account.clone()));
    }
    if let Some(datetime) = input.datetime {
        am.datetime = Set(datetime);
    }
    if let Some(delivery_date) = input.delivery_date {
        am.delivery_date = Set(Some(delivery_date));
    }
    if let Some(customer_id) = input.customer_id {
        if customer_id == 0 {
            return Err("customer is required".to_string());
        }
        am.customer_id = Set(customer_id);
    }
    am.updated_at = Set(Some(now));
    let draft = am.update(&txn).await.map_err(|e| e.to_string())?;

    if let Some(ref payment_term_lines) = input.payment_term_lines {
        upsert_draft_payment_term(&txn, draft.id, payment_term_lines).await?;
    }

    if let Some(ref header_tax_ids) = input.header_tax_ids {
        set_draft_invoice_taxes(&txn, draft.id, header_tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(ref lines) = input.lines {
        let header_tax_ids_for_lines = if let Some(ref header_tax_ids) = input.header_tax_ids {
            header_tax_ids.clone()
        } else {
            crate::plugins::finance_invoices::logic::tax_assoc::load_draft_invoice_tax_ids(
                db, draft_id,
            )
            .await
            .unwrap_or_default()
        };

        draft_invoice_line::Entity::delete_many()
            .filter(draft_invoice_line::Column::DraftInvoiceId.eq(draft_id))
            .exec(&txn)
            .await
            .map_err(|e| e.to_string())?;

        for row in lines {
            let (line, tax_ids) =
                build_line(db, &txn, draft.id, row, &header_tax_ids_for_lines).await?;
            set_draft_line_taxes(&txn, line.id, &tax_ids)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(draft)
}

pub async fn delete_draft(db: &DatabaseConnection, draft_id: i64) -> Result<(), String> {
    err_if_draft_sealed(db, draft_id).await?;
    let term_id = draft_invoice::Entity::find_by_id(draft_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|d| d.draft_payment_term_id);
    draft_invoice::Entity::delete_by_id(draft_id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    // Trigger `trg_draft_invoices_delete_payment_term` also deletes the term;
    // keep this in case the migration has not been applied yet.
    if let Some(term_id) = term_id {
        DraftPaymentTermEntity::delete_by_id(term_id)
            .exec(db)
            .await
            .map_err(|e| e.to_string())?;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoice_state_labels() {
        assert_eq!(InvoiceState::Draft.as_str(), "draft");
        assert_eq!(InvoiceState::Posted.as_str(), "posted");
        assert_eq!(InvoiceState::PartiallyPaid.as_str(), "partially paid");
        assert_eq!(InvoiceState::Paid.as_str(), "paid");
        assert_eq!(InvoiceState::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn err_if_not_draft_state_ok_for_draft() {
        assert_eq!(err_if_not_draft_state(InvoiceState::Draft), Ok(()));
    }

    #[test]
    fn err_if_not_draft_state_names_current_state() {
        assert_eq!(
            err_if_not_draft_state(InvoiceState::Posted).unwrap_err(),
            "invoice is posted and cannot be changed"
        );
        assert_eq!(
            err_if_not_draft_state(InvoiceState::Cancelled).unwrap_err(),
            "invoice is cancelled and cannot be changed"
        );
        assert_eq!(
            err_if_not_draft_state(InvoiceState::Paid).unwrap_err(),
            "invoice is paid and cannot be changed"
        );
        assert_eq!(
            err_if_not_draft_state(InvoiceState::PartiallyPaid).unwrap_err(),
            "invoice is partially paid and cannot be changed"
        );
    }
}
