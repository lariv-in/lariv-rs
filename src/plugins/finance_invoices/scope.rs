//! Invoice list filters (datetime range, tab eligibility).

use chrono::{DateTime, Utc};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, sea_query::Expr};

use crate::plugins::finance_invoices::entities::{
    draft_invoice::{self, Entity as DraftInvoiceEntity},
    paid_invoice::{self, Entity as PaidInvoiceEntity},
    partially_paid_invoice::{self, Entity as PartiallyPaidInvoiceEntity},
    posted_invoice::{self, Entity as PostedInvoiceEntity},
};

pub fn parse_filter_datetime(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|| crate::datetime::parse_naive_datetime(s).map(|ndt| ndt.and_utc()))
}

pub fn sql_posted_not_cancelled() -> sea_orm::sea_query::SimpleExpr {
    Expr::cust(
        "NOT EXISTS (SELECT 1 FROM cancelled_invoices c WHERE c.posted_invoice_id = posted_invoices.id)",
    )
}

pub fn sql_posted_not_fully_paid() -> sea_orm::sea_query::SimpleExpr {
    Expr::cust(
        "NOT EXISTS (SELECT 1 FROM paid_invoices paid WHERE paid.posted_invoice_id = posted_invoices.id)",
    )
}

pub fn sql_posted_not_partially_paid() -> sea_orm::sea_query::SimpleExpr {
    Expr::cust(
        "NOT EXISTS (SELECT 1 FROM partially_paid_invoices pp WHERE pp.posted_invoice_id = posted_invoices.id)",
    )
}

pub fn sql_settlement_posted_not_cancelled(table: &str) -> sea_orm::sea_query::SimpleExpr {
    Expr::cust(format!(
        "NOT EXISTS (SELECT 1 FROM cancelled_invoices c WHERE c.posted_invoice_id = {table}.posted_invoice_id)"
    ))
}

pub fn sql_draft_not_posted() -> sea_orm::sea_query::SimpleExpr {
    Expr::cust(
        "NOT EXISTS (SELECT 1 FROM posted_invoices p WHERE p.draft_invoice_id = draft_invoices.id)",
    )
}

/// Hub list URL for a tab (`drafts`, `posted`, `paid`, `partial`, `cancelled`).
pub fn hub_tab_url(tab: &str) -> String {
    format!("/finance-invoices/?tab={tab}")
}

/// Payments list URL for a tab (`single`, `batches`).
pub fn payments_tab_url(tab: &str) -> String {
    format!("/finance-invoices/payments/?tab={tab}")
}

/// Draft still listed under the drafts hub tab (not deleted, not posted).
pub async fn find_active_draft(db: &DatabaseConnection, id: i64) -> Option<draft_invoice::Model> {
    DraftInvoiceEntity::find_by_id(id)
        .filter(sql_draft_not_posted())
        .one(db)
        .await
        .ok()
        .flatten()
}

/// Posted invoice still listed under the posted hub tab.
pub async fn find_active_posted(db: &DatabaseConnection, id: i64) -> Option<posted_invoice::Model> {
    PostedInvoiceEntity::find_by_id(id)
        .filter(sql_posted_not_cancelled())
        .filter(sql_posted_not_fully_paid())
        .filter(sql_posted_not_partially_paid())
        .one(db)
        .await
        .ok()
        .flatten()
}

/// Posted invoice that can still be cancelled (unpaid, partial, or paid; not already cancelled).
pub async fn find_cancellable_posted(
    db: &DatabaseConnection,
    id: i64,
) -> Option<posted_invoice::Model> {
    PostedInvoiceEntity::find_by_id(id)
        .filter(sql_posted_not_cancelled())
        .one(db)
        .await
        .ok()
        .flatten()
}

/// Paid settlement still listed under the paid hub tab.
pub async fn find_active_paid(db: &DatabaseConnection, id: i64) -> Option<paid_invoice::Model> {
    PaidInvoiceEntity::find_by_id(id)
        .filter(sql_settlement_posted_not_cancelled("paid_invoices"))
        .one(db)
        .await
        .ok()
        .flatten()
}

/// Partial settlement still listed under the partial hub tab.
pub async fn find_active_partial(
    db: &DatabaseConnection,
    id: i64,
) -> Option<partially_paid_invoice::Model> {
    PartiallyPaidInvoiceEntity::find_by_id(id)
        .filter(sql_settlement_posted_not_cancelled(
            "partially_paid_invoices",
        ))
        .one(db)
        .await
        .ok()
        .flatten()
}
