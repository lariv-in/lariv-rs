//! Typed variables and Rune formula evaluation.
//!
//! A catalog model stores a [`VariableSchema`] and one or more formula strings.
//! Each use site supplies a [`Measure`] per variable: kind, magnitude, and unit.
//! [`eval_formula`] converts that magnitude into the definition's accepted unit
//! and binds the resulting decimal.
//!
//! Numerics are [`Decimal`] end-to-end. Length and weight keep the unit that was
//! entered and convert only at evaluation. Duration is seconds. Quantity is an
//! integer count (`Number`, shown as `No.`). Decimal stores unit `None` and
//! renders with no unit. A percent is the number the user typed (`18` for 18%).
//!
//! Formula source is rewritten before it reaches Rune: bare numeric literals are
//! wrapped in `decimal(...)`, and a unary minus on anything else (`-pre_tax`)
//! becomes `decimal("0") - ...` because Rune has no negation protocol.

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use lariv_core::duration::parse_duration;
use lariv_core::length::{self, LengthUnit, parse_length_unit, parse_to_nm};
use lariv_core::weight::{self, WeightUnit, parse_weight_unit};
use rune::termcolor::NoColor;
use rune::{Any, Context, Diagnostics, FromValue, Module, Source, Sources, Value, Vm};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

const NANOS_PER_SECOND: i64 = 1_000_000_000;
const MAX_SOURCE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, thiserror::Error)]
pub enum FormulaError {
    #[error("{0}")]
    Message(String),
}

impl FormulaError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableType {
    Length,
    Weight,
    Duration,
    Quantity,
    Decimal,
    Percent,
}

impl VariableType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::Weight => "weight",
            Self::Duration => "duration",
            Self::Quantity => "quantity",
            Self::Decimal => "decimal",
            Self::Percent => "percent",
        }
    }

    pub fn parse_name(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "length" => Some(Self::Length),
            "weight" => Some(Self::Weight),
            "duration" => Some(Self::Duration),
            "quantity" => Some(Self::Quantity),
            "decimal" => Some(Self::Decimal),
            "percent" => Some(Self::Percent),
            _ => None,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Length,
            Self::Weight,
            Self::Duration,
            Self::Quantity,
            Self::Decimal,
            Self::Percent,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Length => "Length",
            Self::Weight => "Weight",
            Self::Duration => "Duration",
            Self::Quantity => "Quantity",
            Self::Decimal => "Decimal",
            Self::Percent => "Percent",
        }
    }

    pub fn default_placeholder(self) -> &'static str {
        match self {
            Self::Length => "mm, e.g. 1000",
            Self::Weight => "kg, e.g. 1.5",
            Self::Duration => "e.g. 2h 30m",
            Self::Quantity => "e.g. 1",
            Self::Decimal => "e.g. 1.5",
            Self::Percent => "e.g. 18",
        }
    }

    /// Unit stored when a definition does not name one.
    ///
    /// Length and weight can be changed. The other kinds have this single unit.
    pub fn default_accepted_unit(self) -> &'static str {
        match self {
            Self::Length => "mm",
            Self::Weight => "kg",
            Self::Duration => "s",
            Self::Quantity => "Number",
            Self::Decimal => "None",
            Self::Percent => "%",
        }
    }

    /// Units a definition may choose. `None` means [`Self::default_accepted_unit`] only.
    pub fn unit_choices(self) -> Option<&'static [(&'static str, &'static str)]> {
        match self {
            Self::Length => Some(&[
                ("mm", "mm"),
                ("cm", "cm"),
                ("m", "m"),
                ("km", "km"),
                ("in", "in"),
                ("ft", "ft"),
            ]),
            Self::Weight => Some(&[
                ("mg", "mg"),
                ("g", "g"),
                ("kg", "kg"),
                ("t", "t"),
                ("lb", "lb"),
            ]),
            _ => None,
        }
    }
}

/// One variable on a catalog model: its kind and the unit formulas receive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableDef {
    pub ty: VariableType,
    pub accepted_unit: String,
}

impl VariableDef {
    pub fn new(ty: VariableType) -> Self {
        Self {
            ty,
            accepted_unit: ty.default_accepted_unit().to_string(),
        }
    }
}

/// A stored or entered magnitude. `value` is in `unit`, not in the accepted unit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Measure {
    pub kind: VariableType,
    pub value: Decimal,
    pub unit: String,
}

impl Measure {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": self.kind.as_str(),
            "value": self.value.normalize().to_string(),
            "unit": self.unit,
        })
    }
}

pub type VariableSchema = HashMap<String, VariableDef>;
pub type VariableValues = HashMap<String, Measure>;

const RUNE_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "false",
    "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
    "use", "where", "while", "yield", "select", "is", "not", "and", "or",
];

/// Letters, digits, and underscores, with a letter first.
fn is_ident_shape(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic() && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn is_valid_ident(name: &str) -> bool {
    is_ident_shape(name) && !RUNE_KEYWORDS.contains(&name)
}

fn ident_rejection(name: &str) -> Option<&'static str> {
    if !is_ident_shape(name) {
        return Some("must start with a letter and contain only letters, digits, and underscores");
    }
    if RUNE_KEYWORDS.contains(&name) {
        return Some("is a reserved word");
    }
    None
}

fn expected_type_names() -> String {
    VariableType::all()
        .iter()
        .map(|ty| ty.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn parse_schema(value: &serde_json::Value) -> Result<VariableSchema, FormulaError> {
    let obj = value
        .as_object()
        .ok_or_else(|| FormulaError::msg("variables schema must be a JSON object"))?;
    let mut out = VariableSchema::new();
    for (name, spec) in obj {
        let name = name.trim();
        if let Some(reason) = ident_rejection(name) {
            return Err(FormulaError::msg(format!(
                "variable name `{name}` {reason}"
            )));
        }
        out.insert(name.to_string(), parse_variable_def(name, spec)?);
    }
    Ok(out)
}

fn parse_variable_def(name: &str, spec: &serde_json::Value) -> Result<VariableDef, FormulaError> {
    let (ty_s, unit) = match spec {
        serde_json::Value::String(ty_s) => (ty_s.as_str(), None),
        serde_json::Value::Object(obj) => {
            let ty_s = obj
                .get("type")
                .and_then(|ty| ty.as_str())
                .ok_or_else(|| FormulaError::msg(format!("variable `{name}` type is required")))?;
            let unit = obj
                .get("accepted_unit")
                .and_then(|unit| unit.as_str())
                .map(str::trim)
                .filter(|unit| !unit.is_empty());
            (ty_s, unit)
        }
        _ => {
            return Err(FormulaError::msg(format!(
                "variable `{name}` type must be a string or object"
            )));
        }
    };
    let ty = VariableType::parse_name(ty_s).ok_or_else(|| {
        FormulaError::msg(format!(
            "unknown variable type `{ty_s}` for `{name}` (expected {})",
            expected_type_names()
        ))
    })?;
    let accepted_unit = normalize_accepted_unit(ty, unit.unwrap_or(""))?;
    Ok(VariableDef { ty, accepted_unit })
}

fn normalize_accepted_unit(ty: VariableType, unit: &str) -> Result<String, FormulaError> {
    let unit = unit.trim();
    match ty {
        VariableType::Length => {
            let parsed = if unit.is_empty() {
                LengthUnit::Millimetre
            } else {
                length_unit(unit)?
            };
            Ok(parsed.as_str().to_string())
        }
        VariableType::Weight => {
            let parsed = if unit.is_empty() {
                WeightUnit::Kilogram
            } else {
                parse_weight_unit(unit)
                    .ok_or_else(|| FormulaError::msg(format!("unknown weight unit `{unit}`")))?
            };
            Ok(parsed.as_str().to_string())
        }
        VariableType::Quantity => match unit.to_ascii_lowercase().as_str() {
            "" | "1" | "number" | "no." | "no" => Ok("Number".to_string()),
            _ => Err(FormulaError::msg(
                "quantity accepts only unit `Number`".to_string(),
            )),
        },
        VariableType::Decimal => match unit.to_ascii_lowercase().as_str() {
            "" | "1" | "none" | "seconds" | "second" => Ok("None".to_string()),
            _ => Err(FormulaError::msg(
                "decimal accepts only unit `None`".to_string(),
            )),
        },
        _ => {
            let fixed = ty.default_accepted_unit();
            if unit.is_empty() || unit == fixed {
                Ok(fixed.to_string())
            } else {
                Err(FormulaError::msg(format!(
                    "{ty} accepts only unit `{fixed}`"
                )))
            }
        }
    }
}

/// Parse `"name:type"` or `"name:type:unit"` rows from a List widget into a schema.
pub fn parse_schema_list(entries: &[String]) -> Result<VariableSchema, FormulaError> {
    let mut map = serde_json::Map::new();
    for raw in entries {
        let s = raw.trim();
        if s.is_empty() {
            continue;
        }
        let mut parts = s.splitn(3, ':');
        let Some(name) = parts.next() else {
            continue;
        };
        let Some(ty) = parts.next() else {
            return Err(FormulaError::msg(format!(
                "variable `{s}` must be in name:type form (types: {})",
                expected_type_names()
            )));
        };
        let name = name.trim();
        let ty = ty.trim();
        let spec = match parts.next().map(str::trim).filter(|unit| !unit.is_empty()) {
            Some(unit) => serde_json::json!({ "type": ty, "accepted_unit": unit }),
            None => serde_json::Value::String(ty.to_string()),
        };
        map.insert(name.to_string(), spec);
    }
    parse_schema(&serde_json::Value::Object(map))
}

/// Render a schema as sorted `"name:type:unit"` rows for List widgets.
pub fn schema_to_entries(schema: &VariableSchema) -> Vec<String> {
    let mut keys: Vec<_> = schema.keys().cloned().collect();
    keys.sort();
    keys.into_iter()
        .filter_map(|k| {
            schema
                .get(&k)
                .map(|def| format!("{}:{}:{}", k, def.ty.as_str(), def.accepted_unit))
        })
        .collect()
}

pub fn schema_to_json(schema: &VariableSchema) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    let mut keys: Vec<_> = schema.keys().cloned().collect();
    keys.sort();
    for k in keys {
        if let Some(def) = schema.get(&k) {
            map.insert(
                k,
                serde_json::json!({
                    "type": def.ty.as_str(),
                    "accepted_unit": def.accepted_unit,
                }),
            );
        }
    }
    serde_json::Value::Object(map)
}

pub fn standard_sample_values(schema: &VariableSchema) -> VariableValues {
    schema
        .iter()
        .map(|(name, def)| {
            (
                name.clone(),
                Measure {
                    kind: def.ty,
                    value: Decimal::ONE,
                    unit: def.accepted_unit.clone(),
                },
            )
        })
        .collect()
}

pub fn values_to_json(values: &VariableValues) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    let mut keys: Vec<_> = values.keys().cloned().collect();
    keys.sort();
    for k in keys {
        if let Some(v) = values.get(&k) {
            map.insert(k, v.to_json());
        }
    }
    serde_json::Value::Object(map)
}

/// JSON saved with a use site. Each schema variable is a measure object.
pub fn persist_values(schema: &VariableSchema, values: &VariableValues) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    let mut keys: Vec<_> = schema.keys().cloned().collect();
    keys.sort();
    for name in keys {
        if let Some(value) = values.get(&name) {
            map.insert(name, value.to_json());
        }
    }
    serde_json::Value::Object(map)
}

/// Parse raw JSON values against `schema`.
///
/// A measure object keeps its magnitude and unit. A `{value, unit}` object or a
/// scalar uses the schema kind. Bare length is millimetres, bare weight is
/// kilograms, and a bare duration number is seconds.
pub fn parse_values(
    schema: &VariableSchema,
    raw: &serde_json::Value,
) -> Result<VariableValues, FormulaError> {
    let obj = match raw {
        serde_json::Value::Object(m) => m,
        serde_json::Value::String(s) => {
            let parsed: serde_json::Value =
                serde_json::from_str(s.trim()).unwrap_or(serde_json::Value::Null);
            return parse_values(schema, &parsed);
        }
        _ => {
            return Err(FormulaError::msg("variable values must be a JSON object"));
        }
    };
    let mut out = VariableValues::new();
    let mut required = Vec::new();
    let mut invalid = Vec::new();
    let mut names: Vec<&String> = schema.keys().collect();
    names.sort();
    for name in names {
        let Some(def) = schema.get(name) else {
            continue;
        };
        let parsed = match obj.get(name) {
            None => {
                required.push(name.clone());
                continue;
            }
            Some(v) => parse_measure(def.ty, v),
        };
        match parsed {
            Ok(val) => {
                out.insert(name.clone(), val);
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.ends_with(" is required") {
                    required.push(name.clone());
                } else {
                    invalid.push(format!("`{name}`: {msg}"));
                }
            }
        }
    }
    if required.is_empty() && invalid.is_empty() {
        return Ok(out);
    }
    Err(format_variable_errors(&required, &invalid))
}

struct Magnitude {
    raw: String,
    unit: Option<String>,
    kind: Option<VariableType>,
}

fn magnitude_of(value: &serde_json::Value) -> Magnitude {
    if let Some(obj) = value.as_object() {
        let raw = obj.get("value").map(json_to_str).unwrap_or_default();
        let unit = obj
            .get("unit")
            .and_then(|unit| unit.as_str())
            .map(str::trim)
            .filter(|unit| !unit.is_empty())
            .map(str::to_string);
        let kind = obj
            .get("kind")
            .and_then(|kind| kind.as_str())
            .and_then(VariableType::parse_name);
        return Magnitude { raw, unit, kind };
    }
    Magnitude {
        raw: json_to_str(value),
        unit: None,
        kind: None,
    }
}

fn parse_measure(ty: VariableType, value: &serde_json::Value) -> Result<Measure, FormulaError> {
    let mag = magnitude_of(value);
    if let Some(kind) = mag.kind
        && kind != ty
    {
        return Err(FormulaError::msg(format!(
            "kind `{kind}` does not match `{ty}`"
        )));
    }
    match ty {
        VariableType::Length => {
            let unit = mag.unit.unwrap_or_else(|| "mm".to_string());
            let parsed = length_unit(&unit)?;
            let amount = parse_decimal_str(&mag.raw)?;
            Ok(Measure {
                kind: ty,
                value: amount,
                unit: parsed.as_str().to_string(),
            })
        }
        VariableType::Weight => {
            let unit = mag.unit.unwrap_or_else(|| "kg".to_string());
            let parsed = parse_weight_unit(&unit)
                .ok_or_else(|| FormulaError::msg(format!("unknown weight unit `{unit}`")))?;
            Ok(Measure {
                kind: ty,
                value: parse_decimal_str(&mag.raw)?,
                unit: parsed.as_str().to_string(),
            })
        }
        VariableType::Duration => parse_duration_measure(&mag.raw),
        VariableType::Quantity => Ok(Measure {
            kind: ty,
            value: Decimal::from(parse_quantity(&mag.raw)?),
            unit: ty.default_accepted_unit().to_string(),
        }),
        VariableType::Decimal | VariableType::Percent => Ok(Measure {
            kind: ty,
            value: parse_decimal_str(&mag.raw)?,
            unit: ty.default_accepted_unit().to_string(),
        }),
    }
}

fn parse_duration_measure(raw: &str) -> Result<Measure, FormulaError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(FormulaError::msg("duration is required"));
    }
    if let Ok(value) = Decimal::from_str(s) {
        return Ok(Measure {
            kind: VariableType::Duration,
            value,
            unit: "s".to_string(),
        });
    }
    let nanos = parse_duration(s).map_err(FormulaError::msg)?;
    Ok(Measure {
        kind: VariableType::Duration,
        value: duration_nanos_to_seconds(nanos),
        unit: "s".to_string(),
    })
}

/// Convert `measure` into `accepted_unit` for Rune.
pub fn convert_measure(measure: &Measure, accepted_unit: &str) -> Result<Decimal, FormulaError> {
    match measure.kind {
        VariableType::Length => length_in_unit(&measure.value, &measure.unit, accepted_unit),
        VariableType::Weight => weight_in_unit(&measure.value, &measure.unit, accepted_unit),
        VariableType::Duration => {
            if accepted_unit != "s" {
                return Err(FormulaError::msg("duration accepts only unit `s`"));
            }
            Ok(measure.value)
        }
        VariableType::Quantity | VariableType::Decimal | VariableType::Percent => Ok(measure.value),
    }
}

fn length_in_unit(value: &Decimal, from: &str, to: &str) -> Result<Decimal, FormulaError> {
    let mm = parse_length(&value.normalize().to_string(), from)?;
    let to_unit = length_unit(to)?;
    let per_mm = Decimal::from(
        i64::try_from(length::NM_PER_MM).map_err(|_| FormulaError::msg("length overflow"))?,
    );
    let per_to = Decimal::from(
        i64::try_from(to_unit.nm_per_unit()).map_err(|_| FormulaError::msg("length overflow"))?,
    );
    if per_to.is_zero() {
        return Err(FormulaError::msg("length unit has no scale"));
    }
    Ok((mm * per_mm) / per_to)
}

fn weight_in_unit(value: &Decimal, from: &str, to: &str) -> Result<Decimal, FormulaError> {
    let from_unit = parse_weight_unit(from)
        .ok_or_else(|| FormulaError::msg(format!("unknown weight unit `{from}`")))?;
    let to_unit = parse_weight_unit(to)
        .ok_or_else(|| FormulaError::msg(format!("unknown weight unit `{to}`")))?;
    Ok(weight::from_kg(weight::to_kg(*value, from_unit), to_unit))
}

fn backtick_names(names: &[String]) -> String {
    let Some((last, head)) = names.split_last() else {
        return String::new();
    };
    match head {
        [] => format!("`{last}`"),
        [only] => format!("`{only}` and `{last}`"),
        head => {
            let head = head
                .iter()
                .map(|n| format!("`{n}`"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{head}, and `{last}`")
        }
    }
}

fn format_variable_errors(required: &[String], invalid: &[String]) -> FormulaError {
    let mut parts = Vec::new();
    if !required.is_empty() {
        let names = backtick_names(required);
        if required.len() == 1 {
            parts.push(format!("{names} is required"));
        } else {
            parts.push(format!("{names} are required"));
        }
    }
    parts.extend(invalid.iter().cloned());
    FormulaError::msg(parts.join("; "))
}

fn json_to_str(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn length_unit(unit: &str) -> Result<LengthUnit, FormulaError> {
    let unit = unit.trim();
    if unit.is_empty() {
        return Ok(LengthUnit::Millimetre);
    }
    parse_length_unit(unit)
        .ok_or_else(|| FormulaError::msg(format!("unknown length unit `{unit}`")))
}

/// Parse `raw` as a length in `unit` and return canonical millimetres.
pub fn parse_length(raw: &str, unit: &str) -> Result<Decimal, FormulaError> {
    let nm = parse_to_nm(raw, length_unit(unit)?).map_err(FormulaError::msg)?;
    nm_to_mm(nm)
}

fn nm_to_mm(nm: i128) -> Result<Decimal, FormulaError> {
    let Ok(nm) = i64::try_from(nm) else {
        return Err(FormulaError::msg("length overflow"));
    };
    let Ok(per_mm) = i64::try_from(length::NM_PER_MM) else {
        return Err(FormulaError::msg("length overflow"));
    };
    Ok(Decimal::from(nm) / Decimal::from(per_mm))
}

pub fn parse_weight(raw: &str) -> Result<Decimal, FormulaError> {
    parse_decimal_str(raw)
}

pub fn parse_quantity(raw: &str) -> Result<i64, FormulaError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(FormulaError::msg("quantity is required"));
    }
    s.parse::<i64>()
        .map_err(|_err| FormulaError::msg(format!("invalid quantity `{s}`")))
}

/// A bare integer is nanoseconds. Human strings (`"2h"`) use [`parse_duration`].
pub fn parse_duration_nanos(raw: &str) -> Result<i64, FormulaError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(FormulaError::msg("duration is required"));
    }
    if let Ok(n) = s.parse::<i64>() {
        return Ok(n);
    }
    parse_duration(s).map_err(FormulaError::msg)
}

fn parse_decimal_str(raw: &str) -> Result<Decimal, FormulaError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(FormulaError::msg("number is required"));
    }
    Decimal::from_str(s).map_err(|_err| FormulaError::msg(format!("invalid decimal `{s}`")))
}

pub fn duration_nanos_to_seconds(nanos: i64) -> Decimal {
    Decimal::from(nanos) / Decimal::from(NANOS_PER_SECOND)
}

/// Compile and evaluate `formula` with the given schema-ordered values.
pub fn eval_formula(
    schema: &VariableSchema,
    formula: &str,
    values: &VariableValues,
) -> Result<Decimal, FormulaError> {
    if formula.trim().is_empty() {
        return Err(FormulaError::msg("formula is empty"));
    }
    if formula.len() > MAX_SOURCE_BYTES {
        return Err(FormulaError::msg("formula exceeds maximum size"));
    }
    for name in schema.keys() {
        if !values.contains_key(name) {
            return Err(FormulaError::msg(format!(
                "missing value for variable `{name}`"
            )));
        }
    }
    let source = wrap_formula(schema, formula, values)?;
    run_compute(&source)
}

/// Evaluate a formula using standard sample values (1 mm, 1 kg, 1 s, qty 1, decimal 1, percent 1).
pub fn validate_formula(schema: &VariableSchema, formula: &str) -> Result<Decimal, FormulaError> {
    let samples = standard_sample_values(schema);
    eval_formula(schema, formula, &samples)
        .map_err(|e| FormulaError::msg(format!("formula failed with standard sample values: {e}")))
}

fn wrap_formula(
    schema: &VariableSchema,
    formula: &str,
    values: &VariableValues,
) -> Result<String, FormulaError> {
    let trimmed = formula.trim();
    if trimmed.contains("pub fn compute") {
        return Ok(trimmed.to_string());
    }
    let mut keys: Vec<&String> = schema.keys().collect();
    keys.sort();
    let mut body = String::new();
    for name in keys {
        let val = values
            .get(name)
            .ok_or_else(|| FormulaError::msg(format!("missing value for variable `{name}`")))?;
        let def = schema.get(name).ok_or_else(|| {
            FormulaError::msg(format!("missing definition for variable `{name}`"))
        })?;
        if val.kind != def.ty {
            return Err(FormulaError::msg(format!(
                "`{name}` is {}, expected {}",
                val.kind, def.ty
            )));
        }
        let decimal = convert_measure(val, &def.accepted_unit)?;
        bind_decimal(&mut body, name, &decimal);
    }
    Ok(format!("pub fn compute() {{\n{body}    {trimmed}\n}}\n"))
}

fn bind_decimal(body: &mut String, name: &str, value: &Decimal) {
    body.push_str(&format!(
        "    let {name} = decimal(\"{}\");\n",
        value.normalize()
    ));
}

/// Turn base-10 numeric literals into `decimal("...")` calls and expand unary
/// minus on non-literals into `decimal("0") - ...`.
///
/// Rune literals are integers or `f64`. Rewriting the source keeps the digits the
/// user typed. Strings and comments are left alone, so an existing `decimal("1.5")`
/// is not wrapped again. A unary minus on a literal is folded into it (`-1.5`
/// becomes `decimal("-1.5")`) because Rune only negates integers and floats.
/// For everything else Rune has no negation protocol, so `-pre_tax` becomes
/// `(decimal("0") - pre_tax)`.
fn rewrite_decimals(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    let mut last_code: Option<char> = None;
    let mut depth = 0usize;
    let mut negations: Vec<usize> = Vec::new();
    while i < source.len() {
        if tail(source, i).starts_with("//") {
            // A comment ends the operand, so close every expansion first.
            close_negations(&mut out, &mut negations, 0);
            i = copy_line_comment(source, i, &mut out);
            continue;
        }
        if tail(source, i).starts_with("/*") {
            close_negations(&mut out, &mut negations, 0);
            i = copy_block_comment(source, i, &mut out);
            continue;
        }
        let Some(ch) = char_at(source, i) else {
            break;
        };
        if ch == '"' || ch == '\'' {
            i = copy_quoted(source, i, &mut out, ch);
            last_code = Some(ch);
            continue;
        }
        if ch == '-' && is_unary_minus(last_code) {
            if let Some(number) = signed_decimal_literal(source, i) {
                push_decimal_call(&mut out, &number.payload);
                last_code = Some(')');
                i = number.end;
                continue;
            }
            out.push_str("(decimal(\"0\") - ");
            negations.push(depth);
            last_code = Some('-');
            i += ch.len_utf8();
            continue;
        }
        if ch.is_ascii_digit() && !prev_is_ident(source, i) {
            if let Some(end) = skip_radix_literal(source, i) {
                let token = slice(source, i, end);
                note_code(&mut last_code, token);
                out.push_str(token);
                i = end;
                continue;
            }
            let number = scan_decimal_literal(source, i);
            push_decimal_call(&mut out, &number.payload);
            last_code = Some(')');
            i = number.end;
            continue;
        }
        if ends_operand(ch) {
            close_negations(&mut out, &mut negations, depth);
        }
        out.push(ch);
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if !ch.is_whitespace() {
            last_code = Some(ch);
        }
        i += ch.len_utf8();
    }
    close_negations(&mut out, &mut negations, 0);
    out
}

/// Characters that end the operand of an expanded unary minus.
///
/// A closing delimiter is included because `-pre_tax)` has to close both the
/// group and the expansion before the delimiter is copied.
fn ends_operand(ch: char) -> bool {
    matches!(
        ch,
        '+' | '-'
            | '*'
            | '/'
            | '%'
            | '<'
            | '>'
            | '='
            | '!'
            | '&'
            | '|'
            | '^'
            | '~'
            | ','
            | ';'
            | ')'
            | ']'
            | '}'
    )
}

/// Close every expansion that started at or inside `depth`. Depth zero closes
/// all of them, which is what end of input and comments need.
fn close_negations(out: &mut String, negations: &mut Vec<usize>, depth: usize) {
    while negations.last().is_some_and(|open| *open >= depth) {
        negations.pop();
        push_closer(out);
    }
}

/// Push `)` before any whitespace already written, so `-a * b` closes as
/// `(decimal("0") - a) * b` instead of `(decimal("0") - a )* b`.
fn push_closer(out: &mut String) {
    let trimmed = out.trim_end().len();
    let trailing = out.get(trimmed..).unwrap_or("").to_string();
    out.truncate(trimmed);
    out.push(')');
    out.push_str(&trailing);
}

fn tail(source: &str, i: usize) -> &str {
    source.get(i..).unwrap_or("")
}

fn slice(source: &str, start: usize, end: usize) -> &str {
    source.get(start..end).unwrap_or("")
}

fn char_at(source: &str, i: usize) -> Option<char> {
    tail(source, i).chars().next()
}

fn byte_at(source: &str, i: usize) -> Option<u8> {
    source.as_bytes().get(i).copied()
}

fn is_unary_minus(last_code: Option<char>) -> bool {
    match last_code {
        Some(c) if c.is_ascii_alphanumeric() => false,
        Some('_' | ')' | ']' | '}' | '"' | '\'' | '.') => false,
        _ => true,
    }
}

fn signed_decimal_literal(source: &str, minus_at: usize) -> Option<DecimalLiteral> {
    let mut j = minus_at + 1;
    while byte_at(source, j).is_some_and(|c| c.is_ascii_whitespace()) {
        j += 1;
    }
    if !byte_at(source, j).is_some_and(|c| c.is_ascii_digit()) || prev_is_ident(source, j) {
        return None;
    }
    if skip_radix_literal(source, j).is_some() {
        return None;
    }
    let mut number = scan_decimal_literal(source, j);
    number.payload.insert(0, '-');
    Some(number)
}

fn push_decimal_call(out: &mut String, payload: &str) {
    out.push_str("decimal(\"");
    out.push_str(payload);
    out.push_str("\")");
}

fn note_code(last_code: &mut Option<char>, text: &str) {
    if let Some(ch) = text.chars().rev().find(|c| !c.is_whitespace()) {
        *last_code = Some(ch);
    }
}

fn prev_is_ident(source: &str, i: usize) -> bool {
    source
        .get(..i)
        .unwrap_or("")
        .chars()
        .next_back()
        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn copy_line_comment(source: &str, start: usize, out: &mut String) -> usize {
    let rest = tail(source, start);
    let end = rest
        .find('\n')
        .map_or(source.len(), |offset| start + offset + 1);
    out.push_str(slice(source, start, end));
    end
}

fn copy_block_comment(source: &str, start: usize, out: &mut String) -> usize {
    let rest = tail(source, start.saturating_add(2));
    let end = rest
        .find("*/")
        .map_or(source.len(), |offset| start + 2 + offset + 2);
    out.push_str(slice(source, start, end));
    end
}

fn copy_quoted(source: &str, start: usize, out: &mut String, quote: char) -> usize {
    out.push(quote);
    let mut i = start + quote.len_utf8();
    while let Some(ch) = char_at(source, i) {
        out.push(ch);
        i += ch.len_utf8();
        if ch == '\\' {
            if let Some(escaped) = char_at(source, i) {
                out.push(escaped);
                i += escaped.len_utf8();
            }
            continue;
        }
        if ch == quote {
            break;
        }
    }
    i
}

fn skip_radix_literal(source: &str, start: usize) -> Option<usize> {
    if byte_at(source, start) != Some(b'0') {
        return None;
    }
    let kind = byte_at(source, start.saturating_add(1))?;
    let is_digit: fn(u8) -> bool = match kind {
        b'x' | b'X' => |c| c.is_ascii_hexdigit() || c == b'_',
        b'b' | b'B' => |c| c == b'0' || c == b'1' || c == b'_',
        b'o' | b'O' => |c| (b'0'..=b'7').contains(&c) || c == b'_',
        _ => return None,
    };
    let mut end = start + 2;
    while byte_at(source, end).is_some_and(is_digit) {
        end += 1;
    }
    Some(end)
}

struct DecimalLiteral {
    end: usize,
    payload: String,
}

fn scan_decimal_literal(source: &str, start: usize) -> DecimalLiteral {
    let mut end = consume_digits(source, start);
    end = consume_fraction(source, end);
    end = consume_exponent(source, end);
    let mut payload: String = slice(source, start, end)
        .chars()
        .filter(|c| *c != '_')
        .collect();
    if payload.ends_with('.') {
        payload.push('0');
    }
    DecimalLiteral { end, payload }
}

fn consume_digits(source: &str, mut i: usize) -> usize {
    while let Some(c) = byte_at(source, i) {
        if c.is_ascii_digit() || c == b'_' {
            i += 1;
        } else {
            break;
        }
    }
    i
}

fn consume_fraction(source: &str, end: usize) -> usize {
    if !tail(source, end).starts_with('.') {
        return end;
    }
    let after = end + 1;
    match char_at(source, after) {
        Some(c) if c.is_ascii_digit() || c == '_' => consume_digits(source, after),
        Some(c) if c.is_ascii_alphabetic() || c == '.' => end,
        _ => after,
    }
}

fn consume_exponent(source: &str, end: usize) -> usize {
    let Some(marker) = char_at(source, end) else {
        return end;
    };
    if marker != 'e' && marker != 'E' {
        return end;
    }
    let mut j = end + 1;
    let exponent = tail(source, j);
    if exponent.starts_with('+') || exponent.starts_with('-') {
        j += 1;
    }
    let mut saw_digit = false;
    while let Some(c) = byte_at(source, j) {
        if c.is_ascii_digit() {
            saw_digit = true;
            j += 1;
        } else if c == b'_' {
            j += 1;
        } else {
            break;
        }
    }
    if saw_digit { j } else { end }
}

fn run_compute(source: &str) -> Result<Decimal, FormulaError> {
    let source = rewrite_decimals(source);
    let mut context = Context::with_config(false).map_err(|e| FormulaError::msg(e.to_string()))?;
    context
        .install(decimal_module().map_err(|e| FormulaError::msg(e.to_string()))?)
        .map_err(|e| FormulaError::msg(e.to_string()))?;
    let runtime = Arc::new(
        context
            .runtime()
            .map_err(|e| FormulaError::msg(e.to_string()))?,
    );
    let mut sources = Sources::new();
    sources
        .insert(Source::new("formula", source).map_err(|e| FormulaError::msg(e.to_string()))?)
        .map_err(|e| FormulaError::msg(e.to_string()))?;
    let mut diagnostics = Diagnostics::new();
    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();
    if diagnostics.has_error() {
        return Err(FormulaError::msg(format_diagnostics(
            &diagnostics,
            &sources,
        )));
    }
    let unit = result.map_err(|e| FormulaError::msg(e.to_string()))?;
    let mut vm = Vm::new(runtime, Arc::new(unit));
    let output = vm
        .call(["compute"], ())
        .map_err(|e| FormulaError::msg(format_vm_error(&e, &sources)))?;
    value_to_decimal(&output)
}

fn format_diagnostics(diagnostics: &Diagnostics, sources: &Sources) -> String {
    match emit_to_string(|out| diagnostics.emit(out, sources)) {
        Ok(text) if !text.trim().is_empty() => text,
        Ok(_) | Err(_) => "compilation failed".into(),
    }
}

fn format_vm_error(error: &rune::runtime::VmError, sources: &Sources) -> String {
    match emit_to_string(|out| error.emit(out, sources)) {
        Ok(text) if !text.trim().is_empty() => text,
        Ok(_) | Err(_) => error.to_string(),
    }
}

fn emit_to_string<F>(emit: F) -> Result<String, String>
where
    F: FnOnce(&mut NoColor<Vec<u8>>) -> Result<(), rune::diagnostics::EmitError>,
{
    let mut buf = NoColor::new(Vec::new());
    emit(&mut buf).map_err(|e| e.to_string())?;
    String::from_utf8(buf.into_inner()).map_err(|e| e.to_string())
}

fn value_to_decimal(v: &Value) -> Result<Decimal, FormulaError> {
    if let Ok(d) = RuneDecimal::from_value(v.clone()) {
        return Ok(d.0);
    }
    if let Ok(n) = i64::from_value(v.clone()) {
        return Ok(Decimal::from(n));
    }
    Err(FormulaError::msg(
        "formula must return a Decimal (integer results are accepted)",
    ))
}

#[derive(Any, Clone, Debug)]
struct RuneDecimal(Decimal);

fn value_as_decimal(value: Value) -> Result<Decimal, String> {
    if let Ok(d) = RuneDecimal::from_value(value.clone()) {
        return Ok(d.0);
    }
    if let Ok(n) = i64::from_value(value) {
        return Ok(Decimal::from(n));
    }
    Err("expected Decimal or integer (f64 is not allowed)".into())
}

#[rune::function]
fn decimal(s: &str) -> RuneDecimal {
    RuneDecimal(Decimal::from_str(s).expect("valid decimal literal"))
}

#[rune::function(instance, protocol = ADD)]
fn add(a: &RuneDecimal, b: Value) -> RuneDecimal {
    RuneDecimal(a.0 + value_as_decimal(b).expect("add operand"))
}

#[rune::function(instance, protocol = SUB)]
fn sub(a: &RuneDecimal, b: Value) -> RuneDecimal {
    RuneDecimal(a.0 - value_as_decimal(b).expect("sub operand"))
}

#[rune::function(instance, protocol = MUL)]
fn mul(a: &RuneDecimal, b: Value) -> RuneDecimal {
    RuneDecimal(a.0 * value_as_decimal(b).expect("mul operand"))
}

#[rune::function(instance, protocol = DIV)]
#[expect(
    clippy::panic,
    reason = "Rune protocol functions report arithmetic failure by panicking into the VM"
)]
fn div(a: &RuneDecimal, b: Value) -> RuneDecimal {
    let rhs = value_as_decimal(b).expect("div operand");
    if rhs.is_zero() {
        panic!("division by zero");
    }
    RuneDecimal(a.0 / rhs)
}

fn decimal_module() -> Result<Module, rune::ContextError> {
    let mut module = Module::new();
    module.ty::<RuneDecimal>()?;
    module.function_meta(decimal)?;
    module.function_meta(add)?;
    module.function_meta(sub)?;
    module.function_meta(mul)?;
    module.function_meta(div)?;
    Ok(module)
}

pub fn format_values_display(schema: &VariableSchema, values: &VariableValues) -> String {
    let parts = format_value_lines(schema, values);
    if parts.is_empty() {
        "-".into()
    } else {
        parts.join(", ")
    }
}

/// Snake case variable name as title case (`unit_price` → `Unit Price`).
pub fn display_variable_name(name: &str) -> String {
    let titled: Vec<String> = name
        .split('_')
        .filter(|part| !part.is_empty())
        .map(title_word)
        .collect();
    if titled.is_empty() {
        name.to_string()
    } else {
        titled.join(" ")
    }
}

fn title_word(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => {
            let mut out: String = first.to_uppercase().collect();
            out.extend(chars.flat_map(|c| c.to_lowercase()));
            out
        }
        None => String::new(),
    }
}

/// One `name: value` line per variable, in schema order.
pub fn format_value_lines(schema: &VariableSchema, values: &VariableValues) -> Vec<String> {
    if values.is_empty() {
        return Vec::new();
    }
    let mut keys = display_keys(schema, values);
    keys.sort();
    keys.dedup();
    let mut parts = Vec::new();
    for k in keys {
        let Some(v) = values.get(k) else {
            continue;
        };
        let label = display_variable_name(k);
        parts.push(format!("{label}: {}", format_measure(v)));
    }
    parts
}

fn format_measure(measure: &Measure) -> String {
    match measure.kind {
        VariableType::Duration => format_seconds_human(&measure.value),
        VariableType::Percent => format!("{}%", measure.value.normalize()),
        VariableType::Quantity => format!("{} No.", measure.value.normalize()),
        VariableType::Decimal => measure.value.normalize().to_string(),
        VariableType::Length | VariableType::Weight => {
            format!("{} {}", measure.value.normalize(), measure.unit)
        }
    }
}

fn format_seconds_human(total: &Decimal) -> String {
    let total = total.normalize();
    if total.is_zero() {
        return "0 seconds".to_string();
    }
    if total.is_sign_negative() {
        return format!("{total} seconds");
    }
    let units: &[(&str, &str, i64)] = &[
        ("year", "years", 365 * 24 * 3600),
        ("month", "months", 30 * 24 * 3600),
        ("week", "weeks", 7 * 24 * 3600),
        ("day", "days", 24 * 3600),
        ("hour", "hours", 3600),
        ("minute", "minutes", 60),
    ];
    let mut rem = total;
    let mut parts = Vec::new();
    for (singular, plural, size) in units {
        let size = Decimal::from(*size);
        if rem >= size {
            let count = (rem / size).trunc();
            rem -= count * size;
            let label = if count == Decimal::ONE {
                *singular
            } else {
                *plural
            };
            parts.push(format!("{} {label}", count.normalize()));
        }
    }
    if !rem.is_zero() {
        let shown = rem.normalize();
        let label = if shown == Decimal::ONE {
            "second"
        } else {
            "seconds"
        };
        parts.push(format!("{shown} {label}"));
    }
    if parts.is_empty() {
        "0 seconds".to_string()
    } else {
        parts.join(" ")
    }
}

fn display_keys<'a>(schema: &'a VariableSchema, values: &'a VariableValues) -> Vec<&'a String> {
    if schema.is_empty() {
        return values.keys().collect();
    }
    let mut keys: Vec<&String> = schema.keys().filter(|k| values.contains_key(*k)).collect();
    for key in values.keys() {
        if !schema.contains_key(key) {
            keys.push(key);
        }
    }
    keys
}

impl fmt::Display for VariableType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(pairs: &[(&str, VariableType)]) -> VariableSchema {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), VariableDef::new(*v)))
            .collect()
    }

    fn measure(kind: VariableType, value: i64, unit: &str) -> Measure {
        Measure {
            kind,
            value: Decimal::from(value),
            unit: unit.to_string(),
        }
    }

    #[test]
    fn ident_validation() {
        assert!(is_valid_ident("length"));
        assert!(is_valid_ident("qty2"));
        assert!(is_valid_ident("a_b"));
        assert!(is_valid_ident("unit_price_2"));
        assert!(!is_valid_ident("_x"));
        assert!(!is_valid_ident(""));
        assert!(!is_valid_ident("1x"));
        assert!(!is_valid_ident("a-b"));
        assert!(!is_valid_ident("let"));
        assert!(!is_valid_ident("fn"));
    }

    #[test]
    fn parse_schema_rejects_invalid_names() {
        let value = serde_json::json!({ "a-b": "length", "1x": "quantity" });
        let err = parse_schema(&value).unwrap_err().to_string();
        assert!(err.contains(
            "must start with a letter and contain only letters, digits, and underscores"
        ));
    }

    #[test]
    fn parse_schema_accepts_underscores() {
        let value = serde_json::json!({ "unit_price": "decimal" });
        let schema = parse_schema(&value).unwrap();
        assert_eq!(
            schema.get("unit_price").map(|def| def.ty),
            Some(VariableType::Decimal)
        );
    }

    #[test]
    fn eval_underscored_name() {
        let schema = schema(&[("unit_price", VariableType::Decimal)]);
        let mut values = VariableValues::new();
        values.insert(
            "unit_price".into(),
            measure(VariableType::Decimal, 12, "None"),
        );
        let out = eval_formula(&schema, "unit_price * 2", &values).unwrap();
        assert_eq!(out, Decimal::from(24));
    }

    #[test]
    fn eval_length_times_qty() {
        let schema = schema(&[
            ("length", VariableType::Length),
            ("qty", VariableType::Quantity),
        ]);
        let mut values = VariableValues::new();
        values.insert("length".into(), measure(VariableType::Length, 10, "mm"));
        values.insert("qty".into(), measure(VariableType::Quantity, 3, "Number"));
        let out = eval_formula(&schema, "length * qty", &values).unwrap();
        assert_eq!(out, Decimal::from(30));
    }

    #[test]
    fn eval_decimal_division_no_f64() {
        let schema = schema(&[("duration", VariableType::Duration)]);
        let mut values = VariableValues::new();
        values.insert(
            "duration".into(),
            measure(VariableType::Duration, 3600, "s"),
        );
        let out = eval_formula(&schema, "duration / 3600 * 950", &values).unwrap();
        assert_eq!(out, Decimal::from(950));
    }

    #[test]
    fn eval_percent_of_decimal() {
        let schema = schema(&[
            ("amount", VariableType::Decimal),
            ("discount", VariableType::Percent),
        ]);
        let mut values = VariableValues::new();
        values.insert("amount".into(), measure(VariableType::Decimal, 200, "None"));
        values.insert("discount".into(), measure(VariableType::Percent, 18, "%"));
        let out = eval_formula(&schema, "amount * discount / 100", &values).unwrap();
        assert_eq!(out, Decimal::from(36));
    }

    #[test]
    fn eval_fractional_literals_are_exact() {
        let out =
            eval_formula(&VariableSchema::new(), "0.1 + 0.2", &VariableValues::new()).unwrap();
        assert_eq!(out, Decimal::new(3, 1));
    }

    #[test]
    fn eval_fractional_constant_times_variable() {
        let schema = schema(&[("unit_price", VariableType::Decimal)]);
        let mut values = VariableValues::new();
        values.insert(
            "unit_price".into(),
            measure(VariableType::Decimal, 12, "None"),
        );
        let out = eval_formula(&schema, "unit_price * 1.5", &values).unwrap();
        assert_eq!(out, Decimal::from(18));
    }

    #[test]
    fn eval_constant_on_the_left() {
        let schema = schema(&[("unit_price", VariableType::Decimal)]);
        let mut values = VariableValues::new();
        values.insert(
            "unit_price".into(),
            measure(VariableType::Decimal, 12, "None"),
        );
        let out = eval_formula(&schema, "2 * unit_price", &values).unwrap();
        assert_eq!(out, Decimal::from(24));
    }

    #[test]
    fn eval_integer_literals_divide_as_decimals() {
        let schema = schema(&[("amount", VariableType::Decimal)]);
        let mut values = VariableValues::new();
        values.insert("amount".into(), measure(VariableType::Decimal, 200, "None"));
        let out = eval_formula(&schema, "1 / 2 * amount", &values).unwrap();
        assert_eq!(out, Decimal::from(100));
    }

    #[test]
    fn eval_unary_minus_literal() {
        let out = eval_formula(&VariableSchema::new(), "-1.5", &VariableValues::new()).unwrap();
        assert_eq!(out, Decimal::new(-15, 1));
    }

    #[test]
    fn eval_existing_decimal_calls() {
        let out = eval_formula(
            &VariableSchema::new(),
            r#"decimal("0.1") + decimal("0.2")"#,
            &VariableValues::new(),
        )
        .unwrap();
        assert_eq!(out, Decimal::new(3, 1));
        let out = eval_formula(
            &VariableSchema::new(),
            r#"decimal("0") - 1.5"#,
            &VariableValues::new(),
        )
        .unwrap();
        assert_eq!(out, Decimal::new(-15, 1));
    }

    #[test]
    fn eval_underscore_literal() {
        let out = eval_formula(&VariableSchema::new(), "1_000.50", &VariableValues::new()).unwrap();
        assert_eq!(out, Decimal::new(100_050, 2));
    }

    #[test]
    fn rewrite_skips_strings_comments_and_radix() {
        let source = r#"decimal("1.5") + 0x10 // 2.5
/* 3.5 */ 1.foo"#;
        assert_eq!(
            rewrite_decimals(source),
            "decimal(\"1.5\") + 0x10 // 2.5\n/* 3.5 */ decimal(\"1\").foo"
        );
    }

    #[test]
    fn rewrite_expands_negated_variables() {
        assert_eq!(rewrite_decimals("-pre_tax"), "(decimal(\"0\") - pre_tax)");
        assert_eq!(
            rewrite_decimals("-quantity * base_cost"),
            "(decimal(\"0\") - quantity) * base_cost"
        );
        assert_eq!(
            rewrite_decimals("2 * -pre_tax"),
            "decimal(\"2\") * (decimal(\"0\") - pre_tax)"
        );
        assert_eq!(
            rewrite_decimals("-(pre_tax + 5)"),
            "(decimal(\"0\") - (pre_tax + decimal(\"5\")))"
        );
        assert_eq!(
            rewrite_decimals("-f(pre_tax) // credit"),
            "(decimal(\"0\") - f(pre_tax)) // credit"
        );
    }

    #[test]
    fn eval_negated_variable() {
        let schema = schema(&[("pre_tax", VariableType::Decimal)]);
        let mut values = VariableValues::new();
        values.insert("pre_tax".into(), measure(VariableType::Decimal, 20, "None"));
        assert_eq!(
            eval_formula(&schema, "-pre_tax", &values).unwrap(),
            Decimal::from(-20)
        );
        assert_eq!(
            eval_formula(&schema, "-pre_tax / 2", &values).unwrap(),
            Decimal::from(-10)
        );
        assert_eq!(
            eval_formula(&schema, "2 * -pre_tax", &values).unwrap(),
            Decimal::from(-40)
        );
        assert_eq!(
            eval_formula(&schema, "-(pre_tax + 5)", &values).unwrap(),
            Decimal::from(-25)
        );
    }

    #[test]
    fn validate_formula_uses_samples() {
        let schema = schema(&[
            ("length", VariableType::Length),
            ("qty", VariableType::Quantity),
        ]);
        let out = validate_formula(&schema, "length * qty * 85").unwrap();
        assert_eq!(out, Decimal::from(85));
    }

    #[test]
    fn validate_rejects_bad_formula() {
        let schema = schema(&[("length", VariableType::Length)]);
        assert!(validate_formula(&schema, "not_a_variable").is_err());
    }

    #[test]
    fn length_unit_conversion() {
        let mm = parse_length("2", "cm").unwrap();
        assert_eq!(mm, Decimal::from(20));
        let mm = parse_length("1", "in").unwrap();
        assert_eq!(mm, Decimal::new(254, 1));
    }

    #[test]
    fn inch_converts_to_accepted_millimetres() {
        let schema = schema(&[("length", VariableType::Length)]);
        let mut values = VariableValues::new();
        values.insert("length".into(), measure(VariableType::Length, 1, "in"));
        let out = eval_formula(&schema, "length", &values).unwrap();
        assert_eq!(out, Decimal::new(254, 1));
    }

    #[test]
    fn metre_converts_to_accepted_millimetres() {
        let schema = schema(&[("length", VariableType::Length)]);
        let values = parse_values(
            &schema,
            &serde_json::json!({
                "length": {"kind": "length", "value": "1", "unit": "m"}
            }),
        )
        .unwrap();
        assert_eq!(
            values.get("length"),
            Some(&measure(VariableType::Length, 1, "m"))
        );
        let out = eval_formula(&schema, "length", &values).unwrap();
        assert_eq!(out, Decimal::from(1000));
    }

    #[test]
    fn parse_values_keeps_entered_length_unit() {
        let schema = schema(&[("length", VariableType::Length)]);
        let values = parse_values(
            &schema,
            &serde_json::json!({"length": {"value": "2", "unit": "cm"}}),
        )
        .unwrap();
        assert_eq!(
            values.get("length"),
            Some(&measure(VariableType::Length, 2, "cm"))
        );
        let out = eval_formula(&schema, "length", &values).unwrap();
        assert_eq!(out, Decimal::from(20));
    }

    #[test]
    fn parse_values_names_required_variables() {
        let schema = schema(&[
            ("length", VariableType::Length),
            ("qty", VariableType::Quantity),
        ]);
        let err = parse_values(&schema, &serde_json::json!({}))
            .unwrap_err()
            .to_string();
        assert!(err.contains("`length`"), "{err}");
        assert!(err.contains("`qty`"), "{err}");
        assert!(err.contains("are required"), "{err}");
    }

    #[test]
    fn parse_values_names_invalid_variables() {
        let schema = schema(&[("qty", VariableType::Quantity)]);
        let err = parse_values(&schema, &serde_json::json!({"qty": "nope"}))
            .unwrap_err()
            .to_string();
        assert!(err.contains("`qty`"), "{err}");
        assert!(err.contains("invalid quantity"), "{err}");
    }

    #[test]
    fn sample_values() {
        let schema = schema(&[
            ("l", VariableType::Length),
            ("w", VariableType::Weight),
            ("d", VariableType::Duration),
            ("q", VariableType::Quantity),
            ("m", VariableType::Decimal),
            ("p", VariableType::Percent),
        ]);
        let s = standard_sample_values(&schema);
        assert_eq!(s.get("l"), Some(&measure(VariableType::Length, 1, "mm")));
        assert_eq!(s.get("w"), Some(&measure(VariableType::Weight, 1, "kg")));
        assert_eq!(s.get("d"), Some(&measure(VariableType::Duration, 1, "s")));
        assert_eq!(
            s.get("q"),
            Some(&measure(VariableType::Quantity, 1, "Number"))
        );
        assert_eq!(s.get("m"), Some(&measure(VariableType::Decimal, 1, "None")));
        assert_eq!(s.get("p"), Some(&measure(VariableType::Percent, 1, "%")));
    }

    #[test]
    fn format_values_display_duration() {
        let schema = schema(&[("duration", VariableType::Duration)]);
        let mut values = VariableValues::new();
        values.insert(
            "duration".into(),
            measure(VariableType::Duration, 7200, "s"),
        );
        let out = format_values_display(&schema, &values);
        assert_eq!(out, "Duration: 2 hours");
    }

    #[test]
    fn quantity_renders_as_number_abbreviation() {
        let schema = schema(&[("qty", VariableType::Quantity)]);
        let mut values = VariableValues::new();
        values.insert("qty".into(), measure(VariableType::Quantity, 5, "Number"));
        assert_eq!(format_values_display(&schema, &values), "Qty: 5 No.");
    }

    #[test]
    fn decimal_renders_with_an_empty_unit() {
        let schema = parse_schema(&serde_json::json!({
            "time": {"type": "decimal", "accepted_unit": "Seconds"}
        }))
        .unwrap();
        assert_eq!(schema["time"].accepted_unit, "None");
        let raw =
            serde_json::json!({"time": {"kind": "decimal", "value": "90.5", "unit": "Seconds"}});
        let values = parse_values(&schema, &raw).unwrap();
        assert_eq!(values["time"].unit, "None");
        assert_eq!(format_values_display(&schema, &values), "Time: 90.5");
    }

    #[test]
    fn length_and_weight_keep_their_units() {
        let schema = schema(&[
            ("length", VariableType::Length),
            ("mass", VariableType::Weight),
        ]);
        let raw = serde_json::json!({
            "length": {"value": "2", "unit": "cm"},
            "mass": {"value": "1.5", "unit": "kg"},
        });
        let values = parse_values(&schema, &raw).unwrap();
        let stored = persist_values(&schema, &values);
        assert_eq!(
            stored["length"],
            serde_json::json!({"kind": "length", "value": "2", "unit": "cm"})
        );
        assert_eq!(
            stored["mass"],
            serde_json::json!({"kind": "weight", "value": "1.5", "unit": "kg"})
        );
        let again = parse_values(&schema, &stored).unwrap();
        assert_eq!(
            again.get("length"),
            Some(&measure(VariableType::Length, 2, "cm"))
        );
        let text = format_values_display(&schema, &again);
        assert!(text.contains("Length: 2 cm"), "{text}");
        assert!(text.contains("Mass: 1.5 kg"), "{text}");
    }

    #[test]
    fn format_values_display_percent() {
        let schema = schema(&[("discount", VariableType::Percent)]);
        let mut values = VariableValues::new();
        values.insert("discount".into(), measure(VariableType::Percent, 18, "%"));
        let out = format_values_display(&schema, &values);
        assert_eq!(out, "Discount: 18%");
    }

    #[test]
    fn accepted_unit_changes_the_bound_decimal() {
        let mut schema = schema(&[("length", VariableType::Length)]);
        schema.get_mut("length").unwrap().accepted_unit = "cm".into();
        let mut values = VariableValues::new();
        values.insert("length".into(), measure(VariableType::Length, 1, "m"));
        let out = eval_formula(&schema, "length", &values).unwrap();
        assert_eq!(out, Decimal::from(100));
    }

    #[test]
    fn display_variable_name_title_cases_snake_case() {
        assert_eq!(display_variable_name("unit_price"), "Unit Price");
        assert_eq!(display_variable_name("length"), "Length");
        assert_eq!(display_variable_name("line_item_qty"), "Line Item Qty");
    }
}

pub mod variable_schema_input;
pub mod variable_value_input;
