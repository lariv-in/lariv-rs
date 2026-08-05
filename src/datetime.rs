//! Timezone-aware datetime formatting and parsing (stored values remain UTC).

use std::sync::OnceLock;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

pub const DEFAULT_TIMEZONE: &str = "Asia/Kolkata";

/// Match Go [`components.FieldDatetime`] display format.
pub const DISPLAY_DATETIME_FMT: &str = "%a, %d %b %Y %H:%M:%S";
pub const SHORT_DATETIME_FMT: &str = "%Y-%m-%d %H:%M";
pub const SECONDS_DATETIME_FMT: &str = "%Y-%m-%d %H:%M:%S";
pub const LOCAL_INPUT_DATETIME_FMT: &str = "%Y-%m-%dT%H:%M";

pub fn parse_timezone(tz: &str) -> Tz {
    tz.trim().parse().unwrap_or_else(|_| {
        DEFAULT_TIMEZONE
            .parse()
            .unwrap_or(chrono_tz::Tz::Asia__Kolkata)
    })
}

pub fn format_datetime(dt: DateTime<Utc>, tz: &str, fmt: &str) -> String {
    dt.with_timezone(&parse_timezone(tz)).format(fmt).to_string()
}

pub fn format_datetime_display(dt: DateTime<Utc>, tz: &str) -> String {
    format_datetime(dt, tz, DISPLAY_DATETIME_FMT)
}

pub fn format_datetime_short(dt: DateTime<Utc>, tz: &str) -> String {
    format_datetime(dt, tz, SHORT_DATETIME_FMT)
}

pub fn format_datetime_seconds(dt: DateTime<Utc>, tz: &str) -> String {
    format_datetime(dt, tz, SECONDS_DATETIME_FMT)
}

pub fn format_datetime_local_input(dt: DateTime<Utc>, tz: &str) -> String {
    format_datetime(dt, tz, LOCAL_INPUT_DATETIME_FMT)
}

pub fn format_optional_datetime_short(
    dt: Option<DateTime<Utc>>,
    tz: &str,
) -> String {
    dt.map(|d| format_datetime_short(d, tz)).unwrap_or_default()
}

/// Parse a `datetime-local` value as wall time in `tz` and convert to UTC.
pub fn parse_datetime_local_input(value: &str, tz: &str) -> Option<DateTime<Utc>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let naive = NaiveDateTime::parse_from_str(trimmed, LOCAL_INPUT_DATETIME_FMT).ok()?;
    parse_timezone(tz)
        .from_local_datetime(&naive)
        .single()
        .map(|dt| dt.with_timezone(&Utc))
}

/// IANA timezone names for select widgets (sorted).
pub fn timezone_choices() -> &'static [(String, String)] {
    static CHOICES: OnceLock<Vec<(String, String)>> = OnceLock::new();
    CHOICES
        .get_or_init(|| {
            let mut rows: Vec<(String, String)> = chrono_tz::TZ_VARIANTS
                .iter()
                .map(|tz| {
                    let name = tz.to_string();
                    (name.clone(), name)
                })
                .collect();
            rows.sort_by(|a, b| a.0.cmp(&b.0));
            rows
        })
        .as_slice()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn format_datetime_uses_timezone() {
        let dt = Utc.with_ymd_and_hms(2026, 2, 8, 12, 0, 0).unwrap();
        let local = format_datetime_short(dt, "Asia/Kolkata");
        assert_eq!(local, "2026-02-08 17:30");
    }

    #[test]
    fn parse_datetime_local_input_round_trip() {
        let utc = Utc.with_ymd_and_hms(2026, 2, 8, 12, 0, 0).unwrap();
        let input = format_datetime_local_input(utc, "Asia/Kolkata");
        let parsed = parse_datetime_local_input(&input, "Asia/Kolkata").unwrap();
        assert_eq!(parsed, utc);
    }
}
