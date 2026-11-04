//! Length stored as millimetres with `Decimal(15, 6)` scale (nanometre precision).
//!
//! Persist with SeaORM as:
//! `#[sea_orm(column_type = "Decimal(Some((15, 6)))")]`
//!
//! The canonical unit is millimetres. Display conversion covers millimetres,
//! centimetres, metres, kilometres, inches, and feet.

use std::fmt;
use std::str::FromStr;

/// Nanometres in one millimetre (`10^-6` mm). Matches `Decimal(15, 6)` scale.
pub const NM_PER_MM: i128 = 1_000_000;

/// Largest absolute nanometre value that fits `Decimal(15, 6)` millimetres
/// (`999_999_999.999999` mm).
pub const MAX_ABS_NM: i128 = 999_999_999_999_999;

/// Display / input unit for a millimetre-backed length.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LengthUnit {
    #[default]
    Millimetre,
    Centimetre,
    Metre,
    Kilometre,
    Inch,
    Foot,
}

impl LengthUnit {
    /// Units offered by the length input, in selector order.
    pub const ALL: [LengthUnit; 6] = [
        Self::Millimetre,
        Self::Centimetre,
        Self::Metre,
        Self::Kilometre,
        Self::Inch,
        Self::Foot,
    ];

    /// Short selector label (`mm`, `cm`, `m`, `km`, `in`, `ft`).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Millimetre => "mm",
            Self::Centimetre => "cm",
            Self::Metre => "m",
            Self::Kilometre => "km",
            Self::Inch => "in",
            Self::Foot => "ft",
        }
    }

    /// Nanometres in one of this unit (1 nm = `10^-6` mm).
    pub const fn nm_per_unit(self) -> i128 {
        match self {
            Self::Millimetre => NM_PER_MM,
            Self::Centimetre => 10 * NM_PER_MM,
            Self::Metre => 1_000 * NM_PER_MM,
            Self::Kilometre => 1_000_000 * NM_PER_MM,
            Self::Inch => 25_400_000,  // 25.4 mm
            Self::Foot => 304_800_000, // 12 in
        }
    }
}

impl fmt::Display for LengthUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LengthUnit {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_length_unit(s).ok_or_else(|| format!("unknown length unit {s:?}"))
    }
}

/// Parse a unit token (`mm`, `cm`, `m`, `km`, `in`, `ft`).
pub fn parse_length_unit(s: &str) -> Option<LengthUnit> {
    match s.trim().to_ascii_lowercase().as_str() {
        "mm" | "millimetre" | "millimeter" | "millimetres" | "millimeters" => {
            Some(LengthUnit::Millimetre)
        }
        "cm" | "centimetre" | "centimeter" | "centimetres" | "centimeters" => {
            Some(LengthUnit::Centimetre)
        }
        "m" | "metre" | "meter" | "metres" | "meters" => Some(LengthUnit::Metre),
        "km" | "kilometre" | "kilometer" | "kilometres" | "kilometers" => {
            Some(LengthUnit::Kilometre)
        }
        "in" | "inch" | "inches" => Some(LengthUnit::Inch),
        "ft" | "foot" | "feet" => Some(LengthUnit::Foot),
        _ => None,
    }
}

/// Parse `s` as a length in `unit` and return nanometres (`10^-6` mm).
pub fn parse_to_nm(s: &str, unit: LengthUnit) -> Result<i128, String> {
    let s = preprocess(s);
    if s.is_empty() {
        return Err("length is required".into());
    }
    let (neg, rest) = match s.as_bytes().first().copied() {
        Some(b'-') => (true, s[1..].trim()),
        Some(b'+') => (false, s[1..].trim()),
        _ => (false, s.as_ref()),
    };
    if rest.is_empty() || rest == "." {
        return Err("invalid length".into());
    }
    let (int_raw, frac_raw) = match rest.split_once('.') {
        Some((i, f)) => (i, f),
        None => (rest, ""),
    };
    if int_raw.is_empty() && frac_raw.is_empty() {
        return Err("invalid length".into());
    }
    if !int_raw.chars().all(|c| c.is_ascii_digit()) || !frac_raw.chars().all(|c| c.is_ascii_digit())
    {
        return Err("invalid length".into());
    }
    let int_digits = if int_raw.is_empty() {
        "0"
    } else {
        let trimmed = int_raw.trim_start_matches('0');
        if trimmed.is_empty() { "0" } else { trimmed }
    };
    let frac = if frac_raw.len() > 18 {
        &frac_raw[..18]
    } else {
        frac_raw
    };
    let frac_len = frac.len() as u32;
    let frac_den = 10i128.checked_pow(frac_len).ok_or("length overflow")?;
    let int_n: i128 = int_digits
        .parse()
        .map_err(|_| "invalid length".to_string())?;
    let frac_n: i128 = if frac.is_empty() {
        0
    } else {
        frac.parse().map_err(|_| "invalid length".to_string())?
    };
    let whole = int_n
        .checked_mul(frac_den)
        .and_then(|v| v.checked_add(frac_n))
        .ok_or_else(|| "length overflow".to_string())?;
    let numer = whole
        .checked_mul(unit.nm_per_unit())
        .ok_or_else(|| "length overflow".to_string())?;
    let nm = div_round_half_away(numer, frac_den);
    if nm > MAX_ABS_NM {
        return Err("length exceeds Decimal(15, 6) millimetres".into());
    }
    Ok(if neg { -nm } else { nm })
}

/// Parse a millimetre string into nanometres.
pub fn parse_mm(s: &str) -> Result<i128, String> {
    parse_to_nm(s, LengthUnit::Millimetre)
}

/// Format nanometres as a millimetre string (at most 6 fractional digits).
pub fn format_mm(nm: i128) -> String {
    format_nm(nm, LengthUnit::Millimetre)
}

/// Format nanometres in `unit`, stripping trailing zeros.
pub fn format_nm(nm: i128, unit: LengthUnit) -> String {
    let neg = nm < 0;
    let nm = nm.unsigned_abs();
    let denom = unit.nm_per_unit() as u128;
    let whole = nm / denom;
    let rem = nm % denom;
    let sign = if neg { "-" } else { "" };
    if rem == 0 {
        return format!("{sign}{whole}");
    }
    let mut frac = String::new();
    let mut r = rem;
    for _ in 0..18 {
        r *= 10;
        let digit = (r / denom) as u8;
        r %= denom;
        frac.push(char::from(b'0' + digit));
        if r == 0 {
            break;
        }
    }
    let frac = frac.trim_end_matches('0');
    format!("{sign}{whole}.{frac}")
}

/// Convert a millimetre string into `unit` for display.
pub fn format_mm_as(s: &str, unit: LengthUnit) -> Result<String, String> {
    Ok(format_nm(parse_mm(s)?, unit))
}

/// Format `s` (millimetres) with a unit suffix, e.g. `"25.4 mm"`.
pub fn format_length_label(s: &str, unit: LengthUnit) -> String {
    match format_mm_as(s, unit) {
        Ok(n) => format!("{n} {unit}"),
        Err(_) => s.trim().to_string(),
    }
}

/// Normalize a millimetre string to `Decimal(15, 6)` canonical form.
///
/// Empty input stays empty.
pub fn normalize_mm(s: &str) -> Result<String, String> {
    let s = preprocess(s);
    if s.is_empty() {
        return Ok(String::new());
    }
    Ok(format_mm(parse_mm(&s)?))
}

fn preprocess(s: &str) -> std::borrow::Cow<'_, str> {
    let s = s.trim();
    if s.contains(',') {
        std::borrow::Cow::Owned(s.chars().filter(|&c| c != ',').collect())
    } else {
        std::borrow::Cow::Borrowed(s)
    }
}

fn div_round_half_away(numer: i128, denom: i128) -> i128 {
    let q = numer / denom;
    let r = numer % denom;
    if r.saturating_mul(2) >= denom {
        q + 1
    } else {
        q
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_match_metric_and_imperial() {
        assert_eq!(parse_to_nm("1", LengthUnit::Millimetre).unwrap(), NM_PER_MM);
        assert_eq!(
            parse_to_nm("1", LengthUnit::Centimetre).unwrap(),
            10 * NM_PER_MM
        );
        assert_eq!(
            parse_to_nm("1", LengthUnit::Metre).unwrap(),
            1_000 * NM_PER_MM
        );
        assert_eq!(
            parse_to_nm("1", LengthUnit::Kilometre).unwrap(),
            1_000_000 * NM_PER_MM
        );
        assert_eq!(parse_to_nm("1", LengthUnit::Inch).unwrap(), 25_400_000);
        assert_eq!(parse_to_nm("1", LengthUnit::Foot).unwrap(), 304_800_000);
        assert_eq!(parse_to_nm("12", LengthUnit::Inch).unwrap(), 304_800_000);
    }

    #[test]
    fn format_mm_strips_trailing_zeros() {
        assert_eq!(format_mm(25_400_000), "25.4");
        assert_eq!(format_mm(NM_PER_MM), "1");
        assert_eq!(format_mm(1), "0.000001");
    }

    #[test]
    fn inch_round_trip_from_mm() {
        let nm = parse_mm("25.4").unwrap();
        assert_eq!(format_nm(nm, LengthUnit::Inch), "1");
        assert_eq!(format_mm_as("304.8", LengthUnit::Foot).unwrap(), "1");
    }

    #[test]
    fn parse_strips_commas_and_signs() {
        assert_eq!(parse_mm("1,000").unwrap(), 1_000 * NM_PER_MM);
        assert_eq!(parse_mm("+2.5").unwrap(), 2_500_000);
        assert_eq!(parse_mm("-1").unwrap(), -NM_PER_MM);
    }

    #[test]
    fn decimal_15_6_bounds() {
        assert!(parse_mm("999999999.999999").is_ok());
        assert!(parse_mm("1000000000").is_err());
        assert!(parse_to_nm("1000", LengthUnit::Kilometre).is_err());
        assert!(parse_to_nm("999.999999", LengthUnit::Kilometre).is_ok());
    }

    #[test]
    fn normalize_empty_and_canonical() {
        assert_eq!(normalize_mm("").unwrap(), "");
        assert_eq!(normalize_mm("25.400000").unwrap(), "25.4");
        assert_eq!(normalize_mm("  1,234.50 ").unwrap(), "1234.5");
    }

    #[test]
    fn parse_unit_aliases() {
        assert_eq!(parse_length_unit("MM").unwrap(), LengthUnit::Millimetre);
        assert_eq!(parse_length_unit("feet").unwrap(), LengthUnit::Foot);
        assert!(parse_length_unit("yard").is_none());
    }

    #[test]
    fn format_length_label_suffix() {
        assert_eq!(format_length_label("25.4", LengthUnit::Inch), "1 in");
        assert_eq!(format_length_label("1000", LengthUnit::Metre), "1 m");
    }

    #[test]
    fn reject_invalid() {
        assert!(parse_mm("").is_err());
        assert!(parse_mm("abc").is_err());
        assert!(parse_mm("1.2.3").is_err());
    }
}
