//! Invoice list filters (fiscal year, datetime range, tab eligibility).

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, sea_query::Expr};
use serde::Deserialize;

use crate::plugins::finance_fiscal_year::{
    entities::fiscal_year::{self, Entity as FiscalYearEntity},
    scope::{load_active_fiscal_year, load_fiscal_year_for_datetime},
};

use crate::plugins::finance_invoices::entities::{
    draft_invoice::{self, Entity as DraftInvoiceEntity},
    paid_invoice::{self, Entity as PaidInvoiceEntity},
    partially_paid_invoice::{self, Entity as PartiallyPaidInvoiceEntity},
    posted_invoice::{self, Entity as PostedInvoiceEntity},
};

pub const INVOICE_FISCAL_YEAR_COOKIE: &str = "finance_invoices_fiscal_year";

/// Parsed Lariv `environment` JSON cookie (forward-compatible via [`Self::extra`]).
#[derive(Debug, Default, Deserialize)]
pub struct LarivEnvironment {
    #[serde(default, rename = "finance_invoices_fiscal_year")]
    pub fiscal_year_id: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, String>,
}

impl LarivEnvironment {
    pub fn from_cookie_header(cookie_raw: Option<&str>) -> Self {
        let Some(raw) = cookie_raw else {
            return Self::default();
        };
        for part in raw.split(';') {
            let part = part.trim();
            if let Some(val) = part.strip_prefix("environment=") {
                let decoded = percent_decode(val);
                if let Ok(env) = serde_json::from_str::<Self>(&decoded) {
                    return env;
                }
            }
        }
        Self::default()
    }
}

pub fn parse_filter_datetime(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|| {
            NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                .ok()
                .or_else(|| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").ok())
                .map(|ndt| ndt.and_utc())
        })
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(v) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub struct FiscalYearScope {
    pub restrict: bool,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

pub async fn default_fiscal_year_id(db: &DatabaseConnection) -> Option<i64> {
    let now = Utc::now();
    if let Some(fy) = load_fiscal_year_for_datetime(db, now).await {
        return Some(fy.id);
    }
    load_active_fiscal_year(db).await.map(|fy| fy.id)
}

/// Selected fiscal year for the environment dropdown (None = explicit "—" / all years).
pub async fn selected_fiscal_year_id_for_ui(
    db: &DatabaseConnection,
    env: &LarivEnvironment,
) -> Option<i64> {
    if let Some(raw) = env.fiscal_year_id.as_deref() {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }
        if let Ok(id) = raw.parse::<i64>() {
            if id > 0 {
                return Some(id);
            }
        }
        return None;
    }
    default_fiscal_year_id(db).await
}

pub async fn list_fiscal_year_options(db: &DatabaseConnection) -> Vec<(i64, String)> {
    FiscalYearEntity::find()
        .order_by_desc(fiscal_year::Column::StartsAt)
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|fy| (fy.id, fy.name))
        .collect()
}

pub async fn resolve_list_fiscal_year(
    db: &DatabaseConnection,
    env: &LarivEnvironment,
) -> Option<FiscalYearScope> {
    if let Some(raw) = env.fiscal_year_id.as_deref() {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }
        if let Ok(id) = raw.parse::<i64>() {
            if id > 0 {
                if let Ok(Some(fy)) = FiscalYearEntity::find_by_id(id).one(db).await {
                    return Some(FiscalYearScope {
                        restrict: true,
                        starts_at: fy.starts_at,
                        ends_at: fy.ends_at,
                    });
                }
            }
        }
        return None;
    }
    let now = Utc::now();
    if let Some(fy) = load_fiscal_year_for_datetime(db, now).await {
        return Some(FiscalYearScope {
            restrict: true,
            starts_at: fy.starts_at,
            ends_at: fy.ends_at,
        });
    }
    if let Some(fy) = load_active_fiscal_year(db).await {
        return Some(FiscalYearScope {
            restrict: true,
            starts_at: fy.starts_at,
            ends_at: fy.ends_at,
        });
    }
    None
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
