//! Quotation number formatting. Placeholders match posted invoice numbers.

use chrono::{NaiveDate, Utc};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};

use crate::entities::quotation::{self, Entity as QuotationEntity};
use crate::logic::prefs::load_preferences;
use lariv_plugin_finance_common::fiscal_year::FiscalYear;
use lariv_plugin_finance_invoices::logic::invoice_number::format_posted_invoice_number;

pub const DEFAULT_QUOTATION_NUMBER_FORMAT: &str = "QT-{{YYYY}}-{{POSTED_SEQ}}";

pub async fn next_quotation_seq(db: &DatabaseConnection) -> Result<i64, sea_orm::DbErr> {
    let max = QuotationEntity::find()
        .order_by_desc(quotation::Column::Id)
        .one(db)
        .await?;
    Ok(max.map(|row| row.id).unwrap_or(0) + 1)
}

pub async fn next_fiscal_quotation_seq(
    db: &DatabaseConnection,
    date: NaiveDate,
    exclude_id: Option<i64>,
) -> Result<i64, sea_orm::DbErr> {
    let Some(naive) = date.and_hms_opt(0, 0, 0) else {
        return Ok(1);
    };
    let dt = naive.and_utc();
    let (start, end) = FiscalYear::for_datetime(dt).datetime_range();
    let mut query = QuotationEntity::find()
        .filter(quotation::Column::Date.gte(start.date_naive()))
        .filter(quotation::Column::Date.lt(end.date_naive()));
    if let Some(id) = exclude_id {
        query = query.filter(quotation::Column::Id.ne(id));
    }
    let count = query.count(db).await?;
    Ok(count as i64 + 1)
}

/// Keep a typed number. A blank number is filled from the preference format.
pub async fn resolve_quotation_number(
    db: &DatabaseConnection,
    existing_id: Option<i64>,
    date: NaiveDate,
    requested: &str,
) -> Result<String, String> {
    let requested = requested.trim();
    if !requested.is_empty() {
        return Ok(requested.to_string());
    }
    let prefs = load_preferences(db).await;
    let format = prefs
        .quotation_number_format
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_QUOTATION_NUMBER_FORMAT.to_string());
    let seq = next_quotation_seq(db)
        .await
        .map_err(|err| err.to_string())?;
    let fiscal_seq = next_fiscal_quotation_seq(db, date, existing_id)
        .await
        .map_err(|err| err.to_string())?;
    let Some(naive) = date.and_hms_opt(12, 0, 0) else {
        return Err("invalid quotation date".to_string());
    };
    Ok(format_posted_invoice_number(
        &format,
        naive.and_utc(),
        seq,
        fiscal_seq,
    ))
}

pub fn today_utc() -> NaiveDate {
    Utc::now().date_naive()
}
