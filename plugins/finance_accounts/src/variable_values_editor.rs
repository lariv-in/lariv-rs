//! Standard HTML inputs for transaction-template variable values.
//!
//! Renders one labeled control per schema variable and keeps a hidden
//! `VariableValues` JSON field in sync for fill-lines / form round-trips.

use maud::{Markup, PreEscaped, html};

use lariv_core::components::attrs::escape_attr;
use lariv_core::components::{
    HtmlAttrs, InputLength, InputNumber, InputText, input_length, input_number, input_text,
};
use lariv_formula::VariableType;

use crate::variables::{self, VariableInputRow};

const VALUE_PREFIX: &str = "vv__";
const UNIT_PREFIX: &str = "vvunit__";

/// Hidden field name submitted with the create-transaction form.
pub const VARIABLE_VALUES_FIELD: &str = "VariableValues";

fn value_name(var: &str) -> String {
    format!("{VALUE_PREFIX}{var}")
}

fn unit_name(var: &str) -> String {
    format!("{UNIT_PREFIX}{var}")
}

/// Collect `vv__*` / `vvunit__*` (or a `VariableValues` JSON blob) into formula JSON.
pub fn values_json_from_form_map(
    schema_json: &str,
    map: &std::collections::HashMap<String, String>,
) -> String {
    if let Some(raw) = map.get(VARIABLE_VALUES_FIELD) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() && trimmed != "{}" {
            return trimmed.to_string();
        }
    }
    let rows = variables::variable_input_rows(schema_json, "{}");
    let mut out = serde_json::Map::new();
    for row in rows {
        let key = value_name(&row.name);
        let Some(raw) = map.get(&key).map(|s| s.trim()).filter(|s| !s.is_empty()) else {
            continue;
        };
        let unit = map
            .get(&unit_name(&row.name))
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or(row.unit.as_str())
            .to_string();
        out.insert(
            row.name,
            serde_json::json!({
                "kind": row.ty,
                "value": raw,
                "unit": unit,
            }),
        );
    }
    serde_json::Value::Object(out).to_string()
}

fn unit_select(name: &str, selected: &str, choices: &[(&str, &str)]) -> Markup {
    html! {
        select class="select select-bordered w-28 shrink-0" name=(name) {
            @for (value, label) in choices {
                option value=(value) selected[selected == *value] { (label) }
            }
        }
    }
}

fn render_row(row: &VariableInputRow) -> Markup {
    let vname = value_name(&row.name);
    let uname = unit_name(&row.name);
    let label = row.name.as_str();
    match VariableType::parse_name(&row.ty) {
        Some(VariableType::Length) => {
            let unit = if row.unit.trim().is_empty() {
                "mm"
            } else {
                row.unit.as_str()
            };
            // input_length always submits millimetres under `name`.
            input_length(InputLength {
                label,
                name: &vname,
                value: &row.value,
                unit,
                required: true,
                ..Default::default()
            })
        }
        Some(VariableType::Weight) => {
            let choices = VariableType::Weight.unit_choices().unwrap_or(&[]);
            let unit = if row.unit.trim().is_empty() {
                "kg"
            } else {
                row.unit.as_str()
            };
            let ph = VariableType::Weight.default_placeholder();
            html! {
                div class="my-1" {
                    label class="label text-sm font-bold flex flex-col items-start gap-1 w-full" {
                        span { (label) }
                        div class="flex gap-2 w-full" {
                            (PreEscaped(format!(
                                r#"<input type="text" inputmode="decimal" class="input input-bordered min-w-0 flex-1" name="{}" value="{}" placeholder="{}" required autocomplete="off">"#,
                                escape_attr(&vname),
                                escape_attr(&row.value),
                                escape_attr(ph),
                            )))
                            (unit_select(&uname, unit, choices))
                        }
                    }
                }
            }
        }
        Some(VariableType::Quantity) => input_number(InputNumber {
            label,
            name: &vname,
            value: &row.value,
            required: true,
            attrs: HtmlAttrs::new()
                .set("placeholder", VariableType::Quantity.default_placeholder()),
            ..Default::default()
        }),
        Some(VariableType::Percent) => {
            let ph = VariableType::Percent.default_placeholder();
            html! {
                div class="my-1" {
                    label class="label text-sm font-bold flex flex-col items-start gap-1 w-full" {
                        span { (label) }
                        div class="flex items-center gap-2 w-full" {
                            (PreEscaped(format!(
                                r#"<input type="text" inputmode="decimal" class="input input-bordered min-w-0 flex-1" name="{}" value="{}" placeholder="{}" required autocomplete="off">"#,
                                escape_attr(&vname),
                                escape_attr(&row.value),
                                escape_attr(ph),
                            )))
                            span class="text-sm opacity-60 shrink-0" { "%" }
                            input type="hidden" name=(uname) value="%" {}
                        }
                    }
                }
            }
        }
        Some(VariableType::Duration) => input_text(InputText {
            label,
            name: &vname,
            value: &row.value,
            required: true,
            attrs: HtmlAttrs::new()
                .set("placeholder", VariableType::Duration.default_placeholder()),
            ..Default::default()
        }),
        Some(VariableType::Decimal) | None => input_text(InputText {
            label,
            name: &vname,
            value: &row.value,
            required: true,
            attrs: HtmlAttrs::new()
                .set("inputmode", "decimal")
                .set("placeholder", VariableType::Decimal.default_placeholder()),
            ..Default::default()
        }),
    }
}

fn sync_alpine(rows_json: &str) -> String {
    // Keep VariableValues JSON aligned with the standard inputs for fill-lines.
    format!(
        r#"{{
	rows: {rows_json},
	sync() {{
		const root = this.$el;
		const hidden = root.querySelector('input[type="hidden"][name="{field}"]');
		if (!hidden) return;
		const values = {{}};
		for (const row of (this.rows || [])) {{
			const name = String(row.name || '');
			if (!name) continue;
			const type = String(row.type || '').toLowerCase();
			const valueEl = root.querySelector('[name="vv__' + name + '"]');
			const unitEl = root.querySelector('[name="vvunit__' + name + '"]');
			let value = valueEl ? String(valueEl.value || '').trim() : '';
			if (!value) continue;
			let unit = unitEl ? String(unitEl.value || '').trim() : String(row.unit || '');
			if (type === 'length' && !unit) unit = 'mm';
			if (type === 'weight' && !unit) unit = 'kg';
			if (type === 'percent') unit = '%';
			if (type === 'quantity') unit = 'Number';
			if (type === 'duration') unit = 's';
			if (type === 'decimal' && !unit) unit = 'None';
			values[name] = {{ kind: type, value: value, unit: unit }};
		}}
		hidden.value = JSON.stringify(values);
	}}
}}"#,
        rows_json = rows_json,
        field = VARIABLE_VALUES_FIELD,
    )
}

/// Variable value fields for a template schema, plus hidden JSON for round-trips.
pub fn input_variable_values_fields(schema_json: &str, values_json: &str) -> Markup {
    let rows = variables::variable_input_rows(schema_json, values_json);
    if rows.is_empty() {
        return html! {
            div id="transaction-template-variables" class="my-1" {
                input type="hidden" name=(VARIABLE_VALUES_FIELD) value="{}" {}
                p class="text-sm opacity-60" { "This template has no variables." }
            }
        };
    }
    let rows_json = serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into());
    let values = if values_json.trim().is_empty() {
        "{}"
    } else {
        values_json.trim()
    };
    let alpine = sync_alpine(&rows_json);
    html! {
        (PreEscaped(format!(
            r#"<div id="transaction-template-variables" class="my-1" x-data="{}" x-init="sync()" @input="sync()" @change="sync()">"#,
            escape_attr(&alpine),
        )))
        label class="label text-sm font-bold" { "Variable values" }
        input type="hidden" name=(VARIABLE_VALUES_FIELD) value=(values) {}
        (container_rows(&rows))
        (PreEscaped("</div>"))
    }
}

fn container_rows(rows: &[VariableInputRow]) -> Markup {
    html! {
        div class="flex flex-col gap-1" {
            @for row in rows {
                (render_row(row))
            }
        }
    }
}

/// Empty placeholder when no template is selected.
pub fn empty_variable_values_fields() -> Markup {
    html! {
        div id="transaction-template-variables" class="my-1" {
            input type="hidden" name=(VARIABLE_VALUES_FIELD) value="{}" {}
        }
    }
}
