//! Procedural fiscal years (no DB table).
//!
//! Indian financial year: 1 April (start year) through 31 March (start year + 1).

use chrono::{DateTime, Datelike, TimeZone, Utc};

/// A fiscal year derived from a calendar datetime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiscalYear {
    /// Calendar year of the April 1 start.
    pub start_year: i32,
    /// Short code for invoice numbers, e.g. `"24-25"`.
    pub code: String,
    /// Display label, e.g. `"FY 2024-25"`.
    pub label: String,
}

impl FiscalYear {
    /// Fiscal year containing `dt` (UTC date).
    pub fn for_datetime(dt: DateTime<Utc>) -> Self {
        let d = dt.date_naive();
        let start_year = if d.month() >= 4 {
            d.year()
        } else {
            d.year() - 1
        };
        Self::from_start_year(start_year)
    }

    /// Fiscal year that starts on 1 April of `start_year`.
    pub fn from_start_year(start_year: i32) -> Self {
        let y0 = start_year % 100;
        let y1 = (start_year + 1) % 100;
        Self {
            start_year,
            code: format!("{y0:02}-{y1:02}"),
            label: format!("FY {start_year}-{y1:02}"),
        }
    }

    /// Half-open UTC range `[start, end)` covering this fiscal year.
    pub fn datetime_range(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        let start = Utc
            .with_ymd_and_hms(self.start_year, 4, 1, 0, 0, 0)
            .unwrap();
        let end = Utc
            .with_ymd_and_hms(self.start_year + 1, 4, 1, 0, 0, 0)
            .unwrap();
        (start, end)
    }

    /// Options for a hub filter: `before` past years + current + `after` future years.
    pub fn options_around(now: DateTime<Utc>, before: i32, after: i32) -> Vec<Self> {
        let current = Self::for_datetime(now).start_year;
        let from = current - before;
        let to = current + after;
        (from..=to).rev().map(Self::from_start_year).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn april_starts_new_fy() {
        let dt = Utc.with_ymd_and_hms(2024, 4, 1, 0, 0, 0).unwrap();
        let fy = FiscalYear::for_datetime(dt);
        assert_eq!(fy.start_year, 2024);
        assert_eq!(fy.code, "24-25");
        assert_eq!(fy.label, "FY 2024-25");
    }

    #[test]
    fn march_belongs_to_previous_fy() {
        let dt = Utc.with_ymd_and_hms(2025, 3, 31, 12, 0, 0).unwrap();
        let fy = FiscalYear::for_datetime(dt);
        assert_eq!(fy.start_year, 2024);
        assert_eq!(fy.code, "24-25");
    }

    #[test]
    fn datetime_range_is_april_to_april() {
        let fy = FiscalYear::for_datetime(Utc.with_ymd_and_hms(2025, 3, 31, 12, 0, 0).unwrap());
        let (start, end) = fy.datetime_range();
        assert_eq!(start, Utc.with_ymd_and_hms(2024, 4, 1, 0, 0, 0).unwrap());
        assert_eq!(end, Utc.with_ymd_and_hms(2025, 4, 1, 0, 0, 0).unwrap());
    }
}
