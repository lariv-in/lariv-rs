//! Timezone-aware datetime display/edit helpers (stored values remain [`DateTime<Utc>`]).
//!
//! Internal storage and creation use [`DateTime<Utc>`] at full chrono precision.
//! Call [`Utc::now`] for creation timestamps — never format a display string and parse
//! it back to persist “now.”
//!
//! - [`DatetimeLocalInput`] — lossy wall-clock value for `<input type="datetime-local">`
//! - [`DatetimeLabel`] — read-only UI labels
//!
//! Neither type is a storage representation.

use std::fmt;
use std::sync::OnceLock;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

pub const DEFAULT_TIMEZONE: &str = "Asia/Kolkata";

pub fn parse_timezone(tz: &str) -> Tz {
    tz.trim().parse().unwrap_or_else(|_| {
        DEFAULT_TIMEZONE
            .parse()
            .unwrap_or(chrono_tz::Tz::Asia__Kolkata)
    })
}

fn format_in_tz(dt: DateTime<Utc>, tz: &str, fmt: &str) -> String {
    dt.with_timezone(&parse_timezone(tz)).format(fmt).to_string()
}

/// Lossy wall-clock string for HTML `datetime-local` inputs (display/edit only).
///
/// Convert from a stored [`DateTime<Utc>`] with [`Self::from_stored`], and back with
/// [`Self::to_stored`]. Sub-second precision is not preserved across that round-trip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatetimeLocalInput(String);

impl DatetimeLocalInput {
    const FMT: &'static str = "%Y-%m-%dT%H:%M:%S";
    /// Legacy minute-only submissions still accepted by [`Self::to_stored`].
    const FMT_LEGACY_MINUTE: &'static str = "%Y-%m-%dT%H:%M";

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
        let trimmed = self.0.trim();
        if trimmed.is_empty() {
            return None;
        }
        let naive = NaiveDateTime::parse_from_str(trimmed, Self::FMT)
            .ok()
            .or_else(|| NaiveDateTime::parse_from_str(trimmed, Self::FMT_LEGACY_MINUTE).ok())?;
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
    const DISPLAY: &'static str = "%a, %d %b %Y %H:%M:%S";
    const SHORT: &'static str = "%Y-%m-%d %H:%M";
    const SECONDS: &'static str = "%Y-%m-%d %H:%M:%S";

    /// Match Go `components.FieldDatetime` display format.
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
        assert_eq!(local.as_str(), "2026-02-08 17:30");
    }

    #[test]
    fn local_input_round_trip_with_seconds() {
        let utc = Utc.with_ymd_and_hms(2026, 2, 8, 12, 0, 45).unwrap();
        let input = DatetimeLocalInput::from_stored(utc, "Asia/Kolkata");
        assert!(
            input.as_str().ends_with(":45"),
            "expected seconds in input, got {}",
            input.as_str()
        );
        let parsed = input.to_stored("Asia/Kolkata").unwrap();
        assert_eq!(parsed, utc);
    }

    #[test]
    fn local_input_accepts_minute_only() {
        let parsed = DatetimeLocalInput::from_raw("2026-02-08T17:30")
            .to_stored("Asia/Kolkata")
            .unwrap();
        assert_eq!(
            parsed,
            Utc.with_ymd_and_hms(2026, 2, 8, 12, 0, 0).unwrap()
        );
    }
}
