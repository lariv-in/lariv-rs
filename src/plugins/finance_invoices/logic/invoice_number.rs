//! Posted invoice number formatting (invoice_number_format.go).

use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

use crate::plugins::finance_common::fiscal_year::FiscalYear;
use crate::plugins::finance_invoices::entities::draft_invoice;
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

pub fn format_posted_invoice_number(
    format: &str,
    invoice_datetime: DateTime<Utc>,
    posted_seq: i64,
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
    Ok(format_posted_invoice_number(&format, draft.datetime, seq))
}
