//! Duration parsing and formatting for storage as nanoseconds (`i64`).
//!
//! Accepts flexible human input (`"2 months 3 days 5 seconds"`, `"2 months, 3 days"`)
//! and compact Go-style strings (`"720h"`, `"30m"`, `"2h45m"`).

const NS: i64 = 1;
const US: i64 = 1_000;
const MS: i64 = 1_000_000;
const SEC: i64 = 1_000_000_000;
const MIN: i64 = 60 * SEC;
const HOUR: i64 = 60 * MIN;
const DAY: i64 = 24 * HOUR;
const WEEK: i64 = 7 * DAY;

/// `(unit names longest-first, nanoseconds per unit)`
const UNIT_WORDS: &[(&[&str], i64)] = &[
    (&["nanoseconds", "nanosecond"], NS),
    (&["microseconds", "microsecond", "µs"], US),
    (&["us"], US),
    (&["milliseconds", "millisecond"], MS),
    (&["ms"], MS),
    (&["seconds", "second", "secs", "sec"], SEC),
    (&["minutes", "minute", "mins", "min"], MIN),
    (&["hours", "hour", "hrs", "hr"], HOUR),
    (&["weeks", "week", "wks", "wk"], WEEK),
    (&["days", "day"], DAY),
    // Single-letter Go units last so `months` beats `m`, `hours` beats `h`, etc.
    (&["s"], SEC),
    (&["m"], MIN),
    (&["h"], HOUR),
    (&["d"], DAY),
    (&["w"], WEEK),
    (&["ns"], NS),
];

/// Largest units first for human-readable formatting.
const FORMAT_UNITS: &[(&str, &str, i64)] = &[
    ("week", "weeks", WEEK),
    ("day", "days", DAY),
    ("hour", "hours", HOUR),
    ("minute", "minutes", MIN),
    ("second", "seconds", SEC),
];

/// Parse a duration string into nanoseconds.
///
/// Examples: `"2 months 3 days 5 seconds"`, `"2 months, 3 days"`, `"720h"`, `"30m"`.
pub fn parse_duration(s: &str) -> Result<i64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("duration is required".to_string());
    }

    let (negative, rest) = if let Some(stripped) = s.strip_prefix('-') {
        (true, stripped.trim())
    } else {
        (false, s)
    };
    if rest.is_empty() {
        return Err("invalid duration".to_string());
    }

    let normalized: String = rest
        .chars()
        .map(|c| if c == ',' { ' ' } else { c })
        .collect();

    let mut total: i64 = 0;
    let mut i = 0;
    let bytes = normalized.as_bytes();

    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
            i += 1;
        }
        if i == start {
            return Err("invalid duration".to_string());
        }
        let number: f64 = normalized[start..i]
            .parse()
            .map_err(|_| "invalid duration".to_string())?;

        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            return Err("invalid duration".to_string());
        }

        let (unit_nanos, consumed) = match_unit_word(&normalized[i..])?;
        i += consumed;

        let nanos = (number * unit_nanos as f64).round() as i64;
        total = total
            .checked_add(nanos)
            .ok_or_else(|| "duration overflow".to_string())?;
    }

    if negative {
        total = -total;
    }
    if total <= 0 {
        return Err("duration must be positive".to_string());
    }
    Ok(total)
}

/// Alias for [`parse_duration`] (Go-style strings remain supported).
pub fn parse_go_duration(s: &str) -> Result<i64, String> {
    parse_duration(s)
}

/// Format nanoseconds as a human-readable string (e.g. `"2 months 3 days 5 seconds"`).
pub fn format_duration(nanos: i64) -> String {
    if nanos <= 0 {
        return String::new();
    }
    let mut rem = nanos;
    let mut parts = Vec::new();
    for (singular, plural, unit) in FORMAT_UNITS {
        if rem >= *unit {
            let count = rem / unit;
            rem %= unit;
            let label = if count == 1 { singular } else { plural };
            parts.push(format!("{count} {label}"));
        }
    }
    if parts.is_empty() {
        return format!("{nanos}ns");
    }
    parts.join(" ")
}

/// Compact Go-style formatting (e.g. `"720h"`, `"30m"`).
pub fn format_go_duration(nanos: i64) -> String {
    if nanos <= 0 {
        return String::new();
    }
    let mut rem = nanos;
    let mut parts = Vec::new();
    for (label, unit) in [
        ("h", HOUR),
        ("m", MIN),
        ("s", SEC),
        ("ms", MS),
        ("us", US),
        ("ns", NS),
    ] {
        if rem >= unit {
            let count = rem / unit;
            rem %= unit;
            parts.push(format!("{count}{label}"));
        }
    }
    parts.join("")
}

/// Normalize a user-entered duration for re-display in form fields.
pub fn format_duration_input(value: &str, nanos: i64) -> String {
    if !value.trim().is_empty() {
        return value.trim().to_string();
    }
    format_duration(nanos)
}

fn match_unit_word(s: &str) -> Result<(i64, usize), String> {
    let lower = s.to_lowercase();
    for (names, nanos) in UNIT_WORDS {
        for name in *names {
            if lower.starts_with(name) {
                let len = name.len();
                let at_boundary = lower.len() == len
                    || !lower
                        .as_bytes()
                        .get(len)
                        .is_some_and(|b| b.is_ascii_alphabetic());
                if at_boundary {
                    return Ok((*nanos, len));
                }
            }
        }
    }
    Err("invalid duration unit".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_go_style() {
        assert_eq!(parse_duration("720h").unwrap(), 720 * HOUR);
        assert_eq!(parse_duration("30m").unwrap(), 30 * MIN);
        assert_eq!(parse_duration("2h45m").unwrap(), 2 * HOUR + 45 * MIN);
        assert_eq!(
            parse_duration("1.5h").unwrap(),
            (1.5 * HOUR as f64).round() as i64
        );
    }

    #[test]
    fn parse_human_readable() {
        assert_eq!(parse_duration("90 minutes").unwrap(), 90 * MIN);
        assert_eq!(parse_duration("500ms").unwrap(), 500 * MS);
    }

    #[test]
    fn parse_rejects_invalid() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("0 hours").is_err());
        assert!(parse_duration("2").is_err());
    }

    #[test]
    fn format_duration_human() {
        assert_eq!(format_duration(720 * HOUR), "30 days");
        assert_eq!(format_duration(30 * MIN), "30 minutes");
    }

    #[test]
    fn format_go_duration_compact() {
        assert_eq!(format_go_duration(720 * HOUR), "720h");
        assert_eq!(format_go_duration(30 * MIN), "30m");
    }

    #[test]
    fn round_trip_human() {
        let s = "2 months 3 days 5 seconds";
        let nanos = parse_duration(s).unwrap();
        assert_eq!(format_duration(nanos), s);
    }
}
