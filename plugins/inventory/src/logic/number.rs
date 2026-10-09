//! Stock movement number formatting.

use chrono::{DateTime, Datelike, TimeZone, Utc};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};

use crate::entities::stock_movement::{self, Entity as MovementEntity};
use crate::logic::preferences::load_preferences;
use crate::movement_type::MovementType;

pub const DEFAULT_MOVEMENT_NUMBER_FORMAT: &str = "SM-{{YYYY}}-{{SEQ}}";

pub fn format_movement_number(
    format: &str,
    datetime: DateTime<Utc>,
    seq: i64,
    fiscal_seq: i64,
    movement_type: MovementType,
) -> String {
    let format = if format.trim().is_empty() {
        DEFAULT_MOVEMENT_NUMBER_FORMAT
    } else {
        format
    };
    let fiscal = fiscal_bounds(datetime);
    let yyyy = datetime.format("%Y").to_string();
    let yy = datetime.format("%y").to_string();
    let kind = match movement_type {
        MovementType::In => "IN",
        MovementType::Out => "OUT",
    };
    format
        .replace("{{FISCAL_CODE}}", &fiscal.code)
        .replace("{{YYYY}}", &yyyy)
        .replace("{{YY}}", &yy)
        .replace("{{SEQ}}", &seq.to_string())
        .replace("{{FISCAL_SEQ}}", &fiscal_seq.to_string())
        .replace("{{TYPE}}", kind)
}

pub async fn next_movement_number(
    db: &DatabaseConnection,
    datetime: DateTime<Utc>,
    movement_type: MovementType,
    requested: &str,
) -> Result<String, String> {
    let trimmed = requested.trim();
    if !trimmed.is_empty() {
        return Ok(trimmed.to_string());
    }
    let prefs = load_preferences(db).await?;
    let format = prefs.movement_number_format.unwrap_or_default();
    let seq = MovementEntity::find()
        .count(db)
        .await
        .map_err(|err| err.to_string())? as i64
        + 1;
    let (start, end) = fiscal_bounds(datetime).range;
    let fiscal_seq = MovementEntity::find()
        .filter(stock_movement::Column::Datetime.gte(start))
        .filter(stock_movement::Column::Datetime.lt(end))
        .count(db)
        .await
        .map_err(|err| err.to_string())? as i64
        + 1;
    Ok(format_movement_number(
        &format,
        datetime,
        seq,
        fiscal_seq,
        movement_type,
    ))
}

struct FiscalBounds {
    code: String,
    range: (DateTime<Utc>, DateTime<Utc>),
}

fn fiscal_bounds(dt: DateTime<Utc>) -> FiscalBounds {
    let date = dt.date_naive();
    let start_year = if date.month() >= 4 {
        date.year()
    } else {
        date.year() - 1
    };
    let code = format!("{:02}-{:02}", start_year % 100, (start_year + 1) % 100);
    let start = Utc
        .with_ymd_and_hms(start_year, 4, 1, 0, 0, 0)
        .single()
        .unwrap_or(dt);
    let end = Utc
        .with_ymd_and_hms(start_year + 1, 4, 1, 0, 0, 0)
        .single()
        .unwrap_or(dt);
    FiscalBounds {
        code,
        range: (start, end),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn fiscal_and_type_placeholders() {
        let dt = Utc.with_ymd_and_hms(2026, 2, 8, 0, 0, 0).unwrap();
        assert_eq!(
            format_movement_number(
                "SM/{{FISCAL_CODE}}/{{TYPE}}/{{FISCAL_SEQ}}",
                dt,
                99,
                7,
                MovementType::Out,
            ),
            "SM/25-26/OUT/7"
        );
    }

    #[test]
    fn default_format_uses_year_and_seq() {
        let dt = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
        assert_eq!(
            format_movement_number("", dt, 42, 1, MovementType::In),
            "SM-2026-42"
        );
    }
}
