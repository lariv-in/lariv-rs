//! Invoice settlement against a customer payment.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Statement, TransactionTrait,
};

use lariv_formula::decimal;
use lariv_plugin_finance_accounts::logic::journal::JournalLineSpec;
use lariv_plugin_finance_payments::logic::{
    CreatePaymentInput, build_payment_lines, insert_payment, prepare_payment,
};
use lariv_plugin_finance_taxes::entities::tax;
use lariv_plugin_finance_taxes::scope::load_taxes_by_ids;

use crate::entities::{
    cancelled_invoice, paid_invoice, partially_paid_invoice, posted_invoice, posted_invoice_line,
};
use crate::logic::tax_assoc::load_posted_invoice_tax_ids;
use crate::logic::tax_calculations::{
    InvoiceLinesTotals, invoice_line_amounts, invoice_receivable_grand_total,
    merge_invoice_line_tax_ids, payment_withholding_base, validate_payment_taxes,
};

pub struct PayPostedInvoiceInput {
    pub posted_invoice_id: i64,
    pub amount: Decimal,
    pub datetime: DateTime<Utc>,
    pub withholding_tax_ids: Vec<i64>,
    /// `None` evaluates the payment template. `Some` writes these lines.
    pub journal_lines: Option<Vec<JournalLineSpec>>,
    /// Trimmed payment number. Blank or `None` assigns one from Payment number format.
    pub number: Option<String>,
}

pub struct PayPostedInvoiceResult {
    pub payment_id: i64,
    pub is_full: bool,
    pub settlement_id: i64,
}

async fn sum_posted_invoice_payments<C: ConnectionTrait>(
    db: &C,
    posted_invoice_id: i64,
) -> Result<Decimal, String> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            r#"SELECT COALESCE(SUM(p.amount), 0) AS s
               FROM payments p
               WHERE p.id IN (
                   SELECT payment_id FROM paid_invoices WHERE posted_invoice_id = $1
                   UNION
                   SELECT payment_id FROM partially_paid_invoices WHERE posted_invoice_id = $1
               )"#,
            [posted_invoice_id.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    let s: Decimal = row
        .and_then(|r| r.try_get("", "s").ok())
        .unwrap_or(Decimal::ZERO);
    Ok(decimal::normalize(s))
}

struct PostedInvoiceAmounts {
    untaxed_subtotal: Decimal,
    receivable_total: Decimal,
}

async fn posted_invoice_amounts(
    db: &DatabaseConnection,
    posted_id: i64,
) -> Result<PostedInvoiceAmounts, String> {
    let posted = posted_invoice::Entity::find_by_id(posted_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "posted invoice not found".to_string())?;

    let lines = posted_invoice_line::Entity::find()
        .filter(posted_invoice_line::Column::PostedInvoiceId.eq(posted_id))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    let mut totals = InvoiceLinesTotals::default();
    let mut line_tax_ids = HashSet::new();
    for line in &lines {
        let line_tax_ids_vec = crate::logic::tax_assoc::load_posted_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        let line_taxes = load_taxes_by_ids(db, &line_tax_ids_vec)
            .await
            .map_err(|e| e.to_string())?;
        merge_invoice_line_tax_ids(&mut line_tax_ids, &line_taxes);
        let (untaxed, levied, withholding, _) =
            invoice_line_amounts(line.pre_tax_amount, &line_taxes);
        totals.untaxed_subtotal = decimal::dec_sum(totals.untaxed_subtotal, untaxed);
        totals.lines_levied = decimal::dec_sum(totals.lines_levied, levied);
        totals.lines_withholding = decimal::dec_sum(totals.lines_withholding, withholding);
    }
    let header_tax_ids = load_posted_invoice_tax_ids(db, posted.id)
        .await
        .map_err(|e| e.to_string())?;
    let header_taxes = load_taxes_by_ids(db, &header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;
    let receivable_total = invoice_receivable_grand_total(&totals, &header_taxes, &line_tax_ids);
    Ok(PostedInvoiceAmounts {
        untaxed_subtotal: totals.untaxed_subtotal,
        receivable_total,
    })
}

async fn posted_invoice_receivable_total(
    db: &DatabaseConnection,
    posted_id: i64,
) -> Result<Decimal, String> {
    Ok(posted_invoice_amounts(db, posted_id)
        .await?
        .receivable_total)
}

/// Remaining receivable after settlements on a posted invoice.
pub async fn posted_invoice_open_balance(
    db: &DatabaseConnection,
    posted_id: i64,
) -> Result<Decimal, String> {
    let inv_total = posted_invoice_receivable_total(db, posted_id).await?;
    let applied_sum = sum_posted_invoice_payments(db, posted_id).await?;
    Ok(decimal::dec_sub(inv_total, applied_sum))
}

/// Whether a posted invoice can still receive a payment (not cancelled or fully paid).
pub async fn posted_invoice_can_accept_payment(db: &DatabaseConnection, posted_id: i64) -> bool {
    let cancelled_count = cancelled_invoice::Entity::find()
        .filter(cancelled_invoice::Column::PostedInvoiceId.eq(posted_id))
        .count(db)
        .await
        .unwrap_or(0);
    if cancelled_count > 0 {
        return false;
    }
    let paid_count = paid_invoice::Entity::find()
        .filter(paid_invoice::Column::PostedInvoiceId.eq(posted_id))
        .count(db)
        .await
        .unwrap_or(0);
    if paid_count > 0 {
        return false;
    }
    posted_invoice_open_balance(db, posted_id)
        .await
        .map(|open| open > Decimal::ZERO)
        .unwrap_or(false)
}

/// Validate a payment allocation against invoice state and open balance.
pub async fn validate_payment_allocation(
    db: &DatabaseConnection,
    posted_id: i64,
    amount: Decimal,
) -> Result<(posted_invoice::Model, Decimal, Decimal, bool), String> {
    if posted_id == 0 {
        return Err("posted invoice is required".to_string());
    }
    if amount <= Decimal::ZERO {
        return Err("amount must be positive".to_string());
    }

    let posted = posted_invoice::Entity::find_by_id(posted_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "posted invoice not found".to_string())?;

    let cancelled_count = cancelled_invoice::Entity::find()
        .filter(cancelled_invoice::Column::PostedInvoiceId.eq(posted.id))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if cancelled_count > 0 {
        return Err("cannot pay a cancelled invoice".to_string());
    }

    let paid_count = paid_invoice::Entity::find()
        .filter(paid_invoice::Column::PostedInvoiceId.eq(posted.id))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if paid_count > 0 {
        return Err("invoice is already fully paid".to_string());
    }

    let amounts = posted_invoice_amounts(db, posted.id).await?;
    let inv_total = amounts.receivable_total;
    let applied_sum = sum_posted_invoice_payments(db, posted.id).await?;
    let total_after = decimal::dec_sum(applied_sum, amount);
    if decimal::dec_cmp(total_after, inv_total) == std::cmp::Ordering::Greater {
        return Err("payment exceeds open balance".to_string());
    }
    let is_full = decimal::dec_cmp(total_after, inv_total) == std::cmp::Ordering::Equal;
    Ok((posted, inv_total, amounts.untaxed_subtotal, is_full))
}

/// Insert paid or partially-paid settlement row for a payment.
pub async fn record_payment_settlement<C: ConnectionTrait>(
    db: &C,
    pay_id: i64,
    posted_id: i64,
    is_full: bool,
) -> Result<i64, String> {
    let prior = partially_paid_invoice::Entity::find()
        .filter(partially_paid_invoice::Column::PostedInvoiceId.eq(posted_id))
        .order_by_desc(partially_paid_invoice::Column::Id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    let prior_id = prior.map(|p| p.id);
    let now = Utc::now();

    if is_full {
        let row = paid_invoice::ActiveModel {
            payment_id: Set(pay_id),
            posted_invoice_id: Set(posted_id),
            prior_partially_paid_invoice_id: Set(prior_id),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.id)
    } else {
        let row = partially_paid_invoice::ActiveModel {
            payment_id: Set(pay_id),
            posted_invoice_id: Set(posted_id),
            prior_partially_paid_invoice_id: Set(prior_id),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.id)
    }
}

async fn invoice_payment_lines(
    db: &DatabaseConnection,
    settlement: Decimal,
    receivable_total: Decimal,
    untaxed_subtotal: Decimal,
    taxes: &[tax::Model],
) -> Result<Vec<JournalLineSpec>, String> {
    let withholding_base = payment_withholding_base(settlement, receivable_total, untaxed_subtotal);
    build_payment_lines(db, settlement, withholding_base, taxes).await
}

/// Journal-line JSON for paying one posted invoice.
pub async fn suggest_invoice_payment_lines(
    db: &DatabaseConnection,
    posted_invoice_id: i64,
    amount: Decimal,
    withholding_tax_ids: &[i64],
) -> Result<String, String> {
    let posted = posted_invoice::Entity::find_by_id(posted_invoice_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("posted invoice {posted_invoice_id} not found"))?;
    let amounts = posted_invoice_amounts(db, posted.id).await?;
    let taxes = load_taxes_by_ids(db, withholding_tax_ids)
        .await
        .map_err(|e| e.to_string())?;
    validate_payment_taxes(&taxes)?;
    let withholding_base =
        payment_withholding_base(amount, amounts.receivable_total, amounts.untaxed_subtotal);
    lariv_plugin_finance_payments::logic::suggest_payment_lines_json(
        db,
        amount,
        withholding_base,
        withholding_tax_ids,
    )
    .await
}

/// Record a customer payment for this invoice and the settlement row that links them.
pub async fn pay_posted_invoice(
    db: &DatabaseConnection,
    input: PayPostedInvoiceInput,
) -> Result<PayPostedInvoiceResult, String> {
    let (posted, inv_total, untaxed_subtotal, is_full) =
        validate_payment_allocation(db, input.posted_invoice_id, input.amount).await?;
    let taxes = load_taxes_by_ids(db, &input.withholding_tax_ids)
        .await
        .map_err(|e| e.to_string())?;
    validate_payment_taxes(&taxes)?;
    let settlement = decimal::normalize(input.amount);
    let lines = if let Some(lines) = input.journal_lines {
        lines
    } else {
        invoice_payment_lines(db, settlement, inv_total, untaxed_subtotal, &taxes).await?
    };
    let withholding_base = payment_withholding_base(settlement, inv_total, untaxed_subtotal);

    let prepared = prepare_payment(
        db,
        CreatePaymentInput {
            customer_id: posted.customer_id,
            amount: settlement,
            datetime: input.datetime,
            withholding_tax_ids: input.withholding_tax_ids,
            journal_lines: Some(lines),
            withholding_base: Some(withholding_base),
            number: input.number,
        },
    )
    .await?;
    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let payment = insert_payment(&txn, prepared).await?;
    let settlement_id = record_payment_settlement(&txn, payment.id, posted.id, is_full).await?;
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(PayPostedInvoiceResult {
        payment_id: payment.id,
        is_full,
        settlement_id,
    })
}
