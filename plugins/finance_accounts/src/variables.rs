//! Variable schemas for custom source doc templates.

use lariv_formula::{
    VariableSchema, parse_schema, parse_schema_list, schema_to_entries, schema_to_json,
};
use serde::Serialize;

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

#[derive(Clone, Debug, Serialize)]
pub struct VariableInputRow {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub value: String,
    pub unit: String,
}

fn default_unit(ty: &str, accepted_unit: &str) -> String {
    if !accepted_unit.is_empty() {
        return accepted_unit.to_string();
    }
    match ty {
        "length" => "mm".into(),
        "weight" => "kg".into(),
        "quantity" => "Number".into(),
        "duration" => "s".into(),
        "percent" => "%".into(),
        _ => "None".into(),
    }
}

fn stored_magnitude(value: &serde_json::Value, fallback_unit: &str) -> (String, String) {
    if let Some(obj) = value.as_object() {
        let magnitude = obj
            .get("value")
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default();
        let unit = obj
            .get("unit")
            .and_then(|unit| unit.as_str())
            .map(str::trim)
            .filter(|unit| !unit.is_empty())
            .unwrap_or(fallback_unit)
            .to_string();
        return (magnitude, unit);
    }
    let magnitude = match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    };
    (magnitude, fallback_unit.to_string())
}

/// Alpine rows for the variable-values editor (`name`, `type`, `value`, `unit`).
pub fn variable_input_rows(schema_json: &str, values_json: &str) -> Vec<VariableInputRow> {
    let Ok(schema) = load_schema(schema_json) else {
        return Vec::new();
    };
    let stored: serde_json::Value =
        serde_json::from_str(values_json.trim()).unwrap_or(serde_json::json!({}));
    let mut names: Vec<_> = schema.keys().cloned().collect();
    names.sort();
    names
        .into_iter()
        .filter_map(|name| {
            let def = schema.get(&name)?;
            let ty = def.ty.as_str().to_string();
            let fallback = default_unit(&ty, &def.accepted_unit);
            let (value, unit) = stored
                .get(&name)
                .map(|v| stored_magnitude(v, &fallback))
                .unwrap_or_else(|| (String::new(), fallback));
            Some(VariableInputRow {
                name,
                ty,
                value,
                unit,
            })
        })
        .collect()
}

pub fn variable_input_rows_json(schema_json: &str, values_json: &str) -> String {
    serde_json::to_string(&variable_input_rows(schema_json, values_json))
        .unwrap_or_else(|_| "[]".into())
}
