use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Select, sea_query::Expr,
};

use crate::plugins::users::state::AuthContext;

use crate::plugins::finance_common::is_superuser;

use crate::plugins::finance_fiscal_year::entities::fiscal_year::{
    self, Entity as FiscalYearEntity,
};

pub const FISCAL_YEAR_COOKIE: &str = "finance_fiscal_year_id";

pub fn format_fiscal_date(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d").to_string()
}

pub fn format_fiscal_date_input(dt: DateTime<Utc>) -> String {
    format_fiscal_date(dt)
}

fn parse_fiscal_date_only(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                .ok()
                .or_else(|| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").ok())
                .map(|dt| dt.date())
        })
        .or_else(|| {
            NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|dt| dt.date())
        })
}

pub fn parse_fiscal_date_start(s: &str) -> DateTime<Utc> {
    parse_fiscal_date_only(s)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|ndt| ndt.and_utc())
        .unwrap_or_else(Utc::now)
}

pub fn parse_fiscal_date_end(s: &str) -> DateTime<Utc> {
    parse_fiscal_date_only(s)
        .and_then(|d| d.and_hms_opt(23, 59, 59))
        .map(|ndt| ndt.and_utc())
        .unwrap_or_else(Utc::now)
}

pub fn scope_fiscal_years(
    query: Select<FiscalYearEntity>,
    auth: &AuthContext,
) -> Select<FiscalYearEntity> {
    if is_superuser(auth) {
        return query;
    }
    query.filter(Expr::cust("1 = 0"))
}

pub fn apply_fiscal_year_filters(
    mut query: Select<FiscalYearEntity>,
    code: Option<&str>,
    name: Option<&str>,
) -> Select<FiscalYearEntity> {
    if let Some(c) = code.filter(|s| !s.is_empty()) {
        query = query.filter(fiscal_year::Column::Code.contains(c));
    }
    if let Some(n) = name.filter(|s| !s.is_empty()) {
        query = query.filter(fiscal_year::Column::Name.contains(n));
    }
    query
}

pub async fn find_fiscal_year_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<fiscal_year::Model> {
    let query = FiscalYearEntity::find_by_id(id);
    scope_fiscal_years(query, auth).one(db).await.ok().flatten()
}

pub fn model_to_row(
    fy: fiscal_year::Model,
) -> crate::plugins::finance_fiscal_year::templates::FiscalYearRow {
    crate::plugins::finance_fiscal_year::templates::FiscalYearRow {
        id: fy.id,
        code: fy.code,
        name: fy.name,
        start: format_fiscal_date(fy.starts_at),
        end: format_fiscal_date(fy.ends_at),
        is_active: fy.is_active,
    }
}

pub async fn load_fiscal_year_for_datetime(
    db: &DatabaseConnection,
    dt: DateTime<Utc>,
) -> Option<fiscal_year::Model> {
    FiscalYearEntity::find()
        .filter(fiscal_year::Column::StartsAt.lte(dt))
        .filter(fiscal_year::Column::EndsAt.gte(dt))
        .order_by_desc(fiscal_year::Column::StartsAt)
        .one(db)
        .await
        .ok()
        .flatten()
}

pub async fn load_active_fiscal_year(db: &DatabaseConnection) -> Option<fiscal_year::Model> {
    FiscalYearEntity::find()
        .filter(fiscal_year::Column::IsActive.eq(true))
        .order_by_desc(fiscal_year::Column::StartsAt)
        .one(db)
        .await
        .ok()
        .flatten()
}

pub async fn resolve_fiscal_year_for_invoice(
    db: &DatabaseConnection,
    invoice_datetime: DateTime<Utc>,
) -> Option<fiscal_year::Model> {
    if let Some(fy) = load_fiscal_year_for_datetime(db, invoice_datetime).await {
        return Some(fy);
    }
    load_active_fiscal_year(db).await
}

pub fn fiscal_year_from_cookie(cookie_val: Option<&str>) -> Option<i64> {
    cookie_val.and_then(|s| s.trim().parse().ok())
}
