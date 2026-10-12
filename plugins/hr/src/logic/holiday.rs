use std::collections::HashSet;
use std::time::Duration;

use chrono::{Datelike, NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use serde_json::Value;

use crate::entities::holiday::{self, Entity as HolidayEntity};

const INDIA_CMS_URL: &str = "https://www.india.gov.in/internal/cms";
const IMPORT_PAGE_SIZE: u32 = 100;
const IMPORT_MAX_PAGES: u32 = 20;
pub const IMPORT_YEAR_MIN: i32 = 2000;
pub const IMPORT_YEAR_MAX: i32 = 2100;

pub struct HolidayInput {
    pub title: String,
    pub description: String,
    pub date: NaiveDate,
}

pub fn validate_holiday_input(input: &HolidayInput) -> Result<(), String> {
    if input.title.trim().is_empty() {
        return Err("title is required".to_string());
    }
    Ok(())
}

pub async fn create_holiday(
    db: &DatabaseConnection,
    input: HolidayInput,
) -> Result<holiday::Model, String> {
    validate_holiday_input(&input)?;
    let now = Utc::now();
    holiday::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        title: Set(input.title.trim().to_string()),
        description: Set(input.description.trim().to_string()),
        date: Set(input.date),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn update_holiday(
    db: &DatabaseConnection,
    id: i64,
    input: HolidayInput,
) -> Result<holiday::Model, String> {
    validate_holiday_input(&input)?;
    let existing = HolidayEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "holiday not found".to_string())?;
    let now = Utc::now();
    let mut am: holiday::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.title = Set(input.title.trim().to_string());
    am.description = Set(input.description.trim().to_string());
    am.date = Set(input.date);
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn delete_holiday(db: &DatabaseConnection, id: i64) -> Result<(), String> {
    HolidayEntity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndiaHoliday {
    pub title: String,
    pub description: String,
    pub date: NaiveDate,
}

pub struct CalendarPage {
    pub holidays: Vec<IndiaHoliday>,
    pub page_count: u32,
}

/// Current calendar year in India, used as the import modal default.
pub fn current_import_year() -> i32 {
    Utc::now().with_timezone(&chrono_tz::Asia::Kolkata).year()
}

pub fn parse_import_year(raw: &str) -> Result<i32, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("year is required".to_string());
    }
    let year: i32 = raw
        .parse()
        .map_err(|_| "year must be a number".to_string())?;
    if !(IMPORT_YEAR_MIN..=IMPORT_YEAR_MAX).contains(&year) {
        return Err(format!(
            "year must be between {IMPORT_YEAR_MIN} and {IMPORT_YEAR_MAX}"
        ));
    }
    Ok(year)
}

/// Holidays from `incoming` whose trimmed date and title are not already in `existing`.
///
/// Later duplicates inside `incoming` are dropped as well.
pub fn select_new_holidays(
    existing: &HashSet<(NaiveDate, String)>,
    incoming: &[IndiaHoliday],
) -> Vec<IndiaHoliday> {
    let mut seen = existing.clone();
    let mut out = Vec::new();
    for holiday in incoming {
        let title = holiday.title.trim();
        if title.is_empty() {
            continue;
        }
        if seen.insert((holiday.date, title.to_string())) {
            out.push(IndiaHoliday {
                title: title.to_string(),
                description: holiday.description.trim().to_string(),
                date: holiday.date,
            });
        }
    }
    out
}

/// Parse one India.gov CMS page.
///
/// The portal wraps a Strapi payload: `{ "data": { "data": [...], "meta": ... } }`.
/// A direct Strapi body (`data` as an array, with or without `attributes`) is accepted too.
pub fn parse_calendar_page(body: &Value) -> Result<CalendarPage, String> {
    let payload = calendar_payload(body);
    let items = payload
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| "unexpected calendar response".to_string())?;
    let mut holidays = Vec::new();
    for item in items {
        if let Some(holiday) = parse_holiday(item) {
            holidays.push(holiday);
        }
    }
    if !items.is_empty() && holidays.is_empty() {
        return Err("calendar response had no holidays".to_string());
    }
    let page_count = payload
        .pointer("/meta/pagination/pageCount")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .max(1) as u32;
    Ok(CalendarPage {
        holidays,
        page_count,
    })
}

fn calendar_payload(body: &Value) -> &Value {
    match body.get("data") {
        Some(inner) if inner.get("data").and_then(Value::as_array).is_some() => inner,
        _ => body,
    }
}

fn parse_holiday(item: &Value) -> Option<IndiaHoliday> {
    let attrs = item
        .get("attributes")
        .filter(|value| value.is_object())
        .unwrap_or(item);
    let title = attrs
        .get("title")
        .and_then(Value::as_str)
        .or_else(|| attrs.get("name").and_then(Value::as_str))
        .map(str::trim)
        .filter(|title| !title.is_empty())?;
    let date_raw = attrs.get("date").and_then(Value::as_str)?.trim();
    let date = NaiveDate::parse_from_str(date_raw, "%Y-%m-%d")
        .ok()
        .or_else(|| {
            let prefix = date_raw.get(..10)?;
            NaiveDate::parse_from_str(prefix, "%Y-%m-%d").ok()
        })?;
    Some(IndiaHoliday {
        title: title.to_string(),
        description: holiday_description(attrs),
        date,
    })
}

fn holiday_description(attrs: &Value) -> String {
    let kind = type_label(
        attrs
            .get("holiday_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let extra = attrs
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    match (kind.is_empty(), extra.is_empty()) {
        (true, true) => String::new(),
        (false, true) => kind,
        (true, false) => extra.to_string(),
        (false, false) => format!("{kind}. {extra}"),
    }
}

fn type_label(raw: &str) -> String {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return String::new();
    }
    if normalized.contains("gazetted") {
        return "Gazetted holiday".to_string();
    }
    if normalized.contains("restricted") {
        return "Restricted holiday".to_string();
    }
    raw.trim().to_string()
}

fn calendar_route(year: i32, page: u32) -> String {
    format!(
        "/api/calendar-holidays?filters[date][$gte]={year}-01-01&filters[date][$lte]={year}-12-31&pagination[page]={page}&pagination[pageSize]={IMPORT_PAGE_SIZE}"
    )
}

async fn fetch_calendar_page(year: i32, page: u32) -> Result<CalendarPage, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        )
        .build()
        .map_err(|err| format!("calendar request failed: {err}"))?;
    let response = client
        .post(INDIA_CMS_URL)
        .header("Accept", "application/json")
        .header("Origin", "https://www.india.gov.in")
        .header("Referer", "https://www.india.gov.in/calendar")
        .json(&serde_json::json!({ "route": calendar_route(year, page) }))
        .send()
        .await
        .map_err(|err| format!("calendar request failed: {err}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("calendar request failed: {status}"));
    }
    let body: Value = response
        .json()
        .await
        .map_err(|_| "calendar response was not json".to_string())?;
    parse_calendar_page(&body)
}

pub async fn fetch_india_holidays(year: i32) -> Result<Vec<IndiaHoliday>, String> {
    let mut page = 1;
    let mut holidays = Vec::new();
    loop {
        let parsed = fetch_calendar_page(year, page).await?;
        let page_count = parsed.page_count;
        holidays.extend(parsed.holidays);
        if page >= page_count || page >= IMPORT_MAX_PAGES {
            break;
        }
        page += 1;
    }
    Ok(holidays)
}

/// Insert India.gov holidays for `year`, skipping rows that already share a date and title.
pub async fn import_india_holidays(db: &DatabaseConnection, year: i32) -> Result<usize, String> {
    let incoming = fetch_india_holidays(year).await?;
    let start = NaiveDate::from_ymd_opt(year, 1, 1).ok_or_else(|| "invalid year".to_string())?;
    let end = NaiveDate::from_ymd_opt(year, 12, 31).ok_or_else(|| "invalid year".to_string())?;
    let existing = HolidayEntity::find()
        .filter(holiday::Column::Date.gte(start))
        .filter(holiday::Column::Date.lte(end))
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let existing_keys = existing
        .into_iter()
        .map(|row| (row.date, row.title.trim().to_string()))
        .collect();
    let fresh = select_new_holidays(&existing_keys, &incoming);
    let inserted = fresh.len();
    for holiday in fresh {
        create_holiday(
            db,
            HolidayInput {
                title: holiday.title,
                description: holiday.description,
                date: holiday.date,
            },
        )
        .await?;
    }
    Ok(inserted)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PORTAL_PAGE: &str = r#"{
        "data": {
            "data": [
                {
                    "id": 166,
                    "attributes": {
                        "title": "Diwali (Deepavali)",
                        "date": "2026-11-08",
                        "holiday_type": "Gazetted"
                    }
                },
                {
                    "id": 187,
                    "attributes": {
                        "title": "Raksha Bandhan",
                        "date": "2026-08-28",
                        "holiday_type": "Restricted",
                        "description": "Optional"
                    }
                }
            ],
            "meta": { "pagination": { "page": 1, "pageSize": 100, "pageCount": 2, "total": 2 } }
        }
    }"#;

    #[test]
    fn parses_portal_wrapped_strapi_page() {
        let body: Value = serde_json::from_str(PORTAL_PAGE).expect("fixture");
        let page = parse_calendar_page(&body).expect("page");
        assert_eq!(page.page_count, 2);
        assert_eq!(
            page.holidays,
            vec![
                IndiaHoliday {
                    title: "Diwali (Deepavali)".into(),
                    description: "Gazetted holiday".into(),
                    date: NaiveDate::from_ymd_opt(2026, 11, 8).unwrap(),
                },
                IndiaHoliday {
                    title: "Raksha Bandhan".into(),
                    description: "Restricted holiday. Optional".into(),
                    date: NaiveDate::from_ymd_opt(2026, 8, 28).unwrap(),
                },
            ]
        );
    }

    #[test]
    fn parses_flat_strapi_record_with_name_and_datetime() {
        let body = serde_json::json!({
            "data": [{
                "name": "Republic Day",
                "date": "2026-01-26T00:00:00.000Z",
                "holiday_type": "Gazetted"
            }],
            "meta": { "pagination": { "pageCount": 3 } }
        });
        let page = parse_calendar_page(&body).expect("page");
        assert_eq!(page.page_count, 3);
        assert_eq!(page.holidays.len(), 1);
        assert_eq!(page.holidays[0].title, "Republic Day");
        assert_eq!(
            page.holidays[0].date,
            NaiveDate::from_ymd_opt(2026, 1, 26).unwrap()
        );
        assert_eq!(page.holidays[0].description, "Gazetted holiday");
    }

    #[test]
    fn select_new_holidays_skips_existing_date_and_title() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 26).unwrap();
        let existing = HashSet::from([(date, "Republic Day".to_string())]);
        let incoming = vec![
            IndiaHoliday {
                title: " Republic Day ".into(),
                description: "Gazetted holiday".into(),
                date,
            },
            IndiaHoliday {
                title: "Republic Day".into(),
                description: "again".into(),
                date,
            },
            IndiaHoliday {
                title: "Holi".into(),
                description: "Gazetted holiday".into(),
                date: NaiveDate::from_ymd_opt(2026, 3, 4).unwrap(),
            },
        ];
        let fresh = select_new_holidays(&existing, &incoming);
        assert_eq!(fresh.len(), 1);
        assert_eq!(fresh[0].title, "Holi");
    }
}
