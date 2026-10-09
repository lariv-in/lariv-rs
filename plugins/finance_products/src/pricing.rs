//! Product price formulas.
//!
//! A product stores a [`VariableSchema`], a base price formula, and a sales price
//! formula. Each formula is the price of one product. Invoice line quantity
//! multiplies that amount. Variables such as length and weight describe the
//! single product; they are not a substitute for line quantity.

use rust_decimal::Decimal;
use serde::Serialize;

use lariv_formula::{
    FormulaContext, VariableSchema, VariableType, VariableValues, display_variable_name,
    eval_formula, format_value_lines, format_values_display, parse_schema, parse_schema_list,
    parse_values, persist_values, schema_to_entries, schema_to_json, validate_formula,
};
use lariv_plugin_finance_common::decimal;

#[derive(Clone, Debug)]
pub struct PricedLine {
    pub pre_tax: Decimal,
    pub quantity: Decimal,
    pub rate: Decimal,
    /// Typed values as a JSON object. Length and weight are
    /// `{"value","unit"}`; other variables stay scalar.
    pub variable_values: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct VariableInputSpec {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub placeholder: String,
}

/// True when the product defines variables, so invoice lines price from the formula.
pub fn has_variable_pricing(schema_json: &str) -> bool {
    matches!(schema_is_empty(schema_json), Ok(false))
}

pub fn schema_is_empty(schema_json: &str) -> Result<bool, String> {
    Ok(load_schema(schema_json)?.is_empty())
}

fn formula_references_schema(schema: &VariableSchema, formula: &str) -> bool {
    schema.keys().any(|name| mentions_ident(formula, name))
}

fn mentions_ident(formula: &str, name: &str) -> bool {
    let bytes = formula.as_bytes();
    let name_bytes = name.as_bytes();
    if name_bytes.is_empty() || name_bytes.len() > bytes.len() {
        return false;
    }
    let mut index = 0;
    while index + name_bytes.len() <= bytes.len() {
        if &bytes[index..index + name_bytes.len()] == name_bytes {
            let before_ok = index == 0 || !is_ident_byte(bytes[index - 1]);
            let after = index + name_bytes.len();
            let after_ok = after == bytes.len() || !is_ident_byte(bytes[after]);
            if before_ok && after_ok {
                return true;
            }
        }
        index += 1;
    }
    false
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

pub fn load_schema(raw: &str) -> Result<VariableSchema, String> {
    let raw = raw.trim();
    if raw.is_empty() || raw == "{}" {
        return Ok(VariableSchema::new());
    }
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("invalid variables: {e}"))?;
    parse_schema(&value).map_err(|e| e.to_string())
}

pub fn schema_entries_from_stored(raw: &str) -> Vec<String> {
    load_schema(raw)
        .map(|schema| schema_to_entries(&schema))
        .unwrap_or_default()
}

pub fn store_schema(entries: &[String]) -> Result<(String, VariableSchema), String> {
    let schema = parse_schema_list(entries).map_err(|e| e.to_string())?;
    let json = schema_to_json(&schema).to_string();
    Ok((json, schema))
}

pub fn validate_price_formula(
    schema: &VariableSchema,
    formula: &str,
    label: &str,
) -> Result<(), String> {
    let formula = formula.trim();
    if formula.is_empty() {
        return Err(format!("{label} is required"));
    }
    validate_formula(schema, formula)
        .map(|_| ())
        .map_err(|e| format!("{label}: {e}"))
}

/// Evaluate a formula that does not depend on schema variables.
pub fn unit_amount(formula: &str) -> Result<Decimal, String> {
    let amount = eval_formula(&VariableSchema::new(), formula, &VariableValues::new())
        .map_err(|e| e.to_string())?;
    if amount < Decimal::ZERO {
        return Err("price formula returned a negative amount".into());
    }
    Ok(decimal::normalize(amount))
}

/// Line cost from the base price formula of one product, times `quantity`.
///
/// The formula is the cost of a single product. When it names a schema
/// variable, the saved line values are supplied, then `quantity` multiplies
/// the result.
pub fn line_cost(
    schema_json: &str,
    formula: &str,
    raw_variables: &str,
    quantity: Decimal,
) -> Result<Decimal, String> {
    if quantity <= Decimal::ZERO {
        return Err("quantity must be positive".into());
    }
    let schema = load_schema(schema_json)?;
    let unit = if formula_references_schema(&schema, formula) {
        let raw = serde_json::from_str(raw_variables.trim()).unwrap_or(serde_json::json!({}));
        price_line(schema_json, formula, &raw)?.rate
    } else {
        unit_amount(formula)?
    };
    Ok(decimal::dec_mul(unit, quantity))
}

pub fn variable_input_specs(schema_json: &str) -> Vec<VariableInputSpec> {
    let Ok(schema) = load_schema(schema_json) else {
        return Vec::new();
    };
    schema_to_entries(&schema)
        .into_iter()
        .filter_map(|entry| {
            let (name, ty) = entry.split_once(':')?;
            let ty = VariableType::parse_name(ty)?;
            Some(VariableInputSpec {
                name: name.to_string(),
                ty: ty.as_str().to_string(),
                placeholder: ty.default_placeholder().to_string(),
            })
        })
        .collect()
}

pub fn variable_input_specs_json(schema_json: &str) -> String {
    serde_json::to_string(&variable_input_specs(schema_json)).unwrap_or_else(|_| "[]".into())
}

/// Evaluate `formula` as the price of one product, with `raw` values
/// (`{"name": "user text"}`).
pub fn price_line(
    schema_json: &str,
    formula: &str,
    raw: &serde_json::Value,
) -> Result<PricedLine, String> {
    let schema = load_schema(schema_json)?;
    let values =
        parse_values(&schema, raw, &FormulaContext::default()).map_err(|e| e.to_string())?;
    let unit = eval_formula(&schema, formula, &values).map_err(|e| e.to_string())?;
    if unit < Decimal::ZERO {
        return Err("price formula returned a negative amount".into());
    }
    let unit = decimal::normalize(unit);
    let variable_values = persist_values(&schema, raw, &values).to_string();
    Ok(PricedLine {
        pre_tax: unit,
        quantity: Decimal::ONE,
        rate: unit,
        variable_values,
    })
}

/// Multiply a one-product price by invoice line `quantity`.
pub fn with_quantity(unit: PricedLine, quantity: Decimal) -> Result<PricedLine, String> {
    if quantity <= Decimal::ZERO {
        return Err("quantity must be positive".into());
    }
    let quantity = decimal::normalize(quantity);
    let rate = decimal::normalize(unit.rate);
    Ok(PricedLine {
        pre_tax: decimal::dec_mul(quantity, rate),
        quantity,
        rate,
        variable_values: unit.variable_values,
    })
}

/// Line amount for a product with no variables. `rate` is the price of one product.
pub fn from_quantity_rate(quantity: Decimal, rate: Decimal) -> Result<PricedLine, String> {
    if quantity <= Decimal::ZERO {
        return Err("quantity must be positive".into());
    }
    let quantity = decimal::normalize(quantity);
    let rate = decimal::normalize(rate);
    Ok(PricedLine {
        pre_tax: decimal::dec_mul(quantity, rate),
        quantity,
        rate,
        variable_values: "{}".to_string(),
    })
}

/// Text for a saved line: typed variables, or quantity × rate when there are none.
pub fn inputs_display(variable_values: &str, quantity: Decimal, rate: Decimal) -> String {
    let formatted = format_raw_values(variable_values);
    if !formatted.is_empty() {
        return formatted;
    }
    format!(
        "{} × {}",
        decimal::decimal_display(quantity),
        decimal::decimal_display(rate)
    )
}

/// One formatted `name: value` string per variable, for a column of line breaks.
pub fn format_variable_lines(schema_json: &str, raw: &str) -> Vec<String> {
    let Ok(schema) = load_schema(schema_json) else {
        return raw_value_lines(raw);
    };
    if schema.is_empty() {
        return Vec::new();
    }
    let value: serde_json::Value =
        serde_json::from_str(raw.trim()).unwrap_or(serde_json::json!({}));
    match parse_values(&schema, &value, &FormulaContext::default()) {
        Ok(values) if !values.is_empty() => {
            format_value_lines(&schema, &values, &FormulaContext::default())
        }
        _ => raw_value_lines(raw),
    }
}

pub fn format_variable_inputs(schema_json: &str, raw: &str) -> Option<String> {
    let schema = load_schema(schema_json).ok()?;
    if schema.is_empty() {
        return None;
    }
    let value: serde_json::Value =
        serde_json::from_str(raw.trim()).unwrap_or(serde_json::json!({}));
    match parse_values(&schema, &value, &FormulaContext::default()) {
        Ok(values) if !values.is_empty() => {
            let text = format_values_display(&schema, &values, &FormulaContext::default());
            if text == "-" { None } else { Some(text) }
        }
        _ => {
            let text = format_raw_values(raw);
            if text.is_empty() { None } else { Some(text) }
        }
    }
}

pub fn format_raw_values(raw: &str) -> String {
    raw_value_lines(raw).join(", ")
}

fn raw_value_lines(raw: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw.trim()) else {
        return Vec::new();
    };
    let Some(obj) = value.as_object() else {
        return Vec::new();
    };
    let mut keys: Vec<_> = obj.keys().cloned().collect();
    keys.sort();
    keys.into_iter()
        .filter_map(|key| {
            let text = match obj.get(&key)? {
                serde_json::Value::String(s) => s.trim().to_string(),
                serde_json::Value::Object(map) => {
                    let magnitude = map
                        .get("value")
                        .map(|v| match v {
                            serde_json::Value::String(s) => s.trim().to_string(),
                            other => other.to_string(),
                        })
                        .unwrap_or_default();
                    let unit = map
                        .get("unit")
                        .and_then(|u| u.as_str())
                        .map(str::trim)
                        .filter(|u| !u.is_empty());
                    match unit {
                        Some(unit) if !magnitude.is_empty() => format!("{magnitude} {unit}"),
                        _ => magnitude,
                    }
                }
                other => other.to_string(),
            };
            if text.is_empty() {
                None
            } else {
                Some(format!("{}: {text}", display_variable_name(&key)))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formula_price_uses_variables() {
        let (json, schema) =
            store_schema(&["length:length".into(), "qty:quantity".into()]).unwrap();
        validate_price_formula(
            &schema,
            "length * qty * decimal(\"2\")",
            "sales price formula",
        )
        .unwrap();
        let priced = price_line(
            &json,
            "length * decimal(\"2\")",
            &serde_json::json!({"length": "1000", "qty": "3"}),
        )
        .unwrap();
        assert_eq!(priced.pre_tax, Decimal::from(2000));
        assert_eq!(priced.rate, Decimal::from(2000));
        let line = with_quantity(priced, Decimal::from(3)).unwrap();
        assert_eq!(line.pre_tax, Decimal::from(6000));
        assert_eq!(line.quantity, Decimal::from(3));
        assert_eq!(line.rate, Decimal::from(2000));
    }

    #[test]
    fn empty_formula_is_rejected() {
        let schema = VariableSchema::new();
        let err = validate_price_formula(&schema, "   ", "base price formula").unwrap_err();
        assert!(err.contains("base price formula is required"));
    }

    #[test]
    fn constant_base_formula_is_one_product_times_quantity() {
        let one = line_cost("{}", r#"decimal("40")"#, "{}", Decimal::ONE).unwrap();
        assert_eq!(one, Decimal::from(40));
        let three = line_cost("{}", r#"decimal("40")"#, "{}", Decimal::from(3)).unwrap();
        assert_eq!(three, Decimal::from(120));
    }

    #[test]
    fn variable_inputs_include_names_and_units() {
        let (json, _) = store_schema(&[
            "length:length".into(),
            "qty:quantity".into(),
            "mass:weight".into(),
            "gst:percent".into(),
        ])
        .unwrap();
        let text = format_variable_inputs(
            &json,
            r#"{"length":"1000","qty":"2","mass":"1.5","gst":"18"}"#,
        )
        .unwrap();
        assert!(text.contains("Length: 1000 mm"), "{text}");
        assert!(text.contains("Qty: 2"), "{text}");
        assert!(text.contains("Mass: 1.5 kg"), "{text}");
        assert!(text.contains("Gst: 18%"), "{text}");
    }

    #[test]
    fn stored_length_and_weight_keep_units() {
        let (json, _) = store_schema(&["length:length".into(), "mass:weight".into()]).unwrap();
        let priced = price_line(
            &json,
            "length + mass",
            &serde_json::json!({
                "length": {"value": "2", "unit": "cm"},
                "mass": {"value": "1.5", "unit": "kg"},
            }),
        )
        .unwrap();
        let stored: serde_json::Value = serde_json::from_str(&priced.variable_values).unwrap();
        assert_eq!(
            stored["length"],
            serde_json::json!({"value": "2", "unit": "cm"})
        );
        assert_eq!(
            stored["mass"],
            serde_json::json!({"value": "1.5", "unit": "kg"})
        );
        assert_eq!(priced.pre_tax, Decimal::new(215, 1));
    }

    #[test]
    fn variable_base_formula_is_one_product_times_quantity() {
        let (json, _) = store_schema(&["length:length".into()]).unwrap();
        let cost = line_cost(
            &json,
            r#"length * decimal("2")"#,
            r#"{"length":"1000"}"#,
            Decimal::from(3),
        )
        .unwrap();
        assert_eq!(cost, Decimal::from(6000));
    }
}
