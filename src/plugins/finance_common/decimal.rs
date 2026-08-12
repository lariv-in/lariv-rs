//! Six-decimal finance arithmetic (mirrors Go fields.DecimalSix).

use rust_decimal::Decimal;

pub fn normalize(d: Decimal) -> Decimal {
    d.round_dp(6).normalize()
}

pub fn dec_mul(a: Decimal, b: Decimal) -> Decimal {
    normalize(a * b)
}

pub fn dec_sum(a: Decimal, b: Decimal) -> Decimal {
    normalize(a + b)
}

pub fn dec_neg(a: Decimal) -> Decimal {
    normalize(-a)
}

pub fn dec_sub(a: Decimal, b: Decimal) -> Decimal {
    normalize(a - b)
}

pub fn dec_abs(a: Decimal) -> Decimal {
    normalize(a.abs())
}

pub fn dec_is_zero(d: Decimal) -> bool {
    d.is_zero()
}

pub fn dec_cmp(a: Decimal, b: Decimal) -> std::cmp::Ordering {
    a.cmp(&b)
}

pub fn decimal_display(d: Decimal) -> String {
    let s = normalize(d).to_string();
    if !s.contains('.') {
        return s;
    }
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// Format a monetary amount for display.
///
/// Pads fractional digits to at least `minor_unit`, keeps any extra significant
/// digits beyond that, then appends `symbol` when non-empty (`"{number} {symbol}"`).
pub fn decimal_display_currency(d: Decimal, minor_unit: i32, symbol: &str) -> String {
    let number = decimal_display_currency_number(d, minor_unit);
    let symbol = symbol.trim();
    if symbol.is_empty() {
        number
    } else {
        format!("{number} {symbol}")
    }
}

fn decimal_display_currency_number(d: Decimal, minor_unit: i32) -> String {
    let minor = minor_unit.clamp(0, 6) as usize;
    let s = normalize(d).to_string();
    let (int_part, frac_part) = match s.split_once('.') {
        Some((i, f)) => (i.to_string(), f.to_string()),
        None => (s, String::new()),
    };
    let mut frac = frac_part.trim_end_matches('0').to_string();
    while frac.len() < minor {
        frac.push('0');
    }
    if frac.is_empty() {
        int_part
    } else {
        format!("{int_part}.{frac}")
    }
}

pub fn decimal_display_withholding(d: Decimal, minor_unit: i32, symbol: &str) -> String {
    if dec_is_zero(d) {
        "—".to_string()
    } else {
        format!("({})", decimal_display_currency(d, minor_unit, symbol))
    }
}

pub fn parse_decimal(s: &str) -> Option<Decimal> {
    let s = crate::html_form::preprocess_numeric_form_value(s);
    if s.is_empty() {
        return None;
    }
    s.parse().ok().map(normalize)
}

pub fn optional_u64(v: Option<i64>) -> u64 {
    v.unwrap_or(0).max(0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn parse_decimal_strips_commas() {
        assert_eq!(
            parse_decimal("1,234.50"),
            Some(normalize(Decimal::from_str("1234.50").unwrap()))
        );
        assert_eq!(
            parse_decimal("1,000"),
            Some(normalize(Decimal::from_str("1000").unwrap()))
        );
    }

    #[test]
    fn currency_pads_to_minor_unit() {
        assert_eq!(decimal_display_currency(d("10.5"), 2, "INR"), "10.50 INR");
        assert_eq!(decimal_display_currency(d("1000"), 2, "USD"), "1000.00 USD");
        assert_eq!(decimal_display_currency(d("1000"), 0, "JPY"), "1000 JPY");
    }

    #[test]
    fn currency_keeps_extra_significant_digits() {
        assert_eq!(
            decimal_display_currency(d("10.505"), 2, "USD"),
            "10.505 USD"
        );
        assert_eq!(
            decimal_display_currency(d("1.2345"), 2, "INR"),
            "1.2345 INR"
        );
    }

    #[test]
    fn currency_empty_symbol_is_number_only() {
        assert_eq!(decimal_display_currency(d("10.5"), 2, ""), "10.50");
        assert_eq!(decimal_display_currency(d("10.5"), 2, "  "), "10.50");
    }

    #[test]
    fn currency_negative_amounts() {
        assert_eq!(
            decimal_display_currency(d("-10.5"), 2, "INR"),
            "-10.50 INR"
        );
    }

    #[test]
    fn currency_clamps_minor_unit() {
        assert_eq!(decimal_display_currency(d("1"), -1, "X"), "1 X");
        assert_eq!(
            decimal_display_currency(d("1.1234567"), 9, "X"),
            "1.123457 X"
        );
    }

    #[test]
    fn withholding_currency() {
        assert_eq!(decimal_display_withholding(d("0"), 2, "INR"), "—");
        assert_eq!(
            decimal_display_withholding(d("10.5"), 2, "INR"),
            "(10.50 INR)"
        );
    }
}
