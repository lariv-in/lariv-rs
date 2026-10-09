//! Typed variables and Rune formula evaluation.
//!
//! A catalog model stores a [`VariableSchema`] and one or more formula strings.
//! Each use site (a quotation line, a work-order line, an invoice line) supplies
//! raw values plus a [`FormulaContext`]. [`parse_values`] turns those into
//! canonical [`VariableValues`]; [`eval_formula`] then returns a [`Decimal`]
//! with no further context.
//!
//! Numerics are [`Decimal`] end-to-end. Length is stored as millimetres and
//! weight as kilograms, each with the unit that was entered so a template can
//! show that unit later. Duration is nanoseconds exposed to Rune as decimal
//! seconds. Quantity is an integer. `decimal` and `percent` are plain decimals;
//! a percent is the number the user typed (`18` for 18%).

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use lariv_core::duration::parse_duration;
use lariv_core::length::{self, LengthUnit, format_mm_as, parse_length_unit, parse_to_nm};
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariableValue {
    /// Canonical millimetres, plus the unit that was entered (`mm`, `cm`, …).
    Length {
        mm: Decimal,
        unit: String,
    },
    /// Canonical kilograms, plus the unit that was entered (`kg`).
    Weight {
        kg: Decimal,
        unit: String,
    },
    DurationNanos(i64),
    Quantity(i64),
    Decimal(Decimal),
    Percent(Decimal),
}

impl VariableValue {
    pub fn ty(&self) -> VariableType {
        match self {
            Self::Length { .. } => VariableType::Length,
            Self::Weight { .. } => VariableType::Weight,
            Self::DurationNanos(_) => VariableType::Duration,
            Self::Quantity(_) => VariableType::Quantity,
            Self::Decimal(_) => VariableType::Decimal,
            Self::Percent(_) => VariableType::Percent,
        }
    }

    pub fn standard_sample(ty: VariableType) -> Self {
        match ty {
            VariableType::Length => Self::Length {
                mm: Decimal::ONE,
                unit: "mm".to_string(),
            },
            VariableType::Weight => Self::Weight {
                kg: Decimal::ONE,
                unit: "kg".to_string(),
            },
            VariableType::Duration => Self::DurationNanos(NANOS_PER_SECOND),
            VariableType::Quantity => Self::Quantity(1),
            VariableType::Decimal => Self::Decimal(Decimal::ONE),
            VariableType::Percent => Self::Percent(Decimal::ONE),
        }
    }

    fn to_json_value(&self) -> serde_json::Value {
        match self {
            Self::Length { mm, unit } => length_json(mm, unit),
            Self::Weight { kg, unit } => weight_json(kg, unit),
            Self::Decimal(d) | Self::Percent(d) => {
                serde_json::Value::String(d.normalize().to_string())
            }
            Self::DurationNanos(n) => serde_json::Value::Number((*n).into()),
            Self::Quantity(n) => serde_json::Value::Number((*n).into()),
        }
    }
}

pub type VariableSchema = HashMap<String, VariableType>;
pub type VariableValues = HashMap<String, VariableValue>;

/// Per-use-site metadata for parsing and display.
///
/// Evaluation does not take this: once values are canonical, the formula only
/// sees decimals. A bare length number uses `length_units`, or millimetres when
/// that name is missing. A `{value, unit}` object carries its own unit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormulaContext {
    pub length_units: HashMap<String, String>,
}

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
    for (name, ty) in obj {
        let name = name.trim();
        if let Some(reason) = ident_rejection(name) {
            return Err(FormulaError::msg(format!(
                "variable name `{name}` {reason}"
            )));
        }
        let ty_s = ty
            .as_str()
            .ok_or_else(|| FormulaError::msg(format!("variable `{name}` type must be a string")))?;
        let parsed = VariableType::parse_name(ty_s).ok_or_else(|| {
            FormulaError::msg(format!(
                "unknown variable type `{ty_s}` for `{name}` (expected {})",
                expected_type_names()
            ))
        })?;
        out.insert(name.to_string(), parsed);
    }
    Ok(out)
}

/// Parse `"name:type"` rows from a List widget into a schema.
pub fn parse_schema_list(entries: &[String]) -> Result<VariableSchema, FormulaError> {
    let mut lines = Vec::new();
    for raw in entries {
        let s = raw.trim();
        if s.is_empty() {
            continue;
        }
        lines.push(s.to_string());
    }
    let mut map = serde_json::Map::new();
    for s in lines {
        let Some((name, ty)) = s.split_once(':') else {
            return Err(FormulaError::msg(format!(
                "variable `{s}` must be in name:type form (types: {})",
                expected_type_names()
            )));
        };
        map.insert(
            name.trim().to_string(),
            serde_json::Value::String(ty.trim().to_string()),
        );
    }
    parse_schema(&serde_json::Value::Object(map))
}

/// Render a schema as sorted `"name:type"` rows for List widgets.
pub fn schema_to_entries(schema: &VariableSchema) -> Vec<String> {
    let mut keys: Vec<_> = schema.keys().cloned().collect();
    keys.sort();
    keys.into_iter()
        .filter_map(|k| schema.get(&k).map(|ty| format!("{}:{}", k, ty.as_str())))
        .collect()
}

pub fn schema_to_json(schema: &VariableSchema) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    let mut keys: Vec<_> = schema.keys().cloned().collect();
    keys.sort();
    for k in keys {
        if let Some(ty) = schema.get(&k) {
            map.insert(k, serde_json::Value::String(ty.as_str().to_string()));
        }
    }
    serde_json::Value::Object(map)
}

pub fn standard_sample_values(schema: &VariableSchema) -> VariableValues {
    schema
        .iter()
        .map(|(name, ty)| (name.clone(), VariableValue::standard_sample(*ty)))
        .collect()
}

pub fn values_to_json(values: &VariableValues) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (k, v) in values {
        map.insert(k.clone(), v.to_json_value());
    }
    serde_json::Value::Object(map)
}

/// JSON saved with a use site.
///
/// Length and weight are rewritten as `{"value","unit"}` from `values`, using
/// the magnitude in that unit. Other keys stay as they were supplied in `raw`.
pub fn persist_values(
    schema: &VariableSchema,
    raw: &serde_json::Value,
    values: &VariableValues,
) -> serde_json::Value {
    let mut map = match raw {
        serde_json::Value::Object(obj) => obj.clone(),
        _ => serde_json::Map::new(),
    };
    for (name, value) in values {
        let Some(ty) = schema.get(name) else {
            continue;
        };
        if matches!(ty, VariableType::Length | VariableType::Weight) {
            map.insert(name.clone(), value.to_json_value());
        }
    }
    serde_json::Value::Object(map)
}

/// Parse raw JSON values against `schema`, using `ctx` for unit conversion.
pub fn parse_values(
    schema: &VariableSchema,
    raw: &serde_json::Value,
    ctx: &FormulaContext,
) -> Result<VariableValues, FormulaError> {
    let obj = match raw {
        serde_json::Value::Object(m) => m,
        serde_json::Value::String(s) => {
            let parsed: serde_json::Value =
                serde_json::from_str(s.trim()).unwrap_or(serde_json::Value::Null);
            return parse_values(schema, &parsed, ctx);
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
        let Some(ty) = schema.get(name).copied() else {
            continue;
        };
        let parsed = match obj.get(name) {
            None => {
                required.push(name.clone());
                continue;
            }
            Some(v) => parse_typed_value(ty, v, name, ctx),
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
    /// Set when the JSON value is `{value, unit}`.
    unit: Option<String>,
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
        return Magnitude { raw, unit };
    }
    Magnitude {
        raw: json_to_str(value),
        unit: None,
    }
}

fn parse_typed_value(
    ty: VariableType,
    value: &serde_json::Value,
    name: &str,
    ctx: &FormulaContext,
) -> Result<VariableValue, FormulaError> {
    let mag = magnitude_of(value);
    match ty {
        VariableType::Length => {
            let unit = mag.unit.unwrap_or_else(|| {
                ctx.length_units
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| "mm".to_string())
            });
            let parsed = length_unit(&unit)?;
            Ok(VariableValue::Length {
                mm: parse_length(&mag.raw, parsed.as_str())?,
                unit: parsed.as_str().to_string(),
            })
        }
        VariableType::Weight => {
            let unit = canonical_weight_unit(mag.unit.as_deref().unwrap_or("kg"))?;
            Ok(VariableValue::Weight {
                kg: parse_weight(&mag.raw)?,
                unit: unit.to_string(),
            })
        }
        VariableType::Duration => Ok(VariableValue::DurationNanos(parse_duration_nanos(
            &mag.raw,
        )?)),
        VariableType::Quantity => Ok(VariableValue::Quantity(parse_quantity(&mag.raw)?)),
        VariableType::Decimal => Ok(VariableValue::Decimal(parse_decimal_str(&mag.raw)?)),
        VariableType::Percent => Ok(VariableValue::Percent(parse_decimal_str(&mag.raw)?)),
    }
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

fn canonical_weight_unit(unit: &str) -> Result<&'static str, FormulaError> {
    match unit.trim().to_ascii_lowercase().as_str() {
        "" | "kg" | "kilogram" | "kilograms" => Ok("kg"),
        other => Err(FormulaError::msg(format!("unknown weight unit `{other}`"))),
    }
}

fn length_json(mm: &Decimal, unit: &str) -> serde_json::Value {
    let parsed = length_unit(unit).unwrap_or(LengthUnit::Millimetre);
    let mm_s = mm.normalize().to_string();
    let shown = format_mm_as(&mm_s, parsed).unwrap_or(mm_s);
    serde_json::json!({
        "value": shown,
        "unit": parsed.as_str(),
    })
}

fn weight_json(kg: &Decimal, unit: &str) -> serde_json::Value {
    let unit = canonical_weight_unit(unit).unwrap_or("kg");
    serde_json::json!({
        "value": kg.normalize().to_string(),
        "unit": unit,
    })
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
        match val {
            VariableValue::Length { mm: d, .. }
            | VariableValue::Weight { kg: d, .. }
            | VariableValue::Decimal(d)
            | VariableValue::Percent(d) => bind_decimal(&mut body, name, d),
            VariableValue::DurationNanos(n) => {
                bind_decimal(&mut body, name, &duration_nanos_to_seconds(*n));
            }
            VariableValue::Quantity(n) => {
                body.push_str(&format!("    let {name} = decimal(\"{n}\");\n"));
            }
        }
    }
    Ok(format!("pub fn compute() {{\n{body}    {trimmed}\n}}\n"))
}

fn bind_decimal(body: &mut String, name: &str, value: &Decimal) {
    body.push_str(&format!(
        "    let {name} = decimal(\"{}\");\n",
        value.normalize()
    ));
}

/// Turn base-10 numeric literals into `decimal("...")` calls.
///
/// Rune literals are integers or `f64`. Rewriting the source keeps the digits the
/// user typed. Strings and comments are left alone, so an existing `decimal("1.5")`
/// is not wrapped again. A unary minus is folded into the literal (`-1.5` becomes
/// `decimal("-1.5")`) because Rune only negates integers and floats.
fn rewrite_numeric_literals(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    let mut last_code: Option<char> = None;
    while i < source.len() {
        if tail(source, i).starts_with("//") {
            i = copy_line_comment(source, i, &mut out);
            continue;
        }
        if tail(source, i).starts_with("/*") {
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
        if ch == '-'
            && is_unary_minus(last_code)
            && let Some(number) = signed_decimal_literal(source, i)
        {
            push_decimal_call(&mut out, &number.payload);
            last_code = Some(')');
            i = number.end;
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
        out.push(ch);
        if !ch.is_whitespace() {
            last_code = Some(ch);
        }
        i += ch.len_utf8();
    }
    out
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
    let source = rewrite_numeric_literals(source);
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

pub fn format_values_display(
    schema: &VariableSchema,
    values: &VariableValues,
    ctx: &FormulaContext,
) -> String {
    let parts = format_value_lines(schema, values, ctx);
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
pub fn format_value_lines(
    schema: &VariableSchema,
    values: &VariableValues,
    _ctx: &FormulaContext,
) -> Vec<String> {
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
        match v {
            VariableValue::Length { mm, unit } => parts.push(format_length_part(&label, mm, unit)),
            VariableValue::Weight { kg, unit } => {
                parts.push(format!("{label}: {} {unit}", kg.normalize()));
            }
            VariableValue::DurationNanos(n) => {
                parts.push(format!(
                    "{label}: {}",
                    lariv_core::duration::format_duration(*n)
                ));
            }
            VariableValue::Quantity(n) => parts.push(format!("{label}: {n}")),
            VariableValue::Decimal(d) => parts.push(format!("{label}: {}", d.normalize())),
            VariableValue::Percent(d) => parts.push(format!("{label}: {}%", d.normalize())),
        }
    }
    parts
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

fn format_length_part(name: &str, mm: &Decimal, unit_token: &str) -> String {
    let mm_s = mm.normalize().to_string();
    let Ok(unit) = length_unit(unit_token) else {
        return format!("{name}: {mm_s} mm");
    };
    match format_mm_as(&mm_s, unit) {
        Ok(user) => format!("{name}: {user} {unit}"),
        Err(_) => format!("{name}: {mm_s} mm"),
    }
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
        pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
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
        assert_eq!(schema.get("unit_price"), Some(&VariableType::Decimal));
    }

    #[test]
    fn eval_underscored_name() {
        let schema = schema(&[("unit_price", VariableType::Decimal)]);
        let mut values = VariableValues::new();
        values.insert(
            "unit_price".into(),
            VariableValue::Decimal(Decimal::from(12)),
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
        values.insert(
            "length".into(),
            VariableValue::Length {
                mm: Decimal::from(10),
                unit: "mm".into(),
            },
        );
        values.insert("qty".into(), VariableValue::Quantity(3));
        let out = eval_formula(&schema, "length * qty", &values).unwrap();
        assert_eq!(out, Decimal::from(30));
    }

    #[test]
    fn eval_decimal_division_no_f64() {
        let schema = schema(&[("duration", VariableType::Duration)]);
        let mut values = VariableValues::new();
        values.insert(
            "duration".into(),
            VariableValue::DurationNanos(3_600_000_000_000),
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
        values.insert("amount".into(), VariableValue::Decimal(Decimal::from(200)));
        values.insert("discount".into(), VariableValue::Percent(Decimal::from(18)));
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
            VariableValue::Decimal(Decimal::from(12)),
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
            VariableValue::Decimal(Decimal::from(12)),
        );
        let out = eval_formula(&schema, "2 * unit_price", &values).unwrap();
        assert_eq!(out, Decimal::from(24));
    }

    #[test]
    fn eval_integer_literals_divide_as_decimals() {
        let schema = schema(&[("amount", VariableType::Decimal)]);
        let mut values = VariableValues::new();
        values.insert("amount".into(), VariableValue::Decimal(Decimal::from(200)));
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
            rewrite_numeric_literals(source),
            "decimal(\"1.5\") + 0x10 // 2.5\n/* 3.5 */ decimal(\"1\").foo"
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
    fn inch_binds_as_millimetres() {
        let mm = parse_length("1", "in").unwrap();
        assert_eq!(mm, Decimal::new(254, 1));
        let schema = schema(&[("length", VariableType::Length)]);
        let mut values = VariableValues::new();
        values.insert(
            "length".into(),
            VariableValue::Length {
                mm,
                unit: "in".into(),
            },
        );
        let out = eval_formula(&schema, "length", &values).unwrap();
        assert_eq!(out, Decimal::new(254, 1));
    }

    #[test]
    fn parse_values_applies_length_unit_context() {
        let schema = schema(&[("length", VariableType::Length)]);
        let mut ctx = FormulaContext::default();
        ctx.length_units.insert("length".into(), "cm".into());
        let values = parse_values(&schema, &serde_json::json!({"length": "2"}), &ctx).unwrap();
        assert_eq!(
            values.get("length"),
            Some(&VariableValue::Length {
                mm: Decimal::from(20),
                unit: "cm".into(),
            })
        );
    }

    #[test]
    fn parse_values_names_required_variables() {
        let schema = schema(&[
            ("length", VariableType::Length),
            ("qty", VariableType::Quantity),
        ]);
        let err = parse_values(&schema, &serde_json::json!({}), &FormulaContext::default())
            .unwrap_err()
            .to_string();
        assert!(err.contains("`length`"), "{err}");
        assert!(err.contains("`qty`"), "{err}");
        assert!(err.contains("are required"), "{err}");
    }

    #[test]
    fn parse_values_names_invalid_variables() {
        let schema = schema(&[("qty", VariableType::Quantity)]);
        let err = parse_values(
            &schema,
            &serde_json::json!({"qty": "nope"}),
            &FormulaContext::default(),
        )
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
        assert_eq!(
            s.get("l"),
            Some(&VariableValue::Length {
                mm: Decimal::ONE,
                unit: "mm".into(),
            })
        );
        assert_eq!(
            s.get("w"),
            Some(&VariableValue::Weight {
                kg: Decimal::ONE,
                unit: "kg".into(),
            })
        );
        assert_eq!(
            s.get("d"),
            Some(&VariableValue::DurationNanos(NANOS_PER_SECOND))
        );
        assert_eq!(s.get("q"), Some(&VariableValue::Quantity(1)));
        assert_eq!(s.get("m"), Some(&VariableValue::Decimal(Decimal::ONE)));
        assert_eq!(s.get("p"), Some(&VariableValue::Percent(Decimal::ONE)));
    }

    #[test]
    fn format_values_display_duration() {
        let schema = schema(&[("duration", VariableType::Duration)]);
        let mut values = VariableValues::new();
        values.insert(
            "duration".into(),
            VariableValue::DurationNanos(7_200_000_000_000),
        );
        let out = format_values_display(&schema, &values, &FormulaContext::default());
        assert_eq!(out, "Duration: 2 hours");
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
        let values = parse_values(&schema, &raw, &FormulaContext::default()).unwrap();
        let stored = persist_values(&schema, &raw, &values);
        assert_eq!(
            stored["length"],
            serde_json::json!({"value": "2", "unit": "cm"})
        );
        assert_eq!(
            stored["mass"],
            serde_json::json!({"value": "1.5", "unit": "kg"})
        );
        let again = parse_values(&schema, &stored, &FormulaContext::default()).unwrap();
        assert_eq!(
            again.get("length"),
            Some(&VariableValue::Length {
                mm: Decimal::from(20),
                unit: "cm".into(),
            })
        );
        let text = format_values_display(&schema, &again, &FormulaContext::default());
        assert!(text.contains("Length: 2 cm"), "{text}");
        assert!(text.contains("Mass: 1.5 kg"), "{text}");
    }

    #[test]
    fn format_values_display_percent() {
        let schema = schema(&[("discount", VariableType::Percent)]);
        let mut values = VariableValues::new();
        values.insert("discount".into(), VariableValue::Percent(Decimal::from(18)));
        let out = format_values_display(&schema, &values, &FormulaContext::default());
        assert_eq!(out, "Discount: 18%");
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
