//! Per-purchase metrics for hub list tables.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, QueryFilter,
    Statement,
};

use lariv_plugin_finance_common::decimal;
use lariv_plugin_finance_taxes::scope::load_taxes_by_ids;

use crate::entities::{draft_purchase, draft_purchase_line, posted_purchase_line};
use crate::logic::draft_payment_term::{
    load_draft_purchase_payment_term_lines, load_posted_payment_term_for_cancelled,
    load_posted_payment_term_for_posted, resolve_due_date,
};
use crate::logic::tax_assoc::{
    load_cancelled_purchase_tax_ids, load_cancelled_line_tax_ids, load_draft_purchase_tax_ids,
    load_draft_line_tax_ids, load_posted_purchase_tax_ids, load_posted_line_tax_ids,
};
use crate::logic::tax_calculations::{
    PurchaseLinesTotals, purchase_amounts_from_line_totals, purchase_line_amounts,
    merge_purchase_line_tax_ids,
};

#[derive(Clone, Debug, Default)]
pub struct PurchaseListMetrics {
    pub untaxed: Decimal,
    pub total: Decimal,
    pub tax_levied: Decimal,
    pub product_count: u32,
    pub final_due: Option<NaiveDate>,
}

fn accumulate_line(
    totals: &mut PurchaseLinesTotals,
    line_tax_ids: &mut HashSet<i64>,
    untaxed: Decimal,
    taxes: &[lariv_plugin_finance_taxes::entities::tax::Model],
) {
    merge_purchase_line_tax_ids(line_tax_ids, taxes);
    let (untaxed, levied, withholding, _) = purchase_line_amounts(untaxed, taxes);
    totals.untaxed_subtotal = decimal::dec_sum(totals.untaxed_subtotal, untaxed);
    totals.lines_levied = decimal::dec_sum(totals.lines_levied, levied);
    totals.lines_withholding = decimal::dec_sum(totals.lines_withholding, withholding);
}

fn metrics_from_totals(
    totals: &PurchaseLinesTotals,
    header_taxes: &[lariv_plugin_finance_taxes::entities::tax::Model],
    line_tax_ids: &HashSet<i64>,
    product_count: u32,
    final_due: Option<NaiveDate>,
) -> PurchaseListMetrics {
    let (untaxed, tax_levied, total) =
        purchase_amounts_from_line_totals(totals, header_taxes, line_tax_ids);
    PurchaseListMetrics {
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
    tz: &str,
) -> Option<NaiveDate> {
    let lines = load_draft_purchase_payment_term_lines(db, draft_id)
        .await
        .unwrap_or_default();
    lines
        .iter()
        .filter_map(|l| resolve_due_date(l, anchor, tz).ok())
        .max()
}

async fn posted_final_due(db: &DatabaseConnection, posted_id: i64) -> Option<NaiveDate> {
    let Some((_, lines)) = lariv_core::web::opt_or_log(
        load_posted_payment_term_for_posted(db, posted_id).await,
        "db find",
    ) else {
        return None;
    };
    lines.into_iter().map(|l| l.due_date).max()
}

async fn cancelled_final_due(db: &DatabaseConnection, cancelled_id: i64) -> Option<NaiveDate> {
    let Some((_, lines)) = lariv_core::web::opt_or_log(
        load_posted_payment_term_for_cancelled(db, cancelled_id).await,
        "db find",
    ) else {
        return None;
    };
    lines.into_iter().map(|l| l.due_date).max()
}

pub async fn draft_purchase_list_metrics(
    db: &DatabaseConnection,
    draft_id: i64,
    tz: &str,
) -> PurchaseListMetrics {
    let Ok(Some(draft)) = draft_purchase::Entity::find_by_id(draft_id).one(db).await else {
        return PurchaseListMetrics::default();
    };
    let header_tax_ids = load_draft_purchase_tax_ids(db, draft_id)
        .await
        .unwrap_or_default();
    let header_taxes = load_taxes_by_ids(db, &header_tax_ids)
        .await
        .unwrap_or_default();
    let lines = draft_purchase_line::Entity::find()
        .filter(draft_purchase_line::Column::DraftPurchaseId.eq(draft_id))
        .all(db)
        .await
        .unwrap_or_default();
    let product_count = lines.len() as u32;
    let mut totals = PurchaseLinesTotals::default();
    let mut line_tax_ids = HashSet::new();
    for line in &lines {
        let tax_ids = load_draft_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        let taxes = load_taxes_by_ids(db, &tax_ids).await.unwrap_or_default();
        accumulate_line(&mut totals, &mut line_tax_ids, line.pre_tax_amount, &taxes);
    }
    let final_due = draft_final_due(db, draft_id, draft.datetime, tz).await;
    metrics_from_totals(
        &totals,
        &header_taxes,
        &line_tax_ids,
        product_count,
        final_due,
    )
}

pub async fn posted_purchase_list_metrics(
    db: &DatabaseConnection,
    posted_id: i64,
) -> PurchaseListMetrics {
    let header_tax_ids = load_posted_purchase_tax_ids(db, posted_id)
        .await
        .unwrap_or_default();
    let header_taxes = load_taxes_by_ids(db, &header_tax_ids)
        .await
        .unwrap_or_default();
    let lines = posted_purchase_line::Entity::find()
        .filter(posted_purchase_line::Column::PostedPurchaseId.eq(posted_id))
        .all(db)
        .await
        .unwrap_or_default();
    let product_count = lines.len() as u32;
    let mut totals = PurchaseLinesTotals::default();
    let mut line_tax_ids = HashSet::new();
    for line in &lines {
        let tax_ids = load_posted_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        let taxes = load_taxes_by_ids(db, &tax_ids).await.unwrap_or_default();
        accumulate_line(&mut totals, &mut line_tax_ids, line.pre_tax_amount, &taxes);
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

pub async fn cancelled_purchase_list_metrics(
    db: &DatabaseConnection,
    cancelled_id: i64,
) -> PurchaseListMetrics {
    let header_tax_ids = load_cancelled_purchase_tax_ids(db, cancelled_id)
        .await
        .unwrap_or_default();
    let header_taxes = load_taxes_by_ids(db, &header_tax_ids)
        .await
        .unwrap_or_default();

    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, pre_tax_amount FROM cancelled_purchase_lines \
             WHERE cancelled_purchase_id = $1 ORDER BY id ASC",
            [cancelled_id.into()],
        ))
        .await
        .unwrap_or_default();

    let product_count = rows.len() as u32;
    let mut totals = PurchaseLinesTotals::default();
    let mut line_tax_ids = HashSet::new();
    for row in rows {
        let Ok(line_id) = row.try_get::<i64>("", "id") else {
            continue;
        };
        let Ok(pre_tax) = row.try_get::<Decimal>("", "pre_tax_amount") else {
            continue;
        };
        let tax_ids = load_cancelled_line_tax_ids(db, line_id)
            .await
            .unwrap_or_default();
        let taxes = load_taxes_by_ids(db, &tax_ids).await.unwrap_or_default();
        accumulate_line(&mut totals, &mut line_tax_ids, pre_tax, &taxes);
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

/// Batch-load posted metrics keyed by posted purchase id.
pub async fn posted_purchase_list_metrics_map(
    db: &DatabaseConnection,
    posted_ids: &[i64],
) -> HashMap<i64, PurchaseListMetrics> {
    let mut out = HashMap::with_capacity(posted_ids.len());
    for &id in posted_ids {
        if id > 0 && !out.contains_key(&id) {
            out.insert(id, posted_purchase_list_metrics(db, id).await);
        }
    }
    out
}
