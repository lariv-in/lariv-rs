//! Procedural fiscal years (no DB table).
//!
//! Indian financial year: 1 April (start year) through 31 March (start year + 1).

use chrono::{DateTime, Datelike, Utc};

/// A fiscal year derived from a calendar datetime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiscalYear {
    /// Calendar year of the April 1 start.
    pub start_year: i32,
    /// Short code for invoice numbers, e.g. `"24-25"`.
    pub code: String,
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
        let y0 = start_year % 100;
        let y1 = (start_year + 1) % 100;
        Self {
            start_year,
            code: format!("{y0:02}-{y1:02}"),
        }
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
    }

    #[test]
    fn march_belongs_to_previous_fy() {
        let dt = Utc.with_ymd_and_hms(2025, 3, 31, 12, 0, 0).unwrap();
        let fy = FiscalYear::for_datetime(dt);
        assert_eq!(fy.start_year, 2024);
        assert_eq!(fy.code, "24-25");
    }
}
