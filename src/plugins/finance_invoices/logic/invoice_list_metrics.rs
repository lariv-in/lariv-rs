//! Per-invoice metrics for hub list tables.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, QueryFilter,
    Statement,
};

use crate::plugins::finance_common::decimal;
use crate::plugins::finance_taxes::scope::load_taxes_by_ids;

use crate::plugins::finance_invoices::entities::{
    draft_invoice, draft_invoice_line, posted_invoice_line,
};
use crate::plugins::finance_invoices::logic::draft_payment_term::{
    load_draft_payment_term_lines, load_posted_payment_term_for_cancelled,
    load_posted_payment_term_for_posted, resolve_due_datetime,
};
use crate::plugins::finance_invoices::logic::tax_assoc::{
    load_cancelled_invoice_tax_ids, load_cancelled_line_tax_ids, load_draft_invoice_tax_ids,
    load_draft_line_tax_ids, load_posted_invoice_tax_ids, load_posted_line_tax_ids,
};
use crate::plugins::finance_invoices::logic::tax_calculations::{
    InvoiceLinesTotals, invoice_amounts_from_line_totals, invoice_line_amount_breakdown,
    merge_invoice_line_tax_ids,
};

#[derive(Clone, Debug, Default)]
pub struct InvoiceListMetrics {
    pub untaxed: Decimal,
    pub total: Decimal,
    pub tax_levied: Decimal,
    pub product_count: u32,
    pub final_due: Option<DateTime<Utc>>,
}

fn accumulate_line(
    totals: &mut InvoiceLinesTotals,
    line_tax_ids: &mut HashSet<i64>,
    qty: Decimal,
    rate: Decimal,
    taxes: &[crate::plugins::finance_taxes::entities::tax::Model],
) {
    merge_invoice_line_tax_ids(line_tax_ids, taxes);
    let (untaxed, levied, withholding, _) = invoice_line_amount_breakdown(qty, rate, taxes);
    totals.untaxed_subtotal = decimal::dec_sum(totals.untaxed_subtotal, untaxed);
    totals.lines_levied = decimal::dec_sum(totals.lines_levied, levied);
    totals.lines_withholding = decimal::dec_sum(totals.lines_withholding, withholding);
}

fn metrics_from_totals(
    totals: &InvoiceLinesTotals,
    header_taxes: &[crate::plugins::finance_taxes::entities::tax::Model],
    line_tax_ids: &HashSet<i64>,
    product_count: u32,
    final_due: Option<DateTime<Utc>>,
) -> InvoiceListMetrics {
    let (untaxed, tax_levied, total) =
        invoice_amounts_from_line_totals(totals, header_taxes, line_tax_ids);
    InvoiceListMetrics {
        untaxed,
        total,
        tax_levied,
        product_count,
        final_due,
    }
}

async fn draft_final_due(
    db: &DatabaseConnection,
    draft_id: i64,
    anchor: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let lines = load_draft_payment_term_lines(db, draft_id)
        .await
        .unwrap_or_default();
    lines
        .iter()
        .filter_map(|l| resolve_due_datetime(l, anchor).ok())
        .max()
}

async fn posted_final_due(db: &DatabaseConnection, posted_id: i64) -> Option<DateTime<Utc>> {
    let Some((_, lines)) = load_posted_payment_term_for_posted(db, posted_id)
        .await
        .ok()
        .flatten()
    else {
        return None;
    };
    lines.into_iter().map(|l| l.due_datetime).max()
}

async fn cancelled_final_due(db: &DatabaseConnection, cancelled_id: i64) -> Option<DateTime<Utc>> {
    let Some((_, lines)) = load_posted_payment_term_for_cancelled(db, cancelled_id)
        .await
        .ok()
        .flatten()
    else {
        return None;
    };
    lines.into_iter().map(|l| l.due_datetime).max()
}

pub async fn draft_invoice_list_metrics(
    db: &DatabaseConnection,
    draft_id: i64,
) -> InvoiceListMetrics {
    let Ok(Some(draft)) = draft_invoice::Entity::find_by_id(draft_id).one(db).await else {
        return InvoiceListMetrics::default();
    };
    let header_tax_ids = load_draft_invoice_tax_ids(db, draft_id)
        .await
        .unwrap_or_default();
    let header_taxes = load_taxes_by_ids(db, &header_tax_ids)
        .await
        .unwrap_or_default();
    let lines = draft_invoice_line::Entity::find()
        .filter(draft_invoice_line::Column::DraftInvoiceId.eq(draft_id))
        .all(db)
        .await
        .unwrap_or_default();
    let product_count = lines.len() as u32;
    let mut totals = InvoiceLinesTotals::default();
    let mut line_tax_ids = HashSet::new();
    for line in &lines {
        let tax_ids = load_draft_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        let taxes = load_taxes_by_ids(db, &tax_ids).await.unwrap_or_default();
        accumulate_line(
            &mut totals,
            &mut line_tax_ids,
            line.quantity,
            line.rate,
            &taxes,
        );
    }
    let final_due = draft_final_due(db, draft_id, draft.datetime).await;
    metrics_from_totals(
        &totals,
        &header_taxes,
        &line_tax_ids,
        product_count,
        final_due,
    )
}

pub async fn posted_invoice_list_metrics(
    db: &DatabaseConnection,
    posted_id: i64,
) -> InvoiceListMetrics {
    let header_tax_ids = load_posted_invoice_tax_ids(db, posted_id)
        .await
        .unwrap_or_default();
    let header_taxes = load_taxes_by_ids(db, &header_tax_ids)
        .await
        .unwrap_or_default();
    let lines = posted_invoice_line::Entity::find()
        .filter(posted_invoice_line::Column::PostedInvoiceId.eq(posted_id))
        .all(db)
        .await
        .unwrap_or_default();
    let product_count = lines.len() as u32;
    let mut totals = InvoiceLinesTotals::default();
    let mut line_tax_ids = HashSet::new();
    for line in &lines {
        let tax_ids = load_posted_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        let taxes = load_taxes_by_ids(db, &tax_ids).await.unwrap_or_default();
        accumulate_line(
            &mut totals,
            &mut line_tax_ids,
            line.quantity,
            line.rate,
            &taxes,
        );
    }
    let final_due = posted_final_due(db, posted_id).await;
    metrics_from_totals(
        &totals,
        &header_taxes,
        &line_tax_ids,
        product_count,
        final_due,
    )
}

pub async fn cancelled_invoice_list_metrics(
    db: &DatabaseConnection,
    cancelled_id: i64,
) -> InvoiceListMetrics {
    let header_tax_ids = load_cancelled_invoice_tax_ids(db, cancelled_id)
        .await
        .unwrap_or_default();
    let header_taxes = load_taxes_by_ids(db, &header_tax_ids)
        .await
        .unwrap_or_default();

    let rows = db
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, quantity, rate FROM cancelled_invoice_lines \
             WHERE cancelled_invoice_id = $1 ORDER BY id ASC",
            [cancelled_id.into()],
        ))
        .await
        .unwrap_or_default();

    let product_count = rows.len() as u32;
    let mut totals = InvoiceLinesTotals::default();
    let mut line_tax_ids = HashSet::new();
    for row in rows {
        let Ok(line_id) = row.try_get::<i64>("", "id") else {
            continue;
        };
        let Ok(quantity) = row.try_get::<Decimal>("", "quantity") else {
            continue;
        };
        let Ok(rate) = row.try_get::<Decimal>("", "rate") else {
            continue;
        };
        let tax_ids = load_cancelled_line_tax_ids(db, line_id)
            .await
            .unwrap_or_default();
        let taxes = load_taxes_by_ids(db, &tax_ids).await.unwrap_or_default();
        accumulate_line(&mut totals, &mut line_tax_ids, quantity, rate, &taxes);
    }

    let final_due = cancelled_final_due(db, cancelled_id).await;
    metrics_from_totals(
        &totals,
        &header_taxes,
        &line_tax_ids,
        product_count,
        final_due,
    )
}

/// Batch-load posted metrics keyed by posted invoice id.
pub async fn posted_invoice_list_metrics_map(
    db: &DatabaseConnection,
    posted_ids: &[i64],
) -> HashMap<i64, InvoiceListMetrics> {
    let mut out = HashMap::with_capacity(posted_ids.len());
    for &id in posted_ids {
        if id > 0 && !out.contains_key(&id) {
            out.insert(id, posted_invoice_list_metrics(db, id).await);
        }
    }
    out
}
