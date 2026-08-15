//! Timezone-aware datetime display/edit helpers (stored values remain [`DateTime<Utc>`]).
//!
//! Internal storage and creation use [`DateTime<Utc>`] at full chrono precision.
//! Call [`Utc::now`] for creation timestamps — never format a display string and parse
//! it back to persist “now.”
//!
//! On-screen dates and text inputs use day-first `DD/MM/YYYY`. Native HTML
//! `type="date"` / `datetime-local` pickers are overlaid (they require ISO
//! `YYYY-MM-DD` as their value) so the calendar still opens.
//!
//! - [`DatetimeLocalInput`] — lossy wall-clock value for datetime text inputs
//! - [`DatetimeLabel`] — read-only UI labels
//!
//! Neither type is a storage representation.

use std::fmt;
use std::sync::OnceLock;

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

pub const DEFAULT_TIMEZONE: &str = "Asia/Kolkata";

/// On-screen calendar date (`DD/MM/YYYY`).
pub const DATE_FMT: &str = "%d/%m/%Y";
/// On-screen datetime with seconds (`DD/MM/YYYY HH:MM:SS`).
pub const DATETIME_SECONDS_FMT: &str = "%d/%m/%Y %H:%M:%S";
/// On-screen datetime without seconds (`DD/MM/YYYY HH:MM`).
pub const DATETIME_SHORT_FMT: &str = "%d/%m/%Y %H:%M";

pub fn parse_timezone(tz: &str) -> Tz {
    tz.trim().parse().unwrap_or_else(|_| {
        DEFAULT_TIMEZONE
            .parse()
            .unwrap_or(chrono_tz::Tz::Asia__Kolkata)
    })
}

fn format_in_tz(dt: DateTime<Utc>, tz: &str, fmt: &str) -> String {
    dt.with_timezone(&parse_timezone(tz))
        .format(fmt)
        .to_string()
}

/// Format a calendar date for labels and text inputs.
pub fn format_date(d: NaiveDate) -> String {
    d.format(DATE_FMT).to_string()
}

/// Parse a calendar date from a text input.
///
/// Prefers `DD/MM/YYYY`; also accepts ISO `YYYY-MM-DD` and `DD-MM-YYYY`.
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    NaiveDate::parse_from_str(s, DATE_FMT)
        .ok()
        .or_else(|| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .or_else(|| NaiveDate::parse_from_str(s, "%d-%m-%Y").ok())
}

/// Format the calendar date of `dt` in `tz` as `DD/MM/YYYY`.
pub fn format_date_in_tz(dt: DateTime<Utc>, tz: &str) -> String {
    format_in_tz(dt, tz, DATE_FMT)
}

/// Parse a wall-clock datetime from a text input or a legacy ISO value.
pub fn parse_naive_datetime(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    const FMTS: &[&str] = &[
        DATETIME_SECONDS_FMT,
        DATETIME_SHORT_FMT,
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
    ];
    for fmt in FMTS {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(ndt);
        }
    }
    None
}

/// Interpret a date-only string as start-of-day in `tz` and convert to UTC.
pub fn parse_date_start_in_tz(s: &str, tz: &str) -> Option<DateTime<Utc>> {
    let date = parse_date(s)?;
    let naive = date.and_hms_opt(0, 0, 0)?;
    parse_timezone(tz)
        .from_local_datetime(&naive)
        .single()
        .map(|dt| dt.with_timezone(&Utc))
}

/// ISO `YYYY-MM-DD` for a native `type="date"` picker, from a display string.
pub fn date_iso_for_picker(s: &str) -> String {
    parse_date(s)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

/// ISO `YYYY-MM-DDTHH:MM:SS` for a native `datetime-local` picker.
pub fn datetime_iso_for_picker(s: &str) -> String {
    if let Some(dt) = parse_naive_datetime(s) {
        return dt.format("%Y-%m-%dT%H:%M:%S").to_string();
    }
    parse_date(s)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
        .unwrap_or_default()
}

/// Lossy wall-clock string for datetime text inputs (display/edit only).
///
/// Convert from a stored [`DateTime<Utc>`] with [`Self::from_stored`], and back with
/// [`Self::to_stored`]. Sub-second precision is not preserved across that round-trip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatetimeLocalInput(String);

impl DatetimeLocalInput {
    const FMT: &'static str = DATETIME_SECONDS_FMT;

    /// Build an input value from a stored UTC instant (for prefilling controls).
    pub fn from_stored(dt: DateTime<Utc>, tz: &str) -> Self {
        Self(format_in_tz(dt, tz, Self::FMT))
    }

    /// Wrap a raw form/query string (not validated until [`Self::to_stored`]).
    pub fn from_raw(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Parse this control value as wall time in `tz` and convert to UTC.
    pub fn to_stored(&self, tz: &str) -> Option<DateTime<Utc>> {
        let naive = parse_naive_datetime(self.0.trim())?;
        parse_timezone(tz)
            .from_local_datetime(&naive)
            .single()
            .map(|dt| dt.with_timezone(&Utc))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for DatetimeLocalInput {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for DatetimeLocalInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<DatetimeLocalInput> for String {
    fn from(value: DatetimeLocalInput) -> Self {
        value.into_string()
    }
}

/// Read-only formatted datetime for UI labels (not for round-trip storage).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatetimeLabel(String);

impl DatetimeLabel {
    const DISPLAY: &'static str = DATETIME_SECONDS_FMT;
    const SHORT: &'static str = DATETIME_SHORT_FMT;
    const SECONDS: &'static str = DATETIME_SECONDS_FMT;

    /// Day-first datetime label (`DD/MM/YYYY HH:MM:SS`).
    pub fn display(dt: DateTime<Utc>, tz: &str) -> Self {
        Self(format_in_tz(dt, tz, Self::DISPLAY))
    }

    pub fn short(dt: DateTime<Utc>, tz: &str) -> Self {
        Self(format_in_tz(dt, tz, Self::SHORT))
    }

    pub fn seconds(dt: DateTime<Utc>, tz: &str) -> Self {
        Self(format_in_tz(dt, tz, Self::SECONDS))
    }

    pub fn short_optional(dt: Option<DateTime<Utc>>, tz: &str) -> Self {
        match dt {
            Some(d) => Self::short(d, tz),
            None => Self(String::new()),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for DatetimeLabel {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for DatetimeLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<DatetimeLabel> for String {
    fn from(value: DatetimeLabel) -> Self {
        value.into_string()
    }
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
    fn label_short_uses_timezone() {
        let dt = Utc.with_ymd_and_hms(2026, 2, 8, 12, 0, 0).unwrap();
        let local = DatetimeLabel::short(dt, "Asia/Kolkata");
        assert_eq!(local.as_str(), "08/02/2026 17:30");
    }

    #[test]
    fn format_date_is_day_first() {
        let d = NaiveDate::from_ymd_opt(2026, 2, 8).unwrap();
        assert_eq!(format_date(d), "08/02/2026");
        assert_eq!(parse_date("08/02/2026"), Some(d));
        assert_eq!(parse_date("2026-02-08"), Some(d));
    }

    #[test]
    fn local_input_round_trip_with_seconds() {
        let utc = Utc.with_ymd_and_hms(2026, 2, 8, 12, 0, 45).unwrap();
        let input = DatetimeLocalInput::from_stored(utc, "Asia/Kolkata");
        assert_eq!(input.as_str(), "08/02/2026 17:30:45");
        let parsed = input.to_stored("Asia/Kolkata").unwrap();
        assert_eq!(parsed, utc);
    }

    #[test]
    fn local_input_accepts_minute_only() {
        let parsed = DatetimeLocalInput::from_raw("08/02/2026 17:30")
            .to_stored("Asia/Kolkata")
            .unwrap();
        assert_eq!(parsed, Utc.with_ymd_and_hms(2026, 2, 8, 12, 0, 0).unwrap());
    }

    #[test]
    fn local_input_accepts_legacy_iso() {
        let parsed = DatetimeLocalInput::from_raw("2026-02-08T17:30")
            .to_stored("Asia/Kolkata")
            .unwrap();
        assert_eq!(parsed, Utc.with_ymd_and_hms(2026, 2, 8, 12, 0, 0).unwrap());
    }

    #[test]
    fn picker_iso_from_display() {
        assert_eq!(date_iso_for_picker("08/02/2026"), "2026-02-08");
        assert_eq!(
            datetime_iso_for_picker("08/02/2026 17:30:45"),
            "2026-02-08T17:30:45"
        );
    }
}
