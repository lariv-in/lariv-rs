//! Product price formulas.
//!
//! A product stores a [`VariableSchema`], a base price formula, and a sales price
//! formula. Without variables, each formula is a unit amount and invoice lines
//! multiply it by quantity. With variables, [`price_line`] evaluates the sales
//! price formula as the line price. The base price formula is the line cost when
//! it names a variable, and a unit cost otherwise.

use rust_decimal::Decimal;
use serde::Serialize;

use lariv_formula::{
    FormulaContext, VariableSchema, VariableType, VariableValue, VariableValues, eval_formula,
    format_values_display, parse_schema, parse_schema_list, parse_values, schema_to_entries,
    schema_to_json, validate_formula,
};
use lariv_plugin_finance_common::decimal;

#[derive(Clone, Debug)]
pub struct PricedLine {
    pub pre_tax: Decimal,
    pub quantity: Decimal,
    pub rate: Decimal,
    /// Raw values the user typed, as a JSON object.
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

/// Line cost from the base price formula.
///
/// A formula that names a schema variable is the line cost. Otherwise it is a
/// unit cost multiplied by `quantity`.
pub fn line_cost(
    schema_json: &str,
    formula: &str,
    raw_variables: &str,
    quantity: Decimal,
) -> Result<Decimal, String> {
    let schema = load_schema(schema_json)?;
    if formula_references_schema(&schema, formula) {
        let raw = serde_json::from_str(raw_variables.trim()).unwrap_or(serde_json::json!({}));
        return Ok(price_line(schema_json, formula, &raw)?.pre_tax);
    }
    let unit = unit_amount(formula)?;
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

/// Evaluate `formula` with `raw` values (`{"name": "user text"}`).
pub fn price_line(
    schema_json: &str,
    formula: &str,
    raw: &serde_json::Value,
) -> Result<PricedLine, String> {
    let schema = load_schema(schema_json)?;
    let values =
        parse_values(&schema, raw, &FormulaContext::default()).map_err(|e| e.to_string())?;
    let pre_tax = eval_formula(&schema, formula, &values).map_err(|e| e.to_string())?;
    if pre_tax < Decimal::ZERO {
        return Err("price formula returned a negative amount".into());
    }
    let quantity = billing_quantity(&schema, &values)?;
    let pre_tax = decimal::normalize(pre_tax);
    let rate = decimal::normalize(pre_tax / quantity);
    let variable_values = match raw {
        serde_json::Value::Object(_) => raw.to_string(),
        _ => "{}".to_string(),
    };
    Ok(PricedLine {
        pre_tax,
        quantity: decimal::normalize(quantity),
        rate,
        variable_values,
    })
}

pub fn from_quantity_rate(quantity: Decimal, rate: Decimal) -> PricedLine {
    PricedLine {
        pre_tax: decimal::dec_mul(quantity, rate),
        quantity: decimal::normalize(quantity),
        rate: decimal::normalize(rate),
        variable_values: "{}".to_string(),
    }
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

/// Saved formula inputs with each name and its unit (`length: 1000 mm`, `mass: 1.5 kg`).
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
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw.trim()) else {
        return String::new();
    };
    let Some(obj) = value.as_object() else {
        return String::new();
    };
    let mut keys: Vec<_> = obj.keys().cloned().collect();
    keys.sort();
    keys.into_iter()
        .filter_map(|key| {
            let text = match obj.get(&key)? {
                serde_json::Value::String(s) => s.trim().to_string(),
                other => other.to_string(),
            };
            if text.is_empty() {
                None
            } else {
                Some(format!("{key}: {text}"))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn billing_quantity(schema: &VariableSchema, values: &VariableValues) -> Result<Decimal, String> {
    let mut names: Vec<&String> = schema
        .iter()
        .filter(|(_, ty)| **ty == VariableType::Quantity)
        .map(|(name, _)| name)
        .collect();
    if names.is_empty() {
        return Ok(Decimal::ONE);
    }
    names.sort();
    let name = names
        .iter()
        .copied()
        .find(|name| name.as_str() == "quantity")
        .unwrap_or(names[0]);
    match values.get(name) {
        Some(VariableValue::Quantity(n)) if *n > 0 => Ok(Decimal::from(*n)),
        Some(VariableValue::Quantity(_)) => Err(format!("`{name}` must be positive")),
        _ => Err(format!("`{name}` is required")),
    }
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
            "length * qty * decimal(\"2\")",
            &serde_json::json!({"length": "1000", "qty": "3"}),
        )
        .unwrap();
        assert_eq!(priced.pre_tax, Decimal::from(6000));
        assert_eq!(priced.quantity, Decimal::from(3));
        assert_eq!(priced.rate, Decimal::from(2000));
    }

    #[test]
    fn empty_formula_is_rejected() {
        let schema = VariableSchema::new();
        let err = validate_price_formula(&schema, "   ", "base price formula").unwrap_err();
        assert!(err.contains("base price formula is required"));
    }

    #[test]
    fn constant_base_formula_scales_with_quantity() {
        let cost = line_cost("{}", r#"decimal("40")"#, "{}", Decimal::from(2)).unwrap();
        assert_eq!(cost, Decimal::from(80));
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
        assert!(text.contains("length: 1000 mm"), "{text}");
        assert!(text.contains("qty: 2"), "{text}");
        assert!(text.contains("mass: 1.5 kg"), "{text}");
        assert!(text.contains("gst: 18%"), "{text}");
    }

    #[test]
    fn variable_base_formula_is_the_line_cost() {
        let (json, _) = store_schema(&["length:length".into(), "qty:quantity".into()]).unwrap();
        let cost = line_cost(
            &json,
            r#"length * qty * decimal("2")"#,
            r#"{"length":"1000","qty":"3"}"#,
            Decimal::from(3),
        )
        .unwrap();
        assert_eq!(cost, Decimal::from(6000));
    }
}
