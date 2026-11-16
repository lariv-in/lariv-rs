//! Per-user ledger of billed Gemini tokens and the usage-page month calendar.

use std::collections::HashMap;

use chrono::{DateTime, Datelike, LocalResult, NaiveDate, TimeZone, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseBackend, DatabaseConnection, DbErr,
    EntityTrait, QueryFilter, QuerySelect,
};

use crate::entities::session::Entity as SessionEntity;
use crate::entities::token_usage::{self, Entity as TokenUsageEntity};
use crate::genai::UsageMetadata;

/// Monday-first month of token totals for one user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenUsageCalendar {
    pub year: i32,
    pub month: u32,
    pub label: String,
    pub weeks: Vec<TokenUsageWeek>,
    pub month_total: i64,
    pub lifetime_total: i64,
    pub prev_year: i32,
    pub prev_month: u32,
    pub next_year: i32,
    pub next_month: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenUsageWeek {
    pub days: [Option<TokenUsageDay>; 7],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenUsageDay {
    pub day: u32,
    pub tokens: i64,
    pub is_today: bool,
}

pub const TOKEN_USAGE_CALENDAR_ID: &str = "llm-assistant-token-usage";

/// Insert one usage row. Database errors are logged and do not fail the caller.
pub async fn record_token_usage(db: &DatabaseConnection, user_id: i64, model: &str, tokens: u32) {
    if user_id <= 0 || tokens == 0 {
        return;
    }
    let now = Utc::now();
    let row = token_usage::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        user_id: Set(user_id),
        used_at: Set(now),
        model: Set(model.trim().to_string()),
        tokens: Set(i64::from(tokens)),
    };
    if let Err(e) = row.insert(db).await {
        tracing::debug!(error = %e, user_id, tokens, "llm_assistant: record token usage failed");
    }
}

/// Attribute `usage` to the owner of `session_id`.
pub async fn record_session_usage(
    db: &DatabaseConnection,
    session_id: i64,
    model: &str,
    usage: Option<&UsageMetadata>,
) {
    let Some(tokens) = usage.and_then(UsageMetadata::billed_tokens) else {
        return;
    };
    match SessionEntity::find_by_id(session_id).one(db).await {
        Ok(Some(sess)) => record_token_usage(db, sess.user_id, model, tokens).await,
        Ok(None) => {}
        Err(e) => {
            tracing::debug!(error = %e, session_id, "llm_assistant: session owner for token usage");
        }
    }
}

/// Month grid for `user_id`. Invalid `year`/`month` fall back to today in `timezone`.
pub async fn load_token_usage_calendar(
    db: &DatabaseConnection,
    user_id: i64,
    year: Option<i32>,
    month: Option<u32>,
    timezone: &str,
) -> Result<TokenUsageCalendar, DbErr> {
    let today = today_in_timezone(timezone);
    let (year, month) = resolve_calendar_month(year, month, today);
    let (start, end) = month_bounds_utc(timezone, year, month);
    let rows = TokenUsageEntity::find()
        .filter(token_usage::Column::UserId.eq(user_id))
        .filter(token_usage::Column::UsedAt.gte(start))
        .filter(token_usage::Column::UsedAt.lt(end))
        .all(db)
        .await?;
    let tz = lariv_core::datetime::parse_timezone(timezone);
    let mut daily = HashMap::new();
    for row in rows {
        let date = row.used_at.with_timezone(&tz).date_naive();
        let entry = daily.entry(date).or_insert(0_i64);
        *entry = entry.saturating_add(row.tokens);
    }
    let lifetime_total = lifetime_tokens(db, user_id).await?;
    Ok(build_month_grid(year, month, &daily, today, lifetime_total))
}

pub fn empty_token_usage_calendar(timezone: &str) -> TokenUsageCalendar {
    let today = today_in_timezone(timezone);
    build_month_grid(today.year(), today.month(), &HashMap::new(), today, 0)
}

/// Grouped integer (`1048576` → `1,048,576`).
pub fn format_token_total(n: i64) -> String {
    let negative = n < 0;
    let digits: Vec<char> = n.unsigned_abs().to_string().chars().collect();
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    for (i, ch) in digits.iter().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*ch);
    }
    out
}

pub fn resolve_calendar_month(
    year: Option<i32>,
    month: Option<u32>,
    today: NaiveDate,
) -> (i32, u32) {
    match (year, month) {
        (Some(y), Some(m)) if (1..=12).contains(&m) && (1..=9999).contains(&y) => (y, m),
        _ => (today.year(), today.month()),
    }
}

pub fn shift_month(year: i32, month: u32, delta: i32) -> (i32, u32) {
    let month_index = i32::try_from(month).unwrap_or(1).saturating_sub(1);
    let mut idx = year
        .saturating_mul(12)
        .saturating_add(month_index)
        .saturating_add(delta);
    if idx < 0 {
        idx = 0;
    }
    let shifted_year = idx.div_euclid(12);
    let shifted_month = idx.rem_euclid(12) + 1;
    (shifted_year, u32::try_from(shifted_month).unwrap_or(1))
}

pub fn build_month_grid(
    year: i32,
    month: u32,
    daily: &HashMap<NaiveDate, i64>,
    today: NaiveDate,
    lifetime_total: i64,
) -> TokenUsageCalendar {
    let first = NaiveDate::from_ymd_opt(year, month, 1);
    let mut weeks = Vec::new();
    let mut month_total = 0_i64;
    if let Some(first) = first {
        let prefix = usize::try_from(first.weekday().num_days_from_monday()).unwrap_or(0);
        let mut cells: Vec<Option<TokenUsageDay>> = (0..prefix).map(|_| None).collect();
        let mut cursor = first;
        loop {
            if cursor.year() != year || cursor.month() != month {
                break;
            }
            let tokens = daily.get(&cursor).copied().unwrap_or(0);
            month_total = month_total.saturating_add(tokens);
            cells.push(Some(TokenUsageDay {
                day: cursor.day(),
                tokens,
                is_today: cursor == today,
            }));
            let Some(next) = cursor.succ_opt() else {
                break;
            };
            cursor = next;
        }
        while !cells.len().is_multiple_of(7) {
            cells.push(None);
        }
        for chunk in cells.chunks(7) {
            let mut days: [Option<TokenUsageDay>; 7] = std::array::from_fn(|_| None);
            for (i, day) in chunk.iter().enumerate() {
                if let Some(slot) = days.get_mut(i) {
                    *slot = day.clone();
                }
            }
            weeks.push(TokenUsageWeek { days });
        }
    }
    let (prev_year, prev_month) = shift_month(year, month, -1);
    let (next_year, next_month) = shift_month(year, month, 1);
    let label = first
        .map(|d| d.format("%B %Y").to_string())
        .unwrap_or_else(|| format!("{month:02}/{year}"));
    TokenUsageCalendar {
        year,
        month,
        label,
        weeks,
        month_total,
        lifetime_total,
        prev_year,
        prev_month,
        next_year,
        next_month,
    }
}

fn today_in_timezone(timezone: &str) -> NaiveDate {
    Utc::now()
        .with_timezone(&lariv_core::datetime::parse_timezone(timezone))
        .date_naive()
}

fn month_bounds_utc(timezone: &str, year: i32, month: u32) -> (DateTime<Utc>, DateTime<Utc>) {
    let tz = lariv_core::datetime::parse_timezone(timezone);
    let start = local_midnight_utc(&tz, year, month);
    let (next_year, next_month) = shift_month(year, month, 1);
    let end = local_midnight_utc(&tz, next_year, next_month);
    (start, end)
}

fn local_midnight_utc(tz: &chrono_tz::Tz, year: i32, month: u32) -> DateTime<Utc> {
    let Some(naive) = NaiveDate::from_ymd_opt(year, month, 1).and_then(|d| d.and_hms_opt(0, 0, 0))
    else {
        return DateTime::<Utc>::UNIX_EPOCH;
    };
    match tz.from_local_datetime(&naive) {
        LocalResult::Single(dt) => dt.with_timezone(&Utc),
        LocalResult::Ambiguous(earliest, _) => earliest.with_timezone(&Utc),
        LocalResult::None => {
            let shifted = naive + chrono::Duration::hours(1);
            match tz.from_local_datetime(&shifted) {
                LocalResult::Single(dt) | LocalResult::Ambiguous(dt, _) => dt.with_timezone(&Utc),
                LocalResult::None => Utc.from_utc_datetime(&naive),
            }
        }
    }
}

async fn lifetime_tokens(db: &DatabaseConnection, user_id: i64) -> Result<i64, DbErr> {
    let sql = match db.get_database_backend() {
        DatabaseBackend::Postgres => "CAST(COALESCE(SUM(tokens), 0) AS BIGINT)",
        _ => "CAST(COALESCE(SUM(tokens), 0) AS INTEGER)",
    };
    let total = TokenUsageEntity::find()
        .filter(token_usage::Column::UserId.eq(user_id))
        .select_only()
        .column_as(sea_orm::sea_query::Expr::cust(sql), "total")
        .into_tuple::<i64>()
        .one(db)
        .await?;
    Ok(total.unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::{Datelike, NaiveDate, Weekday};

    use super::{build_month_grid, format_token_total, resolve_calendar_month, shift_month};

    #[test]
    fn format_token_total_groups_thousands() {
        assert_eq!(format_token_total(0), "0");
        assert_eq!(format_token_total(842), "842");
        assert_eq!(format_token_total(1_048_576), "1,048,576");
        assert_eq!(format_token_total(-1200), "-1,200");
    }

    #[test]
    fn month_grid_aligns_monday_and_keeps_lifetime_separate() {
        let nov1 = NaiveDate::from_ymd_opt(2026, 11, 1).expect("date");
        assert_eq!(nov1.weekday(), Weekday::Sun);
        let nov2 = NaiveDate::from_ymd_opt(2026, 11, 2).expect("date");
        let mut daily = HashMap::new();
        daily.insert(nov1, 100);
        daily.insert(nov2, 50);
        let grid = build_month_grid(2026, 11, &daily, nov2, 999);
        assert_eq!(grid.label, "November 2026");
        assert_eq!(grid.month_total, 150);
        assert_eq!(grid.lifetime_total, 999);
        assert_ne!(grid.month_total, grid.lifetime_total);
        let first = grid.weeks.first().expect("first week");
        assert!(first.days.first().and_then(|d| d.as_ref()).is_none());
        let sunday = first.days.get(6).and_then(|d| d.as_ref()).expect("1 Nov");
        assert_eq!(sunday.day, 1);
        assert_eq!(sunday.tokens, 100);
        assert!(!sunday.is_today);
        let second = grid.weeks.get(1).expect("second week");
        let monday = second.days.first().and_then(|d| d.as_ref()).expect("2 Nov");
        assert_eq!(monday.day, 2);
        assert_eq!(monday.tokens, 50);
        assert!(monday.is_today);
        assert_eq!(shift_month(2026, 1, -1), (2025, 12));
        assert_eq!(
            resolve_calendar_month(Some(0), Some(13), nov1),
            (nov1.year(), nov1.month())
        );
        assert_eq!(resolve_calendar_month(Some(2024), Some(2), nov1), (2024, 2));
    }
}
