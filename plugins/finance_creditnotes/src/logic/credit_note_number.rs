//! Credit note number formatting. Same placeholders as payment numbers.

use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait, PaginatorTrait, QueryFilter,
    Statement,
};

use crate::entities::credit_note::{self, Entity as CreditNoteEntity};
use crate::logic::preferences::load_credit_note_preferences;
use lariv_plugin_finance_common::fiscal_year::FiscalYear;

/// Used when Credit note number format is missing or blank.
pub const DEFAULT_CREDIT_NOTE_NUMBER_FORMAT: &str = "CN/{{FISCAL_CODE}}/{{POSTED_SEQ}}";

pub fn format_credit_note_number(
    format: &str,
    note_datetime: DateTime<Utc>,
    posted_seq: i64,
    fiscal_posted_seq: i64,
) -> String {
    let format = if format.trim().is_empty() {
        DEFAULT_CREDIT_NOTE_NUMBER_FORMAT
    } else {
        format
    };
    let fiscal_code = FiscalYear::for_datetime(note_datetime).code;
    let yyyy = note_datetime.format("%Y").to_string();
    let yy = note_datetime.format("%y").to_string();
    format
        .replace("{{FISCAL_CODE}}", &fiscal_code)
        .replace("{{YYYY}}", &yyyy)
        .replace("{{YY}}", &yy)
        .replace("{{POSTED_SEQ}}", &posted_seq.to_string())
        .replace("{{FISCAL_POSTED_SEQ}}", &fiscal_posted_seq.to_string())
}

pub async fn next_credit_note_seq<C: ConnectionTrait>(db: &C) -> Result<i64, sea_orm::DbErr> {
    let row = db
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT COALESCE(MAX(id), 0) AS seq FROM credit_notes".to_string(),
        ))
        .await?;
    let seq = row
        .as_ref()
        .and_then(|r| r.try_get::<i64>("", "seq").ok())
        .unwrap_or(0);
    Ok(seq + 1)
}

pub async fn next_fiscal_credit_note_seq<C: ConnectionTrait>(
    db: &C,
    note_datetime: DateTime<Utc>,
) -> Result<i64, sea_orm::DbErr> {
    let (start, end) = FiscalYear::for_datetime(note_datetime).datetime_range();
    let count = CreditNoteEntity::find()
        .filter(credit_note::Column::Datetime.gte(start))
        .filter(credit_note::Column::Datetime.lt(end))
        .count(db)
        .await?;
    Ok(count as i64 + 1)
}

pub async fn generate_credit_note_number<C: ConnectionTrait>(
    db: &C,
    note_datetime: DateTime<Utc>,
) -> Result<String, String> {
    let prefs = load_credit_note_preferences(db).await;
    let format = prefs.credit_note_number_format.unwrap_or_default();
    let seq = next_credit_note_seq(db).await.map_err(|e| e.to_string())?;
    let fiscal_seq = next_fiscal_credit_note_seq(db, note_datetime)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format_credit_note_number(
        &format,
        note_datetime,
        seq,
        fiscal_seq,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn blank_format_uses_fallback() {
        let dt = Utc
            .with_ymd_and_hms(2026, 4, 1, 0, 0, 0)
            .single()
            .expect("valid datetime");
        assert_eq!(format_credit_note_number("", dt, 42, 1), "CN/26-27/42");
        assert_eq!(format_credit_note_number("   ", dt, 42, 1), "CN/26-27/42");
    }

    #[test]
    fn fiscal_posted_seq_placeholder() {
        let dt = Utc
            .with_ymd_and_hms(2026, 2, 8, 0, 0, 0)
            .single()
            .expect("valid datetime");
        assert_eq!(
            format_credit_note_number("CN/{{FISCAL_CODE}}/{{FISCAL_POSTED_SEQ}}", dt, 99, 7),
            "CN/25-26/7"
        );
    }
}
