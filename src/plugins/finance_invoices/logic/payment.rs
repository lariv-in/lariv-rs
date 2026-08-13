//! Payment create (models_payment.go BeforeCreate/AfterCreate).

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Statement, TransactionTrait,
};

use crate::plugins::finance_accounts::{
    logic::journal::debit_balance_type,
    logic::journal::{
        JournalLineSpec, create_source_doc, insert_journal_entry, update_source_doc_id,
    },
    validate_leaf_account_balance_type,
};
use crate::plugins::finance_common::decimal::{self, parse_decimal};
use crate::plugins::finance_products::preferences::optional_i64;
use crate::plugins::finance_taxes::entities::tax;
use crate::plugins::finance_taxes::scope::load_taxes_by_ids;

use crate::plugins::finance_invoices::entities::payment::PAYMENT_SOURCE_DOC_TYPE;
use crate::plugins::finance_invoices::entities::{
    cancelled_invoice, paid_invoice, partially_paid_invoice, payment, posted_invoice,
    posted_invoice_line,
};
use crate::plugins::finance_invoices::logic::preferences::{
    load_payment_preferences, validate_payment_preferences_for_create,
};
use crate::plugins::finance_invoices::logic::tax_assoc::set_payment_taxes;
use crate::plugins::finance_invoices::logic::tax_calculations::{
    InvoiceLinesTotals, invoice_line_amount_breakdown, invoice_receivable_grand_total,
    merge_invoice_line_tax_ids, payment_bank_amount, payment_withholding_base, taxes_withholding,
    validate_payment_taxes, withholding_tax_account_id,
};

pub struct CreatePaymentInput {
    pub posted_invoice_id: i64,
    pub amount: Decimal,
    pub account_id: Option<i64>,
    pub datetime: DateTime<Utc>,
    pub withholding_tax_ids: Vec<i64>,
}

async fn sum_posted_invoice_payments<C: ConnectionTrait>(
    db: &C,
    posted_invoice_id: i64,
) -> Result<Decimal, String> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COALESCE(SUM(amount), 0) AS s FROM payments WHERE posted_invoice_id = $1",
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
    let _posted = posted_invoice::Entity::find_by_id(posted_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "posted invoice not found".to_string())?;

    let header_tax_ids =
        crate::plugins::finance_invoices::logic::tax_assoc::load_posted_invoice_tax_ids(
            db, posted_id,
        )
        .await
        .unwrap_or_default();
    let header_taxes = load_taxes_by_ids(db, &header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    let lines = posted_invoice_line::Entity::find()
        .filter(posted_invoice_line::Column::PostedInvoiceId.eq(posted_id))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    let mut totals = InvoiceLinesTotals::default();
    let mut line_tax_ids = HashSet::new();
    for line in &lines {
        let line_tax_ids_vec =
            crate::plugins::finance_invoices::logic::tax_assoc::load_posted_line_tax_ids(
                db, line.id,
            )
            .await
            .unwrap_or_default();
        let line_taxes = load_taxes_by_ids(db, &line_tax_ids_vec)
            .await
            .map_err(|e| e.to_string())?;
        merge_invoice_line_tax_ids(&mut line_tax_ids, &line_taxes);
        let (untaxed, levied, withholding, _) =
            invoice_line_amount_breakdown(line.quantity, line.rate, &line_taxes);
        totals.untaxed_subtotal = decimal::dec_sum(totals.untaxed_subtotal, untaxed);
        totals.lines_levied = decimal::dec_sum(totals.lines_levied, levied);
        totals.lines_withholding = decimal::dec_sum(totals.lines_withholding, withholding);
    }
    Ok(PostedInvoiceAmounts {
        untaxed_subtotal: totals.untaxed_subtotal,
        receivable_total: invoice_receivable_grand_total(&totals, &header_taxes, &line_tax_ids),
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

/// Remaining receivable after existing payments on a posted invoice.
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
/// Returns the posted invoice, receivable total, untaxed subtotal, and whether this
/// allocation fully pays it.
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

/// AR credit and withholding debit lines for one invoice allocation (excludes bank debit).
///
/// `withholding_base` is the untaxed amount the withholding percent applies to (not the
/// GST-inclusive settlement).
pub fn build_payment_lines_for_allocation(
    posted: &posted_invoice::Model,
    settlement: Decimal,
    withholding_base: Decimal,
    taxes: &[tax::Model],
) -> Result<(Decimal, Vec<JournalLineSpec>), String> {
    let settlement = decimal::normalize(settlement);
    let withholding_base = decimal::normalize(withholding_base);
    let bank_amt = payment_bank_amount(settlement, withholding_base, taxes);
    if bank_amt < Decimal::ZERO {
        return Err("withholding exceeds settlement amount".to_string());
    }

    let mut lines = vec![JournalLineSpec {
        account_id: posted.account_receivable_id,
        amount: decimal::dec_neg(settlement),
    }];
    for tax in taxes_withholding(taxes) {
        let wh_amt = crate::plugins::finance_invoices::logic::tax_calculations::tax_amount_for_tax(
            withholding_base,
            tax,
        );
        if wh_amt.is_zero() {
            continue;
        }
        let acct_id = withholding_tax_account_id(tax)?;
        lines.push(JournalLineSpec {
            account_id: acct_id,
            amount: wh_amt,
        });
    }
    Ok((bank_amt, lines))
}

/// Insert paid or partially-paid settlement row for a payment.
/// Returns the new settlement row id.
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

pub struct CreatePaymentResult {
    pub payment: payment::Model,
    /// True when the payment fully settles the posted invoice.
    pub is_full: bool,
    pub settlement_id: i64,
}

pub async fn create_payment(
    db: &DatabaseConnection,
    input: CreatePaymentInput,
) -> Result<CreatePaymentResult, String> {
    let payment_prefs = load_payment_preferences(db).await;
    validate_payment_preferences_for_create(db, &payment_prefs).await?;

    let account_id = input
        .account_id
        .filter(|id| *id > 0)
        .unwrap_or_else(|| optional_i64(payment_prefs.payment_account_id));
    validate_leaf_account_balance_type(db, account_id, debit_balance_type(), "payment account")
        .await
        .map_err(|e| e.to_string())?;

    let (posted, inv_total, untaxed_subtotal, is_full) =
        validate_payment_allocation(db, input.posted_invoice_id, input.amount).await?;

    let taxes = load_taxes_by_ids(db, &input.withholding_tax_ids)
        .await
        .map_err(|e| e.to_string())?;
    validate_payment_taxes(&taxes)?;

    let now = Utc::now();
    let dt = if input.datetime.timestamp() == 0 {
        now
    } else {
        input.datetime
    };

    let txn = db.begin().await.map_err(|e| e.to_string())?;

    let doc_id = create_source_doc(&txn, PAYMENT_SOURCE_DOC_TYPE)
        .await
        .map_err(|e| e.to_string())?;

    let settlement = decimal::normalize(input.amount);
    let withholding_base = payment_withholding_base(settlement, inv_total, untaxed_subtotal);
    let (bank_amt, alloc_lines) =
        build_payment_lines_for_allocation(&posted, settlement, withholding_base, &taxes)?;

    let mut lines = vec![JournalLineSpec {
        account_id,
        amount: bank_amt,
    }];
    lines.extend(alloc_lines);

    let (je_id, _) = insert_journal_entry(&txn, now, posted.journal_id, doc_id, &lines)
        .await
        .map_err(|e| e.to_string())?;

    let pay = payment::ActiveModel {
        posted_invoice_id: Set(posted.id),
        amount: Set(settlement),
        account_id: Set(account_id),
        datetime: Set(dt),
        journal_entry_id: Set(je_id),
        payment_batch_id: Set(None),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;

    update_source_doc_id(&txn, doc_id, pay.id)
        .await
        .map_err(|e| e.to_string())?;

    set_payment_taxes(&txn, pay.id, &input.withholding_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    let settlement_id = record_payment_settlement(&txn, pay.id, posted.id, is_full).await?;

    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(CreatePaymentResult {
        payment: pay,
        is_full,
        settlement_id,
    })
}

pub fn parse_payment_amount(s: &str) -> Result<Decimal, String> {
    parse_decimal(s).ok_or_else(|| "invalid amount".to_string())
}

pub fn parse_withholding_tax_ids(s: &str) -> Vec<i64> {
    s.split(',')
        .filter_map(|p| p.trim().parse().ok())
        .filter(|id| *id > 0)
        .collect()
}
