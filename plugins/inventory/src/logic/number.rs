//! Stock movement number formatting.

use chrono::{DateTime, Datelike, TimeZone, Utc};
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};

use crate::entities::stock_movement_in::{self, Entity as InEntity};
use crate::entities::stock_movement_out::{self, Entity as OutEntity};
use crate::entities::stock_movement_process::{self, Entity as ProcessEntity};
use crate::logic::preferences::load_preferences;
use crate::movement_type::MovementKind;

pub const DEFAULT_MOVEMENT_NUMBER_FORMAT: &str = "SM-{{YYYY}}-{{SEQ}}";

pub fn format_movement_number(
    format: &str,
    datetime: DateTime<Utc>,
    seq: i64,
    fiscal_seq: i64,
    kind: MovementKind,
) -> String {
    let format = if format.trim().is_empty() {
        DEFAULT_MOVEMENT_NUMBER_FORMAT
    } else {
        format
    };
    let fiscal = fiscal_bounds(datetime);
    let yyyy = datetime.format("%Y").to_string();
    let yy = datetime.format("%y").to_string();
    format
        .replace("{{FISCAL_CODE}}", &fiscal.code)
        .replace("{{YYYY}}", &yyyy)
        .replace("{{YY}}", &yy)
        .replace("{{SEQ}}", &seq.to_string())
        .replace("{{FISCAL_SEQ}}", &fiscal_seq.to_string())
        .replace("{{TYPE}}", kind.type_token())
}

pub async fn next_movement_number(
    db: &DatabaseConnection,
    datetime: DateTime<Utc>,
    kind: MovementKind,
    requested: &str,
) -> Result<String, String> {
    let trimmed = requested.trim();
    if !trimmed.is_empty() {
        return Ok(trimmed.to_string());
    }
    let prefs = load_preferences(db).await?;
    let format = prefs.movement_number_format.unwrap_or_default();
    let seq = movement_count(db, None).await? + 1;
    let (start, end) = fiscal_bounds(datetime).range;
    let fiscal_seq = movement_count(db, Some((start, end))).await? + 1;
    Ok(format_movement_number(
        &format, datetime, seq, fiscal_seq, kind,
    ))
}

async fn movement_count(
    db: &DatabaseConnection,
    range: Option<(DateTime<Utc>, DateTime<Utc>)>,
) -> Result<i64, String> {
    let in_count = count_range(
        InEntity::find(),
        stock_movement_in::Column::Datetime,
        db,
        range,
    )
    .await?;
    let out_count = count_range(
        OutEntity::find(),
        stock_movement_out::Column::Datetime,
        db,
        range,
    )
    .await?;
    let process_count = count_range(
        ProcessEntity::find(),
        stock_movement_process::Column::Datetime,
        db,
        range,
    )
    .await?;
    Ok(in_count + out_count + process_count)
}

async fn count_range<E, C>(
    mut query: sea_orm::Select<E>,
    column: C,
    db: &DatabaseConnection,
    range: Option<(DateTime<Utc>, DateTime<Utc>)>,
) -> Result<i64, String>
where
    E: EntityTrait,
    E::Model: Sync,
    C: sea_orm::ColumnTrait,
{
    if let Some((start, end)) = range {
        query = query.filter(column.gte(start)).filter(column.lt(end));
    }
    let count = query.count(db).await.map_err(|err| err.to_string())?;
    Ok(count as i64)
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
                MovementKind::Out,
            ),
            "SM/25-26/OUT/7"
        );
        assert_eq!(
            format_movement_number("SM/{{TYPE}}/{{SEQ}}", dt, 3, 1, MovementKind::Process,),
            "SM/PROCESS/3"
        );
    }

    #[test]
    fn default_format_uses_year_and_seq() {
        let dt = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
        assert_eq!(
            format_movement_number("", dt, 42, 1, MovementKind::In),
            "SM-2026-42"
        );
    }
}
