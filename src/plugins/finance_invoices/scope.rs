//! Invoice list filters (fiscal year, datetime range, tab eligibility).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, sea_query::Expr};
use serde::Deserialize;

use crate::plugins::finance_common::fiscal_year::FiscalYear;
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
    /// Fiscal year start calendar year as a string (e.g. `"2024"`), or empty for all years.
    #[serde(default, rename = "finance_invoices_fiscal_year")]
    pub fiscal_year: Option<String>,
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
        .or_else(|| crate::datetime::parse_naive_datetime(s).map(|ndt| ndt.and_utc()))
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

fn parse_start_year(raw: &str) -> Option<i32> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    raw.parse::<i32>().ok().filter(|&y| y > 0)
}

/// Default filter: current Indian FY (Apr–Mar).
pub fn default_fiscal_year() -> FiscalYear {
    FiscalYear::for_datetime(Utc::now())
}

/// Selected FY start year for the environment dropdown (`None` = explicit "—" / all years).
pub fn selected_fiscal_year_start_for_ui(env: &LarivEnvironment) -> Option<i32> {
    match env.fiscal_year.as_deref() {
        Some(raw) => {
            let raw = raw.trim();
            if raw.is_empty() {
                None
            } else {
                parse_start_year(raw)
            }
        }
        None => Some(default_fiscal_year().start_year),
    }
}

pub fn list_fiscal_year_options() -> Vec<(i32, String)> {
    FiscalYear::options_around(Utc::now(), 5, 1)
        .into_iter()
        .map(|fy| (fy.start_year, fy.label))
        .collect()
}

/// Restrict list queries to a fiscal year window, if the environment selects one.
pub fn resolve_list_fiscal_year(env: &LarivEnvironment) -> Option<FiscalYear> {
    match env.fiscal_year.as_deref() {
        Some(raw) => {
            let raw = raw.trim();
            if raw.is_empty() {
                None
            } else {
                parse_start_year(raw).map(FiscalYear::from_start_year)
            }
        }
        None => Some(default_fiscal_year()),
    }
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
    crate::web::opt_or_log(
        DraftInvoiceEntity::find_by_id(id)
            .filter(sql_draft_not_posted())
            .one(db)
            .await,
        "find by id",
    )
}

/// Posted invoice still listed under the posted hub tab.
pub async fn find_active_posted(db: &DatabaseConnection, id: i64) -> Option<posted_invoice::Model> {
    crate::web::opt_or_log(
        PostedInvoiceEntity::find_by_id(id)
            .filter(sql_posted_not_cancelled())
            .filter(sql_posted_not_fully_paid())
            .filter(sql_posted_not_partially_paid())
            .one(db)
            .await,
        "find by id",
    )
}

/// Posted invoice that can still be cancelled (unpaid, partial, or paid; not already cancelled).
pub async fn find_cancellable_posted(
    db: &DatabaseConnection,
    id: i64,
) -> Option<posted_invoice::Model> {
    crate::web::opt_or_log(
        PostedInvoiceEntity::find_by_id(id)
            .filter(sql_posted_not_cancelled())
            .one(db)
            .await,
        "find by id",
    )
}

/// Paid settlement still listed under the paid hub tab.
pub async fn find_active_paid(db: &DatabaseConnection, id: i64) -> Option<paid_invoice::Model> {
    crate::web::opt_or_log(
        PaidInvoiceEntity::find_by_id(id)
            .filter(sql_settlement_posted_not_cancelled("paid_invoices"))
            .one(db)
            .await,
        "find by id",
    )
}

/// Partial settlement still listed under the partial hub tab.
pub async fn find_active_partial(
    db: &DatabaseConnection,
    id: i64,
) -> Option<partially_paid_invoice::Model> {
    crate::web::opt_or_log(
        PartiallyPaidInvoiceEntity::find_by_id(id)
            .filter(sql_settlement_posted_not_cancelled(
                "partially_paid_invoices",
            ))
            .one(db)
            .await,
        "find by id",
    )
}
