//! Draft and posted payment term CRUD, validation, and lifecycle conversion.

use std::collections::HashSet;

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Statement,
};
use serde::Deserialize;

use crate::plugins::finance_common::decimal::{self, parse_decimal};
use crate::plugins::finance_taxes::entities::tax::{self, Entity as TaxEntity, Model as TaxModel};

use crate::plugins::finance_invoices::entities::{
    DraftPaymentTermEntity, DraftPaymentTermLineEntity, PostedInvoiceLineEntity,
    PostedPaymentTermEntity, PostedPaymentTermLineEntity,
    draft_payment_term_line::{
        AMOUNT_KIND_ABSOLUTE, AMOUNT_KIND_RELATIVE, DATE_KIND_ABSOLUTE, DATE_KIND_RELATIVE,
    },
};
use crate::plugins::finance_invoices::entities::{
    draft_payment_term, draft_payment_term_line, posted_invoice_line, posted_payment_term,
    posted_payment_term_line,
};
use crate::plugins::finance_invoices::logic::tax_assoc::{
    load_cancelled_invoice_tax_ids, load_cancelled_line_tax_ids, load_posted_invoice_tax_ids,
    load_posted_line_tax_ids,
};
use crate::plugins::finance_invoices::logic::tax_calculations::{
    InvoiceLinesTotals, invoice_line_amount_breakdown, invoice_receivable_grand_total,
    merge_invoice_line_tax_ids,
};

const PAYMENT_TERM_TYPE_DUE_DATE: &str = "p_finance_invoices.PaymentTermDueDate";
const PAYMENT_TERM_TYPE_RELATIVE: &str = "p_finance_invoices.PaymentTermRelative";

const PERCENTAGE_TOLERANCE: Decimal = Decimal::ONE; // ±1% for draft save validation

#[derive(Debug, Deserialize, Clone)]
pub struct DraftPaymentTermLineInput {
    pub date_kind: String,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub due_duration: Option<String>,
    pub amount_kind: String,
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

pub fn parse_due_date_for_term(s: &str, tz: &str) -> Result<DateTime<Utc>, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("due date is required for absolute date".to_string());
    }
    let date =
        NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| "invalid due date".to_string())?;
    let naive = date
        .and_hms_opt(23, 59, 59)
        .ok_or_else(|| "invalid due date".to_string())?;
    crate::datetime::parse_timezone(tz)
        .from_local_datetime(&naive)
        .single()
        .map(|dt| dt.with_timezone(&Utc))
        .ok_or_else(|| "invalid due date".to_string())
}

fn validate_line_input(line: &DraftPaymentTermLineInput, tz: &str) -> Result<(), String> {
    match line.date_kind.as_str() {
        DATE_KIND_ABSOLUTE => {
            parse_due_date_for_term(line.due_date.as_deref().unwrap_or(""), tz)?;
        }
        DATE_KIND_RELATIVE => {
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
        other => return Err(format!("invalid date kind: {other}")),
    }

    match line.amount_kind.as_str() {
        AMOUNT_KIND_ABSOLUTE => {
            let amt = parse_decimal(line.amount.as_deref().unwrap_or(""))
                .ok_or_else(|| "amount is required for absolute amount".to_string())?;
            if amt <= Decimal::ZERO {
                return Err("amount must be positive".to_string());
            }
        }
        AMOUNT_KIND_RELATIVE => {
            let pct = parse_decimal(line.amount_percentage.as_deref().unwrap_or(""))
                .ok_or_else(|| "percentage is required for relative amount".to_string())?;
            if pct <= Decimal::ZERO {
                return Err("percentage must be positive".to_string());
            }
        }
        other => return Err(format!("invalid amount kind: {other}")),
    }
    Ok(())
}

pub fn validate_draft_payment_term_lines(
    lines: &[DraftPaymentTermLineInput],
    tz: &str,
) -> Result<(), String> {
    if lines.is_empty() {
        return Err("add at least one payment term line".to_string());
    }
    for line in lines {
        validate_line_input(line, tz)?;
    }

    let all_relative = lines.iter().all(|l| l.amount_kind == AMOUNT_KIND_RELATIVE);
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
    tz: &str,
    now: DateTime<Utc>,
) -> Result<draft_payment_term_line::ActiveModel, String> {
    let (due_datetime, due_duration) = match line.date_kind.as_str() {
        DATE_KIND_ABSOLUTE => (
            Some(parse_due_date_for_term(
                line.due_date.as_deref().unwrap_or(""),
                tz,
            )?),
            None,
        ),
        DATE_KIND_RELATIVE => {
            let nanos = crate::duration::parse_duration(line.due_duration.as_deref().unwrap_or(""))
                .map_err(|e| e.to_string())?;
            (None, Some(nanos))
        }
        _ => unreachable!(),
    };

    let (amount, amount_percentage) = match line.amount_kind.as_str() {
        AMOUNT_KIND_ABSOLUTE => (
            Some(decimal::normalize(
                parse_decimal(line.amount.as_deref().unwrap_or("")).unwrap(),
            )),
            None,
        ),
        AMOUNT_KIND_RELATIVE => (
            None,
            Some(decimal::normalize(
                parse_decimal(line.amount_percentage.as_deref().unwrap_or("")).unwrap(),
            )),
        ),
        _ => unreachable!(),
    };

    Ok(draft_payment_term_line::ActiveModel {
        draft_payment_term_id: Set(draft_payment_term_id),
        line_order: Set(line_order),
        date_kind: Set(line.date_kind.clone()),
        due_datetime: Set(due_datetime),
        due_duration: Set(due_duration),
        amount_kind: Set(line.amount_kind.clone()),
        amount: Set(amount),
        amount_percentage: Set(amount_percentage),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    })
}

pub async fn upsert_draft_payment_term<C: ConnectionTrait>(
    conn: &C,
    draft_id: i64,
    lines: &[DraftPaymentTermLineInput],
    tz: &str,
) -> Result<draft_payment_term::Model, String> {
    validate_draft_payment_term_lines(lines, tz)?;
    let now = Utc::now();

    let term = if let Some(existing) = DraftPaymentTermEntity::find()
        .filter(draft_payment_term::Column::DraftInvoiceId.eq(draft_id))
        .one(conn)
        .await
        .map_err(|e| e.to_string())?
    {
        existing
    } else {
        draft_payment_term::ActiveModel {
            draft_invoice_id: Set(draft_id),
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
        let am = line_input_to_active(term.id, i as i32, line, tz, now)?;
        am.insert(conn).await.map_err(|e| e.to_string())?;
    }

    Ok(term)
}

pub async fn load_draft_payment_term_lines(
    db: &DatabaseConnection,
    draft_id: i64,
) -> Result<Vec<draft_payment_term_line::Model>, String> {
    let Some(term) = DraftPaymentTermEntity::find()
        .filter(draft_payment_term::Column::DraftInvoiceId.eq(draft_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(Vec::new());
    };
    DraftPaymentTermLineEntity::find()
        .filter(draft_payment_term_line::Column::DraftPaymentTermId.eq(term.id))
        .order_by_asc(draft_payment_term_line::Column::LineOrder)
        .order_by_asc(draft_payment_term_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())
}

pub fn format_due_date_input(dt: DateTime<Utc>, tz: &str) -> String {
    dt.with_timezone(&crate::datetime::parse_timezone(tz))
        .format("%Y-%m-%d")
        .to_string()
}

pub async fn payment_term_lines_form_json(
    db: &DatabaseConnection,
    draft_id: i64,
    tz: &str,
) -> String {
    let lines = load_draft_payment_term_lines(db, draft_id)
        .await
        .unwrap_or_default();
    if lines.is_empty() {
        return default_payment_term_lines_json();
    }
    let out: Vec<serde_json::Value> = lines
        .iter()
        .map(|l| {
            let due_date = l
                .due_datetime
                .map(|dt| format_due_date_input(dt, tz))
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

pub fn resolve_due_datetime(
    line: &draft_payment_term_line::Model,
    anchor: DateTime<Utc>,
) -> Result<DateTime<Utc>, String> {
    match line.date_kind.as_str() {
        DATE_KIND_ABSOLUTE => line
            .due_datetime
            .ok_or_else(|| "missing absolute due datetime".to_string()),
        DATE_KIND_RELATIVE => {
            let dur = line
                .due_duration
                .ok_or_else(|| "missing relative duration".to_string())?;
            anchor
                .checked_add_signed(Duration::nanoseconds(dur))
                .ok_or_else(|| "due date overflow".to_string())
        }
        other => Err(format!("invalid date kind: {other}")),
    }
}

fn resolve_amount(
    line: &draft_payment_term_line::Model,
    grand_total: Decimal,
) -> Result<Decimal, String> {
    match line.amount_kind.as_str() {
        AMOUNT_KIND_ABSOLUTE => line
            .amount
            .ok_or_else(|| "missing absolute amount".to_string()),
        AMOUNT_KIND_RELATIVE => {
            let pct = line
                .amount_percentage
                .ok_or_else(|| "missing relative percentage".to_string())?;
            Ok(decimal::dec_mul(grand_total, pct / Decimal::from(100)))
        }
        other => Err(format!("invalid amount kind: {other}")),
    }
}

pub fn validate_posting_amounts(
    lines: &[draft_payment_term_line::Model],
    grand_total: Decimal,
) -> Result<(), String> {
    if lines.is_empty() {
        return Err("payment term must have at least one line".to_string());
    }
    let all_absolute = lines.iter().all(|l| l.amount_kind == AMOUNT_KIND_ABSOLUTE);
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
    posted_invoice_id: i64,
    anchor: DateTime<Utc>,
    grand_total: Decimal,
) -> Result<posted_payment_term::Model, String> {
    let draft_lines = load_draft_payment_term_lines_conn(conn, draft_id).await?;
    validate_posting_amounts(&draft_lines, grand_total)?;

    let now = Utc::now();
    let mut resolved: Vec<(DateTime<Utc>, Decimal)> = Vec::with_capacity(draft_lines.len());
    for line in &draft_lines {
        resolved.push((
            resolve_due_datetime(line, anchor)?,
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
        posted_invoice_id: Set(Some(posted_invoice_id)),
        cancelled_invoice_id: Set(None),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(conn)
    .await
    .map_err(|e| e.to_string())?;

    for (i, (due_datetime, amount)) in resolved.into_iter().enumerate() {
        posted_payment_term_line::ActiveModel {
            posted_payment_term_id: Set(term.id),
            line_order: Set(i as i32),
            due_datetime: Set(due_datetime),
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
    let Some(term) = DraftPaymentTermEntity::find()
        .filter(draft_payment_term::Column::DraftInvoiceId.eq(draft_id))
        .one(conn)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Err("draft payment term not found".to_string());
    };
    DraftPaymentTermLineEntity::find()
        .filter(draft_payment_term_line::Column::DraftPaymentTermId.eq(term.id))
        .order_by_asc(draft_payment_term_line::Column::LineOrder)
        .order_by_asc(draft_payment_term_line::Column::Id)
        .all(conn)
        .await
        .map_err(|e| e.to_string())
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
    let Some(term) = PostedPaymentTermEntity::find()
        .filter(posted_payment_term::Column::PostedInvoiceId.eq(posted_invoice_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    let lines = PostedPaymentTermLineEntity::find()
        .filter(posted_payment_term_line::Column::PostedPaymentTermId.eq(term.id))
        .order_by_asc(posted_payment_term_line::Column::LineOrder)
        .order_by_asc(posted_payment_term_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Some((term, lines)))
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
    let Some(term) = PostedPaymentTermEntity::find()
        .filter(posted_payment_term::Column::CancelledInvoiceId.eq(cancelled_invoice_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    let lines = PostedPaymentTermLineEntity::find()
        .filter(posted_payment_term_line::Column::PostedPaymentTermId.eq(term.id))
        .order_by_asc(posted_payment_term_line::Column::LineOrder)
        .order_by_asc(posted_payment_term_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Some((term, lines)))
}

pub async fn copy_posted_payment_term_to_cancelled<C: ConnectionTrait>(
    conn: &C,
    posted_invoice_id: i64,
    cancelled_invoice_id: i64,
) -> Result<(), String> {
    let (term, lines) = {
        let term = PostedPaymentTermEntity::find()
            .filter(posted_payment_term::Column::PostedInvoiceId.eq(posted_invoice_id))
            .one(conn)
            .await
            .map_err(|e| e.to_string())?;
        match term {
            None => return Ok(()),
            Some(term) => {
                let lines = PostedPaymentTermLineEntity::find()
                    .filter(posted_payment_term_line::Column::PostedPaymentTermId.eq(term.id))
                    .order_by_asc(posted_payment_term_line::Column::LineOrder)
                    .all(conn)
                    .await
                    .map_err(|e| e.to_string())?;
                (term, lines)
            }
        }
    };

    let now = Utc::now();
    let new_term = posted_payment_term::ActiveModel {
        posted_invoice_id: Set(None),
        cancelled_invoice_id: Set(Some(cancelled_invoice_id)),
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
            due_datetime: Set(line.due_datetime),
            amount: Set(line.amount),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(conn)
        .await
        .map_err(|e| e.to_string())?;
    }
    let _ = term;
    Ok(())
}

pub async fn posted_payment_term_to_draft<C: ConnectionTrait>(
    conn: &C,
    cancelled_invoice_id: i64,
    draft_id: i64,
    tz: &str,
) -> Result<(), String> {
    let (_, lines) = {
        let term = PostedPaymentTermEntity::find()
            .filter(posted_payment_term::Column::CancelledInvoiceId.eq(cancelled_invoice_id))
            .one(conn)
            .await
            .map_err(|e| e.to_string())?;
        match term {
            None => return Ok(()),
            Some(term) => {
                let lines = PostedPaymentTermLineEntity::find()
                    .filter(posted_payment_term_line::Column::PostedPaymentTermId.eq(term.id))
                    .order_by_asc(posted_payment_term_line::Column::LineOrder)
                    .all(conn)
                    .await
                    .map_err(|e| e.to_string())?;
                (term, lines)
            }
        }
    };

    if lines.is_empty() {
        return Ok(());
    }

    let inputs: Vec<DraftPaymentTermLineInput> = lines
        .iter()
        .map(|l| DraftPaymentTermLineInput {
            date_kind: DATE_KIND_ABSOLUTE.to_string(),
            due_date: Some(format_due_date_input(l.due_datetime, tz)),
            due_duration: None,
            amount_kind: AMOUNT_KIND_ABSOLUTE.to_string(),
            amount: Some(decimal::decimal_display(l.amount)),
            amount_percentage: None,
        })
        .collect();

    upsert_draft_payment_term(conn, draft_id, &inputs, tz).await?;
    Ok(())
}

pub fn draft_payment_term_line_display(
    line: &draft_payment_term_line::Model,
    tz: &str,
) -> PaymentTermLineDisplayRow {
    let due_display = match line.date_kind.as_str() {
        DATE_KIND_ABSOLUTE => line
            .due_datetime
            .map(|dt| crate::datetime::DatetimeLabel::short(dt, tz).into_string())
            .unwrap_or_else(|| "—".to_string()),
        DATE_KIND_RELATIVE => line
            .due_duration
            .map(crate::duration::format_duration)
            .unwrap_or_else(|| "—".to_string()),
        _ => "—".to_string(),
    };
    let amount_display = match line.amount_kind.as_str() {
        AMOUNT_KIND_ABSOLUTE => line
            .amount
            .map(decimal::decimal_display)
            .unwrap_or_else(|| "—".to_string()),
        AMOUNT_KIND_RELATIVE => line
            .amount_percentage
            .map(|p| format!("{}%", decimal::decimal_display(p)))
            .unwrap_or_else(|| "—".to_string()),
        _ => "—".to_string(),
    };
    PaymentTermLineDisplayRow {
        due_display,
        amount_display,
    }
}

pub fn posted_payment_term_line_display(
    line: &posted_payment_term_line::Model,
    tz: &str,
    minor_unit: i32,
    symbol: &str,
) -> PaymentTermLineDisplayRow {
    PaymentTermLineDisplayRow {
        due_display: crate::datetime::DatetimeLabel::short(line.due_datetime, tz).into_string(),
        amount_display: decimal::decimal_display_currency(line.amount, minor_unit, symbol),
    }
}

pub async fn draft_payment_term_display_rows(
    db: &DatabaseConnection,
    draft_id: i64,
    tz: &str,
) -> Vec<PaymentTermLineDisplayRow> {
    load_draft_payment_term_lines(db, draft_id)
        .await
        .unwrap_or_default()
        .iter()
        .map(|l| draft_payment_term_line_display(l, tz))
        .collect()
}

pub async fn posted_payment_term_display_rows(
    db: &DatabaseConnection,
    posted_invoice_id: i64,
    tz: &str,
    minor_unit: i32,
    symbol: &str,
) -> Vec<PaymentTermLineDisplayRow> {
    match load_posted_payment_term_for_posted(db, posted_invoice_id).await {
        Ok(Some((_, lines))) => lines
            .iter()
            .map(|l| posted_payment_term_line_display(l, tz, minor_unit, symbol))
            .collect(),
        _ => Vec::new(),
    }
}

pub async fn cancelled_payment_term_display_rows(
    db: &DatabaseConnection,
    cancelled_invoice_id: i64,
    tz: &str,
    minor_unit: i32,
    symbol: &str,
) -> Vec<PaymentTermLineDisplayRow> {
    match load_posted_payment_term_for_cancelled(db, cancelled_invoice_id).await {
        Ok(Some((_, lines))) => lines
            .iter()
            .map(|l| posted_payment_term_line_display(l, tz, minor_unit, symbol))
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

    let (date_kind, due_datetime, due_duration) = legacy_date_fields(conn, &pt).await?;

    let term = draft_payment_term::ActiveModel {
        draft_invoice_id: Set(draft_id),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(conn)
    .await
    .map_err(|e| e.to_string())?;

    draft_payment_term_line::ActiveModel {
        draft_payment_term_id: Set(term.id),
        line_order: Set(0),
        date_kind: Set(date_kind),
        due_datetime: Set(due_datetime),
        due_duration: Set(due_duration),
        amount_kind: Set(AMOUNT_KIND_RELATIVE.to_string()),
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
) -> Result<(String, Option<DateTime<Utc>>, Option<i64>), String> {
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
            Ok((DATE_KIND_ABSOLUTE.to_string(), Some(datetime), None))
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
            Ok((DATE_KIND_RELATIVE.to_string(), None, Some(duration)))
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

    let (date_kind, due_datetime, due_duration) = legacy_date_fields(conn, &pt).await?;
    let grand_total = compute_posted_receivable_grand_total(conn, posted_id).await?;

    let due = match date_kind.as_str() {
        DATE_KIND_ABSOLUTE => due_datetime.unwrap_or(anchor),
        DATE_KIND_RELATIVE => anchor
            .checked_add_signed(Duration::nanoseconds(due_duration.unwrap_or(0)))
            .unwrap_or(anchor),
        _ => anchor,
    };

    let now = Utc::now();
    let term = posted_payment_term::ActiveModel {
        posted_invoice_id: Set(Some(posted_id)),
        cancelled_invoice_id: Set(None),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(conn)
    .await
    .map_err(|e| e.to_string())?;

    posted_payment_term_line::ActiveModel {
        posted_payment_term_id: Set(term.id),
        line_order: Set(0),
        due_datetime: Set(due),
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

    let (date_kind, due_datetime, due_duration) = legacy_date_fields(conn, &pt).await?;
    let grand_total = compute_cancelled_receivable_grand_total(conn, cancelled_id).await?;

    let due = match date_kind.as_str() {
        DATE_KIND_ABSOLUTE => due_datetime.unwrap_or(anchor),
        DATE_KIND_RELATIVE => anchor
            .checked_add_signed(Duration::nanoseconds(due_duration.unwrap_or(0)))
            .unwrap_or(anchor),
        _ => anchor,
    };

    let now = Utc::now();
    let term = posted_payment_term::ActiveModel {
        posted_invoice_id: Set(None),
        cancelled_invoice_id: Set(Some(cancelled_id)),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(conn)
    .await
    .map_err(|e| e.to_string())?;

    posted_payment_term_line::ActiveModel {
        posted_payment_term_id: Set(term.id),
        line_order: Set(0),
        due_datetime: Set(due),
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
