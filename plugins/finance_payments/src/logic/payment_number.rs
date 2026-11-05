//! Payment number formatting. Same placeholders as posted invoice numbers.

use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait, PaginatorTrait, QueryFilter,
    Statement,
};

use crate::entities::payment::{self, Entity as PaymentEntity};
use crate::logic::preferences::load_payment_preferences;
use lariv_plugin_finance_common::fiscal_year::FiscalYear;

/// Used when Payment number format is missing or blank.
pub const DEFAULT_PAYMENT_NUMBER_FORMAT: &str = "PAY/{{FISCAL_CODE}}/{{POSTED_SEQ}}";

pub fn format_payment_number(
    format: &str,
    payment_datetime: DateTime<Utc>,
    posted_seq: i64,
    fiscal_posted_seq: i64,
) -> String {
    let format = if format.trim().is_empty() {
        DEFAULT_PAYMENT_NUMBER_FORMAT
    } else {
        format
    };
    let fiscal_code = FiscalYear::for_datetime(payment_datetime).code;
    let yyyy = payment_datetime.format("%Y").to_string();
    let yy = payment_datetime.format("%y").to_string();
    format
        .replace("{{FISCAL_CODE}}", &fiscal_code)
        .replace("{{YYYY}}", &yyyy)
        .replace("{{YY}}", &yy)
        .replace("{{POSTED_SEQ}}", &posted_seq.to_string())
        .replace("{{FISCAL_POSTED_SEQ}}", &fiscal_posted_seq.to_string())
}

pub async fn next_payment_seq<C: ConnectionTrait>(db: &C) -> Result<i64, sea_orm::DbErr> {
    let row = db
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT COALESCE(MAX(id), 0) AS seq FROM payments".to_string(),
        ))
        .await?;
    let seq = row
        .and_then(|r| r.try_get::<i64>("", "seq").ok())
        .unwrap_or(0);
    Ok(seq + 1)
}

/// Next sequence among payments whose datetime falls in the same Indian fiscal
/// year as `payment_datetime` (`COUNT(*) + 1`).
pub async fn next_fiscal_payment_seq<C: ConnectionTrait>(
    db: &C,
    payment_datetime: DateTime<Utc>,
) -> Result<i64, sea_orm::DbErr> {
    let (start, end) = FiscalYear::for_datetime(payment_datetime).datetime_range();
    let count = PaymentEntity::find()
        .filter(payment::Column::Datetime.gte(start))
        .filter(payment::Column::Datetime.lt(end))
        .count(db)
        .await?;
    Ok(count as i64 + 1)
}

/// Explicit number when one was typed; otherwise the payment number format.
pub async fn resolve_payment_number<C: ConnectionTrait>(
    db: &C,
    explicit: Option<&str>,
    payment_datetime: DateTime<Utc>,
) -> Result<String, String> {
    let number = match explicit.map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => n.to_string(),
        None => generate_payment_number(db, payment_datetime).await?,
    };
    let existing = PaymentEntity::find()
        .filter(payment::Column::Number.eq(&number))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if existing > 0 {
        return Err(format!("payment number {number} is already used"));
    }
    Ok(number)
}

async fn generate_payment_number<C: ConnectionTrait>(
    db: &C,
    payment_datetime: DateTime<Utc>,
) -> Result<String, String> {
    let prefs = load_payment_preferences(db).await;
    let format = prefs.payment_number_format.unwrap_or_default();
    let seq = next_payment_seq(db).await.map_err(|e| e.to_string())?;
    let fiscal_seq = next_fiscal_payment_seq(db, payment_datetime)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format_payment_number(
        &format,
        payment_datetime,
        seq,
        fiscal_seq,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn fiscal_code_and_posted_seq() {
        let dt = Utc
            .with_ymd_and_hms(2026, 2, 8, 0, 0, 0)
            .single()
            .expect("valid datetime");
        assert_eq!(
            format_payment_number("PAY/{{FISCAL_CODE}}/{{POSTED_SEQ}}", dt, 42, 7),
            "PAY/25-26/42"
        );
    }

    #[test]
    fn blank_format_uses_fallback() {
        let dt = Utc
            .with_ymd_and_hms(2026, 4, 1, 0, 0, 0)
            .single()
            .expect("valid datetime");
        assert_eq!(format_payment_number("", dt, 42, 1), "PAY/26-27/42");
        assert_eq!(format_payment_number("   ", dt, 42, 1), "PAY/26-27/42");
    }

    #[test]
    fn fiscal_posted_seq_placeholder() {
        let dt = Utc
            .with_ymd_and_hms(2026, 2, 8, 0, 0, 0)
            .single()
            .expect("valid datetime");
        assert_eq!(
            format_payment_number("PAY/{{FISCAL_CODE}}/{{FISCAL_POSTED_SEQ}}", dt, 99, 7),
            "PAY/25-26/7"
        );
    }
}
