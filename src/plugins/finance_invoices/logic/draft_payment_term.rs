//! Draft and posted payment term CRUD, validation, and lifecycle conversion.

use std::collections::HashSet;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Statement,
};
use serde::Deserialize;

use crate::plugins::finance_common::decimal::{self, parse_decimal};
use crate::plugins::finance_taxes::entities::tax::{self, Entity as TaxEntity, Model as TaxModel};

use crate::plugins::finance_invoices::entities::{
    CancelledInvoiceEntity, DraftInvoiceEntity, DraftPaymentTermEntity, DraftPaymentTermLineEntity,
    PostedInvoiceEntity, PostedInvoiceLineEntity, PostedPaymentTermEntity,
    PostedPaymentTermLineEntity,
};
use crate::plugins::finance_invoices::entities::{
    draft_invoice, draft_payment_term, draft_payment_term_line, posted_invoice_line,
    posted_payment_term, posted_payment_term_line,
};
use crate::plugins::finance_invoices::logic::tax_assoc::{
    load_cancelled_invoice_tax_ids, load_cancelled_line_tax_ids, load_posted_invoice_tax_ids,
    load_posted_line_tax_ids,
};
use crate::plugins::finance_invoices::logic::tax_calculations::{
    InvoiceLinesTotals, invoice_line_amount_breakdown, invoice_receivable_grand_total,
    merge_invoice_line_tax_ids,
};
use crate::plugins::finance_invoices::{PaymentTermAmountKind, PaymentTermDateKind};

const PAYMENT_TERM_TYPE_DUE_DATE: &str = "p_finance_invoices.PaymentTermDueDate";
const PAYMENT_TERM_TYPE_RELATIVE: &str = "p_finance_invoices.PaymentTermRelative";

const PERCENTAGE_TOLERANCE: Decimal = Decimal::ONE; // ±1% for draft save validation

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct DraftPaymentTermLineInput {
    pub date_kind: PaymentTermDateKind,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub due_duration: Option<String>,
    pub amount_kind: PaymentTermAmountKind,
    #[serde(default)]
    pub amount: Option<String>,
    #[serde(default)]
    pub amount_percentage: Option<String>,
}

#[derive(Clone)]
pub struct PaymentTermLineDisplayRow {
    pub due_display: String,
    pub amount_display: String,
}

pub fn default_payment_term_lines_json() -> String {
    r#"[{"date_kind":"relative","due_date":"","due_duration":"15 days","amount_kind":"relative","amount":"","amount_percentage":"100"}]"#
    .to_string()
}

pub fn parse_payment_term_lines_json(raw: &str) -> Result<Vec<DraftPaymentTermLineInput>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("add at least one payment term line".to_string());
    }
    serde_json::from_str(raw).map_err(|e| format!("invalid payment term lines: {e}"))
}

pub fn parse_due_date_for_term(s: &str) -> Result<NaiveDate, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("due date is required for absolute date".to_string());
    }
    crate::datetime::parse_date(s).ok_or_else(|| "invalid due date".to_string())
}

fn relative_duration_fields(
    line: &DraftPaymentTermLineInput,
) -> Result<(Option<NaiveDate>, Option<i64>), String> {
    let nanos = crate::duration::parse_duration(line.due_duration.as_deref().unwrap_or(""))
        .map_err(|e| e.to_string())?;
    Ok((None, Some(nanos)))
}

fn resolve_relative_due(
    due_duration: Option<i64>,
    anchor: DateTime<Utc>,
    tz: &str,
) -> Result<NaiveDate, String> {
    let dur = due_duration.ok_or_else(|| "missing relative duration".to_string())?;
    let resolved = anchor
        .checked_add_signed(Duration::nanoseconds(dur))
        .ok_or_else(|| "due date overflow".to_string())?;
    Ok(resolved
        .with_timezone(&crate::datetime::parse_timezone(tz))
        .date_naive())
}

fn resolve_legacy_relative_due(anchor: DateTime<Utc>, due_duration: Option<i64>) -> NaiveDate {
    let resolved = anchor
        .checked_add_signed(Duration::nanoseconds(due_duration.unwrap_or(0)))
        .unwrap_or(anchor);
    resolved
        .with_timezone(&crate::datetime::parse_timezone(
            crate::datetime::DEFAULT_TIMEZONE,
        ))
        .date_naive()
}

fn datetime_to_due_date(dt: DateTime<Utc>) -> NaiveDate {
    dt.with_timezone(&crate::datetime::parse_timezone(
        crate::datetime::DEFAULT_TIMEZONE,
    ))
    .date_naive()
}

fn validate_line_input(line: &DraftPaymentTermLineInput) -> Result<(), String> {
    match line.date_kind {
        PaymentTermDateKind::Absolute => {
            parse_due_date_for_term(line.due_date.as_deref().unwrap_or(""))?;
        }
        PaymentTermDateKind::Relative => {
            let dur = line.due_duration.as_deref().unwrap_or("").trim();
            if dur.is_empty() {
                return Err("duration is required for relative date".to_string());
            }
            let nanos = crate::duration::parse_duration(dur)
                .map_err(|e| format!("invalid duration: {e}"))?;
            if nanos <= 0 {
                return Err("duration must be positive".to_string());
            }
        }
        PaymentTermDateKind::RelativeDelivery => {
            return Err("invalid date kind: relative_delivery".to_string());
        }
    }

    match line.amount_kind {
        PaymentTermAmountKind::Absolute => {
            let amt = parse_decimal(line.amount.as_deref().unwrap_or(""))
                .ok_or_else(|| "amount is required for absolute amount".to_string())?;
            if amt <= Decimal::ZERO {
                return Err("amount must be positive".to_string());
            }
        }
        PaymentTermAmountKind::Relative => {
            let pct = parse_decimal(line.amount_percentage.as_deref().unwrap_or(""))
                .ok_or_else(|| "percentage is required for relative amount".to_string())?;
            if pct <= Decimal::ZERO {
                return Err("percentage must be positive".to_string());
            }
        }
    }
    Ok(())
}

pub fn validate_draft_payment_term_lines(
    lines: &[DraftPaymentTermLineInput],
) -> Result<(), String> {
    if lines.is_empty() {
        return Err("add at least one payment term line".to_string());
    }
    for line in lines {
        validate_line_input(line)?;
    }

    let all_relative = lines
        .iter()
        .all(|l| l.amount_kind == PaymentTermAmountKind::Relative);
    if all_relative {
        let sum: Decimal = lines
            .iter()
            .map(|l| {
                parse_decimal(l.amount_percentage.as_deref().unwrap_or("")).unwrap_or(Decimal::ZERO)
            })
            .sum();
        let diff = (sum - Decimal::from(100)).abs();
        if diff > PERCENTAGE_TOLERANCE {
            return Err(format!("relative percentages must sum to 100 (got {sum})"));
        }
    }
    Ok(())
}

fn line_input_to_active(
    draft_payment_term_id: i64,
    line_order: i32,
    line: &DraftPaymentTermLineInput,
    now: DateTime<Utc>,
) -> Result<draft_payment_term_line::ActiveModel, String> {
    let (due_date, due_duration) = match line.date_kind {
        PaymentTermDateKind::Absolute => (
            Some(parse_due_date_for_term(
                line.due_date.as_deref().unwrap_or(""),
            )?),
            None,
        ),
        PaymentTermDateKind::Relative => relative_duration_fields(line)?,
        PaymentTermDateKind::RelativeDelivery => {
            return Err("invalid date kind: relative_delivery".to_string());
        }
    };

    let (amount, amount_percentage) = match line.amount_kind {
        PaymentTermAmountKind::Absolute => (
            Some(decimal::normalize(
                parse_decimal(line.amount.as_deref().unwrap_or("")).unwrap(),
            )),
            None,
        ),
        PaymentTermAmountKind::Relative => (
            None,
            Some(decimal::normalize(
                parse_decimal(line.amount_percentage.as_deref().unwrap_or("")).unwrap(),
            )),
        ),
    };

    Ok(draft_payment_term_line::ActiveModel {
        draft_payment_term_id: Set(draft_payment_term_id),
        line_order: Set(line_order),
        date_kind: Set(line.date_kind),
        due_date: Set(due_date),
        due_duration: Set(due_duration),
        amount_kind: Set(line.amount_kind),
        amount: Set(amount),
        amount_percentage: Set(amount_percentage),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    })
}

async fn draft_invoice_term_id<C: ConnectionTrait>(
    conn: &C,
    draft_id: i64,
) -> Result<Option<i64>, String> {
    Ok(DraftInvoiceEntity::find_by_id(draft_id)
        .one(conn)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|d| d.draft_payment_term_id))
}

async fn set_draft_invoice_term_id<C: ConnectionTrait>(
    conn: &C,
    draft_id: i64,
    term_id: i64,
) -> Result<(), String> {
    let mut am: draft_invoice::ActiveModel = DraftInvoiceEntity::find_by_id(draft_id)
        .one(conn)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "draft not found".to_string())?
        .into();
    am.draft_payment_term_id = Set(Some(term_id));
    am.update(conn).await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn upsert_draft_payment_term_lines<C: ConnectionTrait>(
    conn: &C,
    existing_term_id: Option<i64>,
    lines: &[DraftPaymentTermLineInput],
) -> Result<draft_payment_term::Model, String> {
    validate_draft_payment_term_lines(lines)?;
    let now = Utc::now();

    let term = if let Some(id) = existing_term_id {
        DraftPaymentTermEntity::find_by_id(id)
            .one(conn)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "draft payment term not found".to_string())?
    } else {
        draft_payment_term::ActiveModel {
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(conn)
        .await
        .map_err(|e| e.to_string())?
    };

    DraftPaymentTermLineEntity::delete_many()
        .filter(draft_payment_term_line::Column::DraftPaymentTermId.eq(term.id))
        .exec(conn)
        .await
        .map_err(|e| e.to_string())?;

    for (i, line) in lines.iter().enumerate() {
        let am = line_input_to_active(term.id, i as i32, line, now)?;
        am.insert(conn).await.map_err(|e| e.to_string())?;
    }

    Ok(term)
}

pub async fn upsert_draft_payment_term<C: ConnectionTrait>(
    conn: &C,
    draft_id: i64,
    lines: &[DraftPaymentTermLineInput],
) -> Result<draft_payment_term::Model, String> {
    let existing_id = draft_invoice_term_id(conn, draft_id).await?;
    let term = upsert_draft_payment_term_lines(conn, existing_id, lines).await?;
    if existing_id.is_none() {
        set_draft_invoice_term_id(conn, draft_id, term.id).await?;
    }
    Ok(term)
}

async fn load_draft_lines_for_term<C: ConnectionTrait>(
    conn: &C,
    term_id: i64,
) -> Result<Vec<draft_payment_term_line::Model>, String> {
    DraftPaymentTermLineEntity::find()
        .filter(draft_payment_term_line::Column::DraftPaymentTermId.eq(term_id))
        .order_by_asc(draft_payment_term_line::Column::LineOrder)
        .order_by_asc(draft_payment_term_line::Column::Id)
        .all(conn)
        .await
        .map_err(|e| e.to_string())
}

async fn load_posted_lines_for_term<C: ConnectionTrait>(
    conn: &C,
    term_id: i64,
) -> Result<Vec<posted_payment_term_line::Model>, String> {
    PostedPaymentTermLineEntity::find()
        .filter(posted_payment_term_line::Column::PostedPaymentTermId.eq(term_id))
        .order_by_asc(posted_payment_term_line::Column::LineOrder)
        .order_by_asc(posted_payment_term_line::Column::Id)
        .all(conn)
        .await
        .map_err(|e| e.to_string())
}

pub async fn load_draft_payment_term_lines(
    db: &DatabaseConnection,
    draft_id: i64,
) -> Result<Vec<draft_payment_term_line::Model>, String> {
    let Some(term_id) = draft_invoice_term_id(db, draft_id).await? else {
        return Ok(Vec::new());
    };
    load_draft_lines_for_term(db, term_id).await
}

pub fn format_due_date_input(d: NaiveDate) -> String {
    crate::datetime::format_date(d)
}

pub async fn payment_term_lines_form_json_for_term<C: ConnectionTrait>(
    conn: &C,
    term_id: Option<i64>,
) -> String {
    let lines = match term_id {
        Some(id) => load_draft_lines_for_term(conn, id)
            .await
            .unwrap_or_default(),
        None => Vec::new(),
    };
    if lines.is_empty() {
        return default_payment_term_lines_json();
    }
    let out: Vec<serde_json::Value> = lines
        .iter()
        .map(|l| {
            let due_date = l
                .due_date
                .map(format_due_date_input)
                .unwrap_or_default();
            let due_duration = l
                .due_duration
                .map(|d| crate::duration::format_duration(d))
                .unwrap_or_default();
            serde_json::json!({
                "date_kind": l.date_kind,
                "due_date": due_date,
                "due_duration": due_duration,
                "amount_kind": l.amount_kind,
                "amount": l.amount.map(decimal::decimal_display).unwrap_or_default(),
                "amount_percentage": l.amount_percentage.map(decimal::decimal_display).unwrap_or_default(),
            })
        })
        .collect();
    serde_json::to_string(&out).unwrap_or_else(|_| default_payment_term_lines_json())
}

pub async fn payment_term_lines_form_json(db: &DatabaseConnection, draft_id: i64) -> String {
    let term_id = crate::web::opt_or_log(
        draft_invoice_term_id(db, draft_id).await,
        "draft invoice term id",
    );
    payment_term_lines_form_json_for_term(db, term_id).await
}

pub fn resolve_due_date(
    line: &draft_payment_term_line::Model,
    anchor: DateTime<Utc>,
    tz: &str,
) -> Result<NaiveDate, String> {
    match line.date_kind {
        PaymentTermDateKind::Absolute => line
            .due_date
            .ok_or_else(|| "missing absolute due date".to_string()),
        PaymentTermDateKind::Relative => resolve_relative_due(line.due_duration, anchor, tz),
        PaymentTermDateKind::RelativeDelivery => {
            Err("invalid date kind: relative_delivery".to_string())
        }
    }
}

fn resolve_amount(
    line: &draft_payment_term_line::Model,
    grand_total: Decimal,
) -> Result<Decimal, String> {
    match line.amount_kind {
        PaymentTermAmountKind::Absolute => line
            .amount
            .ok_or_else(|| "missing absolute amount".to_string()),
        PaymentTermAmountKind::Relative => {
            let pct = line
                .amount_percentage
                .ok_or_else(|| "missing relative percentage".to_string())?;
            Ok(decimal::dec_mul(grand_total, pct / Decimal::from(100)))
        }
    }
}

pub fn validate_posting_amounts(
    lines: &[draft_payment_term_line::Model],
    grand_total: Decimal,
) -> Result<(), String> {
    if lines.is_empty() {
        return Err("payment term must have at least one line".to_string());
    }
    let all_absolute = lines
        .iter()
        .all(|l| l.amount_kind == PaymentTermAmountKind::Absolute);
    if all_absolute {
        let sum: Decimal = lines.iter().filter_map(|l| l.amount).sum();
        if sum != grand_total {
            return Err(format!(
                "absolute payment term amounts must sum to invoice total ({grand_total})"
            ));
        }
    }
    Ok(())
}

pub async fn convert_draft_to_posted_payment_term<C: ConnectionTrait>(
    conn: &C,
    draft_id: i64,
    anchor: DateTime<Utc>,
    grand_total: Decimal,
    tz: &str,
) -> Result<posted_payment_term::Model, String> {
    let draft_lines = load_draft_payment_term_lines_conn(conn, draft_id).await?;
    validate_posting_amounts(&draft_lines, grand_total)?;

    let now = Utc::now();
    let mut resolved: Vec<(NaiveDate, Decimal)> = Vec::with_capacity(draft_lines.len());
    for line in &draft_lines {
        resolved.push((
            resolve_due_date(line, anchor, tz)?,
            resolve_amount(line, grand_total)?,
        ));
    }

    // Adjust final line so amounts sum exactly to grand_total
    if !resolved.is_empty() {
        let sum: Decimal = resolved.iter().map(|(_, a)| *a).sum();
        let diff = decimal::dec_sub(grand_total, sum);
        if !decimal::dec_is_zero(diff) {
            let last = resolved.len() - 1;
            resolved[last].1 = decimal::dec_sum(resolved[last].1, diff);
        }
    }

    let term = posted_payment_term::ActiveModel {
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(conn)
    .await
    .map_err(|e| e.to_string())?;

    for (i, (due_date, amount)) in resolved.into_iter().enumerate() {
        posted_payment_term_line::ActiveModel {
            posted_payment_term_id: Set(term.id),
            line_order: Set(i as i32),
            due_date: Set(due_date),
            amount: Set(decimal::normalize(amount)),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(conn)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(term)
}

async fn load_draft_payment_term_lines_conn<C: ConnectionTrait>(
    conn: &C,
    draft_id: i64,
) -> Result<Vec<draft_payment_term_line::Model>, String> {
    let Some(term_id) = draft_invoice_term_id(conn, draft_id).await? else {
        return Err("draft payment term not found".to_string());
    };
    load_draft_lines_for_term(conn, term_id).await
}

async fn load_posted_payment_term_by_id<C: ConnectionTrait>(
    conn: &C,
    term_id: i64,
) -> Result<
    Option<(
        posted_payment_term::Model,
        Vec<posted_payment_term_line::Model>,
    )>,
    String,
> {
    let Some(term) = PostedPaymentTermEntity::find_by_id(term_id)
        .one(conn)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    let lines = load_posted_lines_for_term(conn, term.id).await?;
    Ok(Some((term, lines)))
}

pub async fn load_posted_payment_term_for_posted(
    db: &DatabaseConnection,
    posted_invoice_id: i64,
) -> Result<
    Option<(
        posted_payment_term::Model,
        Vec<posted_payment_term_line::Model>,
    )>,
    String,
> {
    let Some(term_id) = PostedInvoiceEntity::find_by_id(posted_invoice_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|p| p.posted_payment_term_id)
    else {
        return Ok(None);
    };
    load_posted_payment_term_by_id(db, term_id).await
}

pub async fn load_posted_payment_term_for_cancelled(
    db: &DatabaseConnection,
    cancelled_invoice_id: i64,
) -> Result<
    Option<(
        posted_payment_term::Model,
        Vec<posted_payment_term_line::Model>,
    )>,
    String,
> {
    let Some(term_id) = CancelledInvoiceEntity::find_by_id(cancelled_invoice_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|c| c.posted_payment_term_id)
    else {
        return Ok(None);
    };
    load_posted_payment_term_by_id(db, term_id).await
}

pub async fn copy_posted_payment_term<C: ConnectionTrait>(
    conn: &C,
    source_term_id: Option<i64>,
) -> Result<Option<i64>, String> {
    let Some(source_term_id) = source_term_id else {
        return Ok(None);
    };
    let Some((_, lines)) = load_posted_payment_term_by_id(conn, source_term_id).await? else {
        return Ok(None);
    };

    let now = Utc::now();
    let new_term = posted_payment_term::ActiveModel {
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(conn)
    .await
    .map_err(|e| e.to_string())?;

    for line in lines {
        posted_payment_term_line::ActiveModel {
            posted_payment_term_id: Set(new_term.id),
            line_order: Set(line.line_order),
            due_date: Set(line.due_date),
            amount: Set(line.amount),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(conn)
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(Some(new_term.id))
}

pub async fn posted_payment_term_to_draft<C: ConnectionTrait>(
    conn: &C,
    source_term_id: Option<i64>,
    draft_id: i64,
) -> Result<(), String> {
    let Some(source_term_id) = source_term_id else {
        return Ok(());
    };
    let Some((_, lines)) = load_posted_payment_term_by_id(conn, source_term_id).await? else {
        return Ok(());
    };

    if lines.is_empty() {
        return Ok(());
    }

    let inputs: Vec<DraftPaymentTermLineInput> = lines
        .iter()
        .map(|l| DraftPaymentTermLineInput {
            date_kind: PaymentTermDateKind::Absolute,
            due_date: Some(format_due_date_input(l.due_date)),
            due_duration: None,
            amount_kind: PaymentTermAmountKind::Absolute,
            amount: Some(decimal::decimal_display(l.amount)),
            amount_percentage: None,
        })
        .collect();

    upsert_draft_payment_term(conn, draft_id, &inputs).await?;
    Ok(())
}

pub fn draft_payment_term_line_display(
    line: &draft_payment_term_line::Model,
) -> PaymentTermLineDisplayRow {
    let due_display = match line.date_kind {
        PaymentTermDateKind::Absolute => line
            .due_date
            .map(crate::datetime::format_date)
            .unwrap_or_else(|| "—".to_string()),
        PaymentTermDateKind::Relative => line
            .due_duration
            .map(crate::duration::format_duration)
            .unwrap_or_else(|| "—".to_string()),
        PaymentTermDateKind::RelativeDelivery => line
            .due_duration
            .map(crate::duration::format_duration)
            .unwrap_or_else(|| "—".to_string()),
    };
    let amount_display = match line.amount_kind {
        PaymentTermAmountKind::Absolute => line
            .amount
            .map(decimal::decimal_display)
            .unwrap_or_else(|| "—".to_string()),
        PaymentTermAmountKind::Relative => line
            .amount_percentage
            .map(|p| format!("{}%", decimal::decimal_display(p)))
            .unwrap_or_else(|| "—".to_string()),
    };
    PaymentTermLineDisplayRow {
        due_display,
        amount_display,
    }
}

pub fn posted_payment_term_line_display(
    line: &posted_payment_term_line::Model,
    minor_unit: i32,
    symbol: &str,
) -> PaymentTermLineDisplayRow {
    PaymentTermLineDisplayRow {
        due_display: crate::datetime::format_date(line.due_date),
        amount_display: decimal::decimal_display_currency(line.amount, minor_unit, symbol),
    }
}

pub async fn draft_payment_term_display_rows(
    db: &DatabaseConnection,
    draft_id: i64,
) -> Vec<PaymentTermLineDisplayRow> {
    load_draft_payment_term_lines(db, draft_id)
        .await
        .unwrap_or_default()
        .iter()
        .map(draft_payment_term_line_display)
        .collect()
}

pub async fn posted_payment_term_display_rows(
    db: &DatabaseConnection,
    posted_invoice_id: i64,
    minor_unit: i32,
    symbol: &str,
) -> Vec<PaymentTermLineDisplayRow> {
    match load_posted_payment_term_for_posted(db, posted_invoice_id).await {
        Ok(Some((_, lines))) => lines
            .iter()
            .map(|l| posted_payment_term_line_display(l, minor_unit, symbol))
            .collect(),
        _ => Vec::new(),
    }
}

pub async fn cancelled_payment_term_display_rows(
    db: &DatabaseConnection,
    cancelled_invoice_id: i64,
    minor_unit: i32,
    symbol: &str,
) -> Vec<PaymentTermLineDisplayRow> {
    match load_posted_payment_term_for_cancelled(db, cancelled_invoice_id).await {
        Ok(Some((_, lines))) => lines
            .iter()
            .map(|l| posted_payment_term_line_display(l, minor_unit, symbol))
            .collect(),
        _ => Vec::new(),
    }
}

/// Compute receivable grand total from draft invoice lines and taxes (for posting conversion).
pub fn compute_draft_grand_total(
    lines: &[(Decimal, Decimal, Vec<TaxModel>)],
    header_taxes: &[TaxModel],
) -> Decimal {
    let mut line_totals = InvoiceLinesTotals::default();
    let mut line_tax_ids = HashSet::new();
    for (qty, rate, taxes) in lines {
        let (u, lev, wh, _) = invoice_line_amount_breakdown(*qty, *rate, taxes);
        line_totals.untaxed_subtotal = decimal::dec_sum(line_totals.untaxed_subtotal, u);
        line_totals.lines_levied = decimal::dec_sum(line_totals.lines_levied, lev);
        line_totals.lines_withholding = decimal::dec_sum(line_totals.lines_withholding, wh);
        merge_invoice_line_tax_ids(&mut line_tax_ids, taxes);
    }
    invoice_receivable_grand_total(&line_totals, header_taxes, &line_tax_ids)
}

/// Migrate legacy polymorphic payment_terms into the new model. Called from migration m00017.
pub async fn migrate_legacy_payment_terms<C: ConnectionTrait>(conn: &C) -> Result<(), String> {
    use sea_orm::Statement;

    let draft_rows = conn
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT id, payment_term_id FROM draft_invoices WHERE payment_term_id IS NOT NULL AND payment_term_id > 0".to_string(),
        ))
        .await
        .map_err(|e| e.to_string())?;

    for row in draft_rows {
        let draft_id: i64 = row.try_get("", "id").map_err(|e| e.to_string())?;
        let payment_term_id: i64 = row
            .try_get("", "payment_term_id")
            .map_err(|e| e.to_string())?;
        migrate_draft_legacy(conn, draft_id, payment_term_id).await?;
    }

    let posted_rows = conn
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT id, datetime, payment_term_id FROM posted_invoices WHERE payment_term_id IS NOT NULL AND payment_term_id > 0".to_string(),
        ))
        .await
        .map_err(|e| e.to_string())?;

    for row in posted_rows {
        let posted_id: i64 = row.try_get("", "id").map_err(|e| e.to_string())?;
        let datetime: DateTime<Utc> = row.try_get("", "datetime").map_err(|e| e.to_string())?;
        let payment_term_id: i64 = row
            .try_get("", "payment_term_id")
            .map_err(|e| e.to_string())?;
        migrate_posted_row(conn, posted_id, datetime, payment_term_id).await?;
    }

    let cancelled_rows = conn
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT id, datetime, payment_term_id FROM cancelled_invoices WHERE payment_term_id IS NOT NULL AND payment_term_id > 0".to_string(),
        ))
        .await
        .map_err(|e| e.to_string())?;

    for row in cancelled_rows {
        let cancelled_id: i64 = row.try_get("", "id").map_err(|e| e.to_string())?;
        let datetime: DateTime<Utc> = row.try_get("", "datetime").map_err(|e| e.to_string())?;
        let payment_term_id: i64 = row
            .try_get("", "payment_term_id")
            .map_err(|e| e.to_string())?;
        migrate_cancelled_row(conn, cancelled_id, datetime, payment_term_id).await?;
    }

    Ok(())
}

async fn migrate_draft_legacy<C: ConnectionTrait>(
    conn: &C,
    draft_id: i64,
    payment_term_id: i64,
) -> Result<(), String> {
    let Some(pt) = load_legacy_payment_term(conn, payment_term_id).await? else {
        return Ok(());
    };

    let (date_kind, due_date, due_duration) = legacy_date_fields(conn, &pt).await?;

    let now = Utc::now();
    let term_id = insert_legacy_draft_payment_term(conn, draft_id, now).await?;

    draft_payment_term_line::ActiveModel {
        draft_payment_term_id: Set(term_id),
        line_order: Set(0),
        date_kind: Set(date_kind),
        due_date: Set(due_date),
        due_duration: Set(due_duration),
        amount_kind: Set(PaymentTermAmountKind::Relative),
        amount: Set(None),
        amount_percentage: Set(Some(Decimal::from(100))),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(conn)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// m00017 still has `draft_payment_terms.draft_invoice_id`; write it with SQL.
async fn insert_legacy_draft_payment_term<C: ConnectionTrait>(
    conn: &C,
    draft_id: i64,
    now: DateTime<Utc>,
) -> Result<i64, String> {
    let row = conn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO draft_payment_terms (created_at, updated_at, draft_invoice_id) \
             VALUES ($1, $2, $3) RETURNING id",
            [now.into(), now.into(), draft_id.into()],
        ))
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "failed to insert draft payment term".to_string())?;
    row.try_get("", "id").map_err(|e| e.to_string())
}

async fn insert_legacy_posted_payment_term<C: ConnectionTrait>(
    conn: &C,
    posted_id: Option<i64>,
    cancelled_id: Option<i64>,
    now: DateTime<Utc>,
) -> Result<i64, String> {
    let row = conn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO posted_payment_terms \
             (created_at, updated_at, posted_invoice_id, cancelled_invoice_id) \
             VALUES ($1, $2, $3, $4) RETURNING id",
            [
                now.into(),
                now.into(),
                posted_id.into(),
                cancelled_id.into(),
            ],
        ))
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "failed to insert posted payment term".to_string())?;
    row.try_get("", "id").map_err(|e| e.to_string())
}

struct LegacyPaymentTerm {
    term_type: String,
    backing_id: i64,
}

async fn load_legacy_payment_term<C: ConnectionTrait>(
    conn: &C,
    payment_term_id: i64,
) -> Result<Option<LegacyPaymentTerm>, String> {
    let row = conn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT type, backing_id FROM payment_terms WHERE id = $1",
            [payment_term_id.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(LegacyPaymentTerm {
        term_type: row.try_get("", "type").map_err(|e| e.to_string())?,
        backing_id: row.try_get("", "backing_id").map_err(|e| e.to_string())?,
    }))
}

async fn legacy_date_fields<C: ConnectionTrait>(
    conn: &C,
    pt: &LegacyPaymentTerm,
) -> Result<(PaymentTermDateKind, Option<NaiveDate>, Option<i64>), String> {
    match pt.term_type.as_str() {
        PAYMENT_TERM_TYPE_DUE_DATE => {
            let row = conn
                .query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT datetime FROM payment_term_due_dates WHERE id = $1",
                    [pt.backing_id.into()],
                ))
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "due date backing not found".to_string())?;
            let datetime: DateTime<Utc> = row.try_get("", "datetime").map_err(|e| e.to_string())?;
            Ok((
                PaymentTermDateKind::Absolute,
                Some(datetime_to_due_date(datetime)),
                None,
            ))
        }
        PAYMENT_TERM_TYPE_RELATIVE => {
            let row = conn
                .query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT duration FROM payment_term_relatives WHERE id = $1",
                    [pt.backing_id.into()],
                ))
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "relative backing not found".to_string())?;
            let duration: i64 = row.try_get("", "duration").map_err(|e| e.to_string())?;
            Ok((PaymentTermDateKind::Relative, None, Some(duration)))
        }
        other => Err(format!("unknown payment term type: {other}")),
    }
}

async fn migrate_posted_row<C: ConnectionTrait>(
    conn: &C,
    posted_id: i64,
    anchor: DateTime<Utc>,
    payment_term_id: i64,
) -> Result<(), String> {
    let Some(pt) = load_legacy_payment_term(conn, payment_term_id).await? else {
        return Ok(());
    };

    let (date_kind, due_date, due_duration) = legacy_date_fields(conn, &pt).await?;
    let grand_total = compute_posted_receivable_grand_total(conn, posted_id).await?;

    let due = match date_kind {
        PaymentTermDateKind::Absolute => due_date.unwrap_or_else(|| datetime_to_due_date(anchor)),
        PaymentTermDateKind::Relative => resolve_legacy_relative_due(anchor, due_duration),
        PaymentTermDateKind::RelativeDelivery => resolve_legacy_relative_due(anchor, due_duration),
    };

    let now = Utc::now();
    let term_id = insert_legacy_posted_payment_term(conn, Some(posted_id), None, now).await?;

    posted_payment_term_line::ActiveModel {
        posted_payment_term_id: Set(term_id),
        line_order: Set(0),
        due_date: Set(due),
        amount: Set(decimal::normalize(grand_total)),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(conn)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

async fn migrate_cancelled_row<C: ConnectionTrait>(
    conn: &C,
    cancelled_id: i64,
    anchor: DateTime<Utc>,
    payment_term_id: i64,
) -> Result<(), String> {
    let Some(pt) = load_legacy_payment_term(conn, payment_term_id).await? else {
        return Ok(());
    };

    let (date_kind, due_date, due_duration) = legacy_date_fields(conn, &pt).await?;
    let grand_total = compute_cancelled_receivable_grand_total(conn, cancelled_id).await?;

    let due = match date_kind {
        PaymentTermDateKind::Absolute => due_date.unwrap_or_else(|| datetime_to_due_date(anchor)),
        PaymentTermDateKind::Relative => resolve_legacy_relative_due(anchor, due_duration),
        PaymentTermDateKind::RelativeDelivery => resolve_legacy_relative_due(anchor, due_duration),
    };

    let now = Utc::now();
    let term_id = insert_legacy_posted_payment_term(conn, None, Some(cancelled_id), now).await?;

    posted_payment_term_line::ActiveModel {
        posted_payment_term_id: Set(term_id),
        line_order: Set(0),
        due_date: Set(due),
        amount: Set(decimal::normalize(grand_total)),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(conn)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

async fn load_taxes_by_ids_conn<C: ConnectionTrait>(
    conn: &C,
    ids: &[i64],
) -> Result<Vec<tax::Model>, String> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    TaxEntity::find()
        .filter(tax::Column::Id.is_in(ids.to_vec()))
        .all(conn)
        .await
        .map_err(|e| e.to_string())
}

async fn compute_posted_receivable_grand_total<C: ConnectionTrait>(
    conn: &C,
    posted_id: i64,
) -> Result<Decimal, String> {
    let header_tax_ids = load_posted_invoice_tax_ids(conn, posted_id)
        .await
        .map_err(|e| e.to_string())?;
    let header_taxes = load_taxes_by_ids_conn(conn, &header_tax_ids).await?;

    let lines = PostedInvoiceLineEntity::find()
        .filter(posted_invoice_line::Column::PostedInvoiceId.eq(posted_id))
        .all(conn)
        .await
        .map_err(|e| e.to_string())?;

    let mut totals = InvoiceLinesTotals::default();
    let mut line_tax_ids = HashSet::new();
    for line in &lines {
        let line_tax_ids_vec = load_posted_line_tax_ids(conn, line.id)
            .await
            .map_err(|e| e.to_string())?;
        let line_taxes = load_taxes_by_ids_conn(conn, &line_tax_ids_vec).await?;
        merge_invoice_line_tax_ids(&mut line_tax_ids, &line_taxes);
        let (untaxed, levied, withholding, _) =
            invoice_line_amount_breakdown(line.quantity, line.rate, &line_taxes);
        totals.untaxed_subtotal = decimal::dec_sum(totals.untaxed_subtotal, untaxed);
        totals.lines_levied = decimal::dec_sum(totals.lines_levied, levied);
        totals.lines_withholding = decimal::dec_sum(totals.lines_withholding, withholding);
    }
    Ok(invoice_receivable_grand_total(
        &totals,
        &header_taxes,
        &line_tax_ids,
    ))
}

async fn compute_cancelled_receivable_grand_total<C: ConnectionTrait>(
    conn: &C,
    cancelled_id: i64,
) -> Result<Decimal, String> {
    let header_tax_ids = load_cancelled_invoice_tax_ids(conn, cancelled_id)
        .await
        .map_err(|e| e.to_string())?;
    let header_taxes = load_taxes_by_ids_conn(conn, &header_tax_ids).await?;

    let rows = conn
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, quantity, rate FROM cancelled_invoice_lines \
             WHERE cancelled_invoice_id = $1 ORDER BY id ASC",
            [cancelled_id.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;

    let mut totals = InvoiceLinesTotals::default();
    let mut line_tax_ids = HashSet::new();
    for row in rows {
        let line_id: i64 = row.try_get("", "id").map_err(|e| e.to_string())?;
        let quantity: Decimal = row.try_get("", "quantity").map_err(|e| e.to_string())?;
        let rate: Decimal = row.try_get("", "rate").map_err(|e| e.to_string())?;
        let line_tax_ids_vec = load_cancelled_line_tax_ids(conn, line_id)
            .await
            .map_err(|e| e.to_string())?;
        let line_taxes = load_taxes_by_ids_conn(conn, &line_tax_ids_vec).await?;
        merge_invoice_line_tax_ids(&mut line_tax_ids, &line_taxes);
        let (untaxed, levied, withholding, _) =
            invoice_line_amount_breakdown(quantity, rate, &line_taxes);
        totals.untaxed_subtotal = decimal::dec_sum(totals.untaxed_subtotal, untaxed);
        totals.lines_levied = decimal::dec_sum(totals.lines_levied, levied);
        totals.lines_withholding = decimal::dec_sum(totals.lines_withholding, withholding);
    }
    Ok(invoice_receivable_grand_total(
        &totals,
        &header_taxes,
        &line_tax_ids,
    ))
}
