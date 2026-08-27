//! Posted invoice number formatting (invoice_number_format.go).

use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, Statement,
};

use crate::plugins::finance_common::fiscal_year::FiscalYear;
use crate::plugins::finance_invoices::entities::draft_invoice;
use crate::plugins::finance_invoices::entities::posted_invoice::{
    self, Entity as PostedInvoiceEntity,
};
use crate::plugins::finance_invoices::logic::preferences::load_invoice_preferences;

pub async fn next_posted_invoice_seq(db: &DatabaseConnection) -> Result<i64, sea_orm::DbErr> {
    let row = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT COALESCE(MAX(id), 0) AS seq FROM posted_invoices".to_string(),
        ))
        .await?;
    let seq = row
        .and_then(|r| r.try_get::<i64>("", "seq").ok())
        .unwrap_or(0);
    Ok(seq + 1)
}

/// Next sequence among posted invoices whose invoice datetime falls in the same
/// Indian fiscal year as `invoice_datetime` (`COUNT(*) + 1`).
pub async fn next_fiscal_posted_invoice_seq(
    db: &DatabaseConnection,
    invoice_datetime: DateTime<Utc>,
) -> Result<i64, sea_orm::DbErr> {
    let (start, end) = FiscalYear::for_datetime(invoice_datetime).datetime_range();
    let count = PostedInvoiceEntity::find()
        .filter(posted_invoice::Column::Datetime.gte(start))
        .filter(posted_invoice::Column::Datetime.lt(end))
        .count(db)
        .await?;
    Ok(count as i64 + 1)
}

pub fn format_posted_invoice_number(
    format: &str,
    invoice_datetime: DateTime<Utc>,
    posted_seq: i64,
    fiscal_posted_seq: i64,
) -> String {
    let format = if format.is_empty() {
        "INV-{{YYYY}}-{{POSTED_SEQ}}"
    } else {
        format
    };
    let fiscal_code = FiscalYear::for_datetime(invoice_datetime).code;
    let yyyy = invoice_datetime.format("%Y").to_string();
    let yy = invoice_datetime.format("%y").to_string();
    format
        .replace("{{FISCAL_CODE}}", &fiscal_code)
        .replace("{{YYYY}}", &yyyy)
        .replace("{{YY}}", &yy)
        .replace("{{POSTED_SEQ}}", &posted_seq.to_string())
        .replace("{{FISCAL_POSTED_SEQ}}", &fiscal_posted_seq.to_string())
}

pub async fn posted_invoice_number(
    db: &DatabaseConnection,
    draft: &draft_invoice::Model,
) -> Result<String, String> {
    if let Some(ref n) = draft.number {
        let t = n.trim();
        if !t.is_empty() {
            return Ok(t.to_string());
        }
    }
    let prefs = load_invoice_preferences(db).await;
    let format = prefs.invoice_number_format.unwrap_or_default();
    let seq = next_posted_invoice_seq(db)
        .await
        .map_err(|e| e.to_string())?;
    let fiscal_seq = next_fiscal_posted_invoice_seq(db, draft.datetime)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format_posted_invoice_number(
        &format,
        draft.datetime,
        seq,
        fiscal_seq,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn fiscal_posted_seq_placeholder() {
        let dt = Utc.with_ymd_and_hms(2026, 2, 8, 0, 0, 0).unwrap();
        assert_eq!(
            format_posted_invoice_number("INV/{{FISCAL_CODE}}/{{FISCAL_POSTED_SEQ}}", dt, 99, 7),
            "INV/25-26/7"
        );
    }

    #[test]
    fn posted_seq_unchanged_when_fiscal_placeholder_absent() {
        let dt = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
        assert_eq!(
            format_posted_invoice_number("INV-{{YYYY}}-{{POSTED_SEQ}}", dt, 42, 1),
            "INV-2026-42"
        );
    }
}
