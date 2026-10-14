//! On-hand quantity from movement lines.
//!
//! Length is summed in millimetres, then shown in the stock's unit. Weight is
//! summed in kilograms, then shown in the stock's unit. Other formula types sum
//! the line decimals when the unit matches.

use lariv_core::length::{NM_PER_MM, parse_length_unit};
pub use lariv_core::weight::{self, WeightUnit, parse_weight_unit};
use lariv_formula::{VariableType, parse_length};
use rust_decimal::Decimal;

/// Whether a movement line adds to or subtracts from on-hand quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QtySign {
    Add,
    Subtract,
}

/// One movement line contributing to a stock's on-hand quantity.
pub struct QtyLine {
    pub sign: QtySign,
    pub qty: Decimal,
    pub qty_unit: String,
    pub qty_type: String,
}

pub fn type_label(raw: &str) -> String {
    VariableType::parse_name(raw)
        .map(|ty| ty.label().to_string())
        .unwrap_or_else(|| raw.to_string())
}

pub fn format_on_hand(qty: Decimal, unit: &str) -> String {
    let num = qty.round_dp(6).normalize().to_string();
    let unit = unit.trim();
    if unit.is_empty() {
        num
    } else {
        format!("{num} {unit}")
    }
}

pub fn parse_qty(raw: &str) -> Result<Decimal, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("quantity is required".into());
    }
    let qty = s
        .parse::<Decimal>()
        .map_err(|_err| format!("invalid quantity `{s}`"))?;
    if qty.is_sign_negative() {
        return Err("quantity cannot be negative".into());
    }
    Ok(qty.round_dp(6))
}

pub fn parse_type_name(raw: &str) -> Result<VariableType, String> {
    VariableType::parse_name(raw).ok_or_else(|| format!("unknown quantity type `{raw}`"))
}

/// Unit stored on a stock. Length and weight keep a canonical unit; other types do not.
pub fn stock_display_unit(ty: VariableType, unit: &str) -> Result<String, String> {
    match ty {
        VariableType::Length | VariableType::Weight => normalize_unit(ty, unit),
        _ => Ok(String::new()),
    }
}

pub fn type_uses_unit(ty: VariableType) -> bool {
    matches!(ty, VariableType::Length | VariableType::Weight)
}

/// Canonical unit string stored on a stock or line.
pub fn normalize_unit(ty: VariableType, unit: &str) -> Result<String, String> {
    match ty {
        VariableType::Length => {
            if unit.trim().is_empty() {
                return Err("length unit is required".into());
            }
            let parsed =
                parse_length_unit(unit).ok_or_else(|| format!("unknown length unit `{unit}`"))?;
            Ok(parsed.as_str().to_string())
        }
        VariableType::Weight => {
            if unit.trim().is_empty() {
                return Ok(WeightUnit::Kilogram.as_str().to_string());
            }
            let parsed =
                parse_weight_unit(unit).ok_or_else(|| format!("unknown weight unit `{unit}`"))?;
            Ok(parsed.as_str().to_string())
        }
        _ => Ok(unit.trim().to_string()),
    }
}

/// Check a line against its stock and return the stored type and unit.
pub fn check_line(
    stock_type: &str,
    stock_unit: &str,
    qty: Decimal,
    line_unit: &str,
    line_type: &str,
) -> Result<(String, String), String> {
    let stock_ty = parse_type_name(stock_type)?;
    let line_ty = parse_type_name(line_type)?;
    if stock_ty != line_ty {
        return Err("line type must match the stock".into());
    }
    let unit = normalize_unit(line_ty, line_unit)?;
    let stock_unit_norm = normalize_unit(stock_ty, stock_unit)?;
    if !matches!(line_ty, VariableType::Length | VariableType::Weight) && unit != stock_unit_norm {
        return Err("line unit must match the stock".into());
    }
    if line_ty == VariableType::Quantity && !qty.fract().is_zero() {
        return Err("quantity must be a whole number".into());
    }
    if line_ty == VariableType::Length {
        length_mm(qty, &unit)?;
    }
    if line_ty == VariableType::Weight {
        weight_kg(qty, &unit)?;
    }
    Ok((line_ty.as_str().to_string(), unit))
}

/// Signed sum of `lines` in the stock's display unit.
pub fn sum_on_hand(
    stock_type: &str,
    stock_unit: &str,
    lines: &[QtyLine],
) -> Result<Decimal, String> {
    let stock_ty = parse_type_name(stock_type)?;
    let stock_unit_norm = normalize_unit(stock_ty, stock_unit)?;
    if stock_ty == VariableType::Length {
        let mut mm = Decimal::ZERO;
        for line in lines {
            mm += signed_length_mm(stock_ty, &stock_unit_norm, line)?;
        }
        return Ok(mm_in_unit(mm, &stock_unit_norm)?.round_dp(6).normalize());
    }
    if stock_ty == VariableType::Weight {
        let mut kg = Decimal::ZERO;
        for line in lines {
            kg += signed_weight_kg(stock_ty, line)?;
        }
        return Ok(kg_in_unit(kg, &stock_unit_norm)?.round_dp(6).normalize());
    }
    let mut total = Decimal::ZERO;
    for line in lines {
        total += signed_scalar(stock_ty, &stock_unit_norm, line)?;
    }
    Ok(total.round_dp(6).normalize())
}

fn signed_length_mm(
    stock_ty: VariableType,
    _stock_unit: &str,
    line: &QtyLine,
) -> Result<Decimal, String> {
    let line_ty = parse_type_name(&line.qty_type)?;
    if line_ty != stock_ty {
        return Err("line type must match the stock".into());
    }
    let unit = normalize_unit(line_ty, &line.qty_unit)?;
    let mm = length_mm(line.qty, &unit)?;
    Ok(apply_sign(line.sign, mm))
}

fn signed_weight_kg(stock_ty: VariableType, line: &QtyLine) -> Result<Decimal, String> {
    let line_ty = parse_type_name(&line.qty_type)?;
    if line_ty != stock_ty {
        return Err("line type must match the stock".into());
    }
    let unit = normalize_unit(line_ty, &line.qty_unit)?;
    let kg = weight_kg(line.qty, &unit)?;
    Ok(apply_sign(line.sign, kg))
}

fn signed_scalar(
    stock_ty: VariableType,
    stock_unit: &str,
    line: &QtyLine,
) -> Result<Decimal, String> {
    let line_ty = parse_type_name(&line.qty_type)?;
    if line_ty != stock_ty {
        return Err("line type must match the stock".into());
    }
    let unit = normalize_unit(line_ty, &line.qty_unit)?;
    if unit != stock_unit {
        return Err("line unit must match the stock".into());
    }
    if line_ty == VariableType::Quantity && !line.qty.fract().is_zero() {
        return Err("quantity must be a whole number".into());
    }
    Ok(apply_sign(line.sign, line.qty))
}

fn apply_sign(sign: QtySign, qty: Decimal) -> Decimal {
    match sign {
        QtySign::Add => qty,
        QtySign::Subtract => -qty,
    }
}

fn length_mm(qty: Decimal, unit: &str) -> Result<Decimal, String> {
    let raw = qty.normalize().to_string();
    parse_length(&raw, unit).map_err(|err| err.to_string())
}

fn weight_kg(qty: Decimal, unit: &str) -> Result<Decimal, String> {
    let parsed = parse_weight_unit(unit).ok_or_else(|| format!("unknown weight unit `{unit}`"))?;
    Ok(weight::to_kg(qty, parsed))
}

fn kg_in_unit(kg: Decimal, unit: &str) -> Result<Decimal, String> {
    let parsed = parse_weight_unit(unit).ok_or_else(|| format!("unknown weight unit `{unit}`"))?;
    Ok(weight::from_kg(kg, parsed))
}

fn mm_in_unit(mm: Decimal, unit: &str) -> Result<Decimal, String> {
    let parsed = parse_length_unit(unit).ok_or_else(|| format!("unknown length unit `{unit}`"))?;
    let per_mm = i128_dec(NM_PER_MM)?;
    let per_unit = i128_dec(parsed.nm_per_unit())?;
    if per_unit.is_zero() {
        return Err("length unit has no scale".into());
    }
    Ok((mm * per_mm) / per_unit)
}

fn i128_dec(v: i128) -> Result<Decimal, String> {
    let n = i64::try_from(v).map_err(|_err| "length overflow".to_string())?;
    Ok(Decimal::from(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(raw: &str) -> Decimal {
        raw.parse().expect("decimal")
    }

    fn line(sign: QtySign, qty: &str, unit: &str, ty: &str) -> QtyLine {
        QtyLine {
            sign,
            qty: d(qty),
            qty_unit: unit.to_string(),
            qty_type: ty.to_string(),
        }
    }

    #[test]
    fn length_converts_to_stock_unit() {
        let lines = [
            line(QtySign::Add, "1", "m", "length"),
            line(QtySign::Subtract, "50", "cm", "length"),
        ];
        let got = sum_on_hand("length", "cm", &lines).expect("sum");
        assert_eq!(got, d("50"));
    }

    #[test]
    fn length_centimetre_in_is_ten_millimetres() {
        let lines = [line(QtySign::Add, "1", "cm", "length")];
        let got = sum_on_hand("length", "mm", &lines).expect("sum");
        assert_eq!(got, d("10"));
    }

    #[test]
    fn process_input_adds_and_output_subtracts() {
        let lines = [
            line(QtySign::Add, "8", "", "decimal"),
            line(QtySign::Subtract, "3", "", "decimal"),
        ];
        let got = sum_on_hand("decimal", "", &lines).expect("sum");
        assert_eq!(got, d("5"));
    }

    #[test]
    fn out_subtracts_from_in() {
        let lines = [
            line(QtySign::Add, "10", "", "decimal"),
            line(QtySign::Subtract, "4", "", "decimal"),
        ];
        let got = sum_on_hand("decimal", "", &lines).expect("sum");
        assert_eq!(got, d("6"));
    }

    #[test]
    fn weight_sums_kilograms() {
        let lines = [
            line(QtySign::Add, "1.5", "kg", "weight"),
            line(QtySign::Subtract, "0.25", "kilogram", "weight"),
        ];
        let got = sum_on_hand("weight", "kg", &lines).expect("sum");
        assert_eq!(got, d("1.25"));
    }

    #[test]
    fn quantity_must_be_whole() {
        let err = check_line("quantity", "", d("1.5"), "", "quantity").expect_err("fraction");
        assert!(err.contains("whole number"));
    }

    #[test]
    fn type_mismatch_is_rejected() {
        let err = check_line("decimal", "", d("1"), "mm", "length").expect_err("type");
        assert!(err.contains("type"));
    }

    #[test]
    fn stock_unit_only_for_length_and_weight() {
        assert_eq!(
            stock_display_unit(VariableType::Quantity, "box").unwrap(),
            ""
        );
        assert_eq!(
            stock_display_unit(VariableType::Length, "cm").unwrap(),
            "cm"
        );
        assert_eq!(
            stock_display_unit(VariableType::Weight, "kilogram").unwrap(),
            "kg"
        );
        assert_eq!(
            stock_display_unit(VariableType::Weight, "grams").unwrap(),
            "g"
        );
        assert_eq!(
            stock_display_unit(VariableType::Weight, "pound").unwrap(),
            "lb"
        );
    }

    #[test]
    fn weight_converts_to_stock_unit() {
        let lines = [
            line(QtySign::Add, "1", "kg", "weight"),
            line(QtySign::Subtract, "250", "g", "weight"),
            line(QtySign::Add, "1", "lb", "weight"),
        ];
        let got = sum_on_hand("weight", "g", &lines).expect("sum");
        assert_eq!(got, d("1203.59237"));
    }

    #[test]
    fn non_length_unit_must_match() {
        let err = check_line("decimal", "box", d("1"), "crate", "decimal").expect_err("unit");
        assert!(err.contains("unit"));
        let (ty, unit) = check_line("decimal", "box", d("2"), "box", "decimal").expect("ok");
        assert_eq!(ty, "decimal");
        assert_eq!(unit, "box");
    }
}
