//! One formula-variable value row: name, typed input, and unit.
//!
//! The row object has `.name`, `.type`, `.value`, and `.unit`. `value` is the
//! magnitude in `.unit`. Length and weight keep that pair on the row. The other
//! kinds set their single unit.

use maud::{Markup, PreEscaped, html};

use lariv_core::components::attrs::escape_attr;
use lariv_core::length::LengthUnit;
use lariv_core::weight::WeightUnit;

use crate::VariableType;

/// Alpine bindings for [`variable_value_input`].
pub struct VariableValueInput<'a> {
    /// Alpine object with `.name`, `.type`, `.value`, and `.unit` (`vrow`).
    pub row: &'a str,
    /// Model expression (`vrow.value`).
    pub value: &'a str,
    /// Expression run when a value changes (`refreshPreTax(line)`).
    pub on_input: &'a str,
    /// Debounce scalar `@input`. `None` runs `on_input` on every input.
    pub debounce_ms: Option<u32>,
    /// Compact controls for a dense line table.
    pub compact: bool,
}

fn field_class(compact: bool, width: &str, align_right: bool) -> String {
    if compact {
        let align = if align_right { " text-right" } else { "" };
        format!("input input-xs input-bordered {width}{align} font-mono h-7 min-h-0")
    } else {
        "input input-bordered w-full min-w-[5rem]".to_string()
    }
}

fn unit_options(row: &str, units: &[&str], default_unit: &str) -> String {
    units
        .iter()
        .map(|unit| {
            let selected = if *unit == default_unit {
                format!(" :selected=\"{row}.unit === '{unit}' || !{row}.unit\"")
            } else {
                format!(" :selected=\"{row}.unit === '{unit}'\"")
            };
            format!(r#"<option value="{unit}"{selected}>{unit}</option>"#)
        })
        .collect()
}

/// Name label plus a control for each [`VariableType`].
pub fn variable_value_input(opts: VariableValueInput<'_>) -> Markup {
    let row = escape_attr(opts.row);
    let value = escape_attr(opts.value);
    let on_input = escape_attr(opts.on_input);
    let event = match opts.debounce_ms {
        Some(ms) if ms > 0 => format!("@input.debounce.{ms}ms"),
        _ => "@input".to_string(),
    };
    let weight_class = escape_attr(&field_class(opts.compact, "w-24", true));
    let qty_class = escape_attr(&field_class(opts.compact, "w-20", true));
    let duration_class = escape_attr(&field_class(opts.compact, "w-28", false));
    let decimal_class = escape_attr(&field_class(opts.compact, "w-24", true));
    let percent_class = escape_attr(&field_class(opts.compact, "w-20", true));
    let length_class = escape_attr(&field_class(opts.compact, "w-24", true));
    let length_ph = escape_attr(VariableType::Length.default_placeholder());
    let weight_ph = escape_attr(VariableType::Weight.default_placeholder());
    let qty_ph = escape_attr(VariableType::Quantity.default_placeholder());
    let duration_ph = escape_attr(VariableType::Duration.default_placeholder());
    let decimal_ph = escape_attr(VariableType::Decimal.default_placeholder());
    let percent_ph = escape_attr(VariableType::Percent.default_placeholder());
    let length_units = LengthUnit::ALL.map(|unit| unit.as_str());
    let weight_units = WeightUnit::ALL.map(|unit| unit.as_str());
    let length_options = unit_options(&row, &length_units, LengthUnit::Millimetre.as_str());
    let weight_options = unit_options(&row, &weight_units, WeightUnit::Kilogram.as_str());

    html! {
        (PreEscaped(format!(
            r#"<div class="flex items-center gap-2"><span class="text-xs font-mono font-medium opacity-80 w-24 text-right shrink-0 truncate" x-text="{row}.name + ':'"></span><template x-if="{row}.type === 'length'"><div class="flex items-center gap-1 flex-1 min-w-0" x-init="{row}.unit = {row}.unit || 'mm'"><input type="text" inputmode="decimal" class="{length_class}" x-model="{value}" placeholder="{length_ph}" {event}="{on_input}"><select class="select select-bordered select-xs w-16" x-model="{row}.unit" @change="{on_input}" aria-label="Length unit">{length_options}</select></div></template><template x-if="{row}.type === 'weight'"><div class="flex items-center gap-1 flex-1 min-w-0" x-init="{row}.unit = {row}.unit || 'kg'"><input type="text" inputmode="decimal" class="{weight_class}" x-model="{value}" placeholder="{weight_ph}" {event}="{on_input}"><select class="select select-bordered select-xs w-16" x-model="{row}.unit" @change="{on_input}">{weight_options}</select></div></template><template x-if="{row}.type === 'quantity'"><input type="text" inputmode="numeric" class="{qty_class} flex-1" x-init="{row}.unit = 'Number'" x-model="{value}" placeholder="{qty_ph}" {event}="{on_input}"></template><template x-if="{row}.type === 'duration'"><input type="text" class="{duration_class} flex-1" x-init="{row}.unit = 's'" x-model="{value}" placeholder="{duration_ph}" {event}="{on_input}"></template><template x-if="{row}.type === 'decimal'"><input type="text" inputmode="decimal" class="{decimal_class} flex-1" x-init="{row}.unit = 'None'" x-model="{value}" placeholder="{decimal_ph}" {event}="{on_input}"></template><template x-if="{row}.type === 'percent'"><div class="flex items-center gap-1 flex-1 min-w-0" x-init="{row}.unit = '%'"><input type="text" inputmode="decimal" class="{percent_class}" x-model="{value}" placeholder="{percent_ph}" {event}="{on_input}"><span class="text-xs opacity-60 shrink-0">%</span></div></template></div>"#,
            row = row,
            length_class = length_class,
            value = value,
            length_ph = length_ph,
            event = event,
            on_input = on_input,
            length_options = length_options,
            weight_class = weight_class,
            weight_ph = weight_ph,
            weight_options = weight_options,
            qty_class = qty_class,
            qty_ph = qty_ph,
            duration_class = duration_class,
            duration_ph = duration_ph,
            decimal_class = decimal_class,
            decimal_ph = decimal_ph,
            percent_class = percent_class,
            percent_ph = percent_ph,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_shows_name_and_units() {
        let html = variable_value_input(VariableValueInput {
            row: "vrow",
            value: "vrow.value",
            on_input: "refreshPreTax(line)",
            debounce_ms: Some(300),
            compact: false,
        })
        .into_string();
        assert!(html.contains("vrow.name + ':'"), "{html}");
        assert!(html.contains("vrow.type === 'length'"), "{html}");
        assert!(html.contains("Length unit"), "{html}");
        assert!(
            html.contains(":selected=\"vrow.unit === 'kg' || !vrow.unit\""),
            "{html}"
        );
        assert!(
            html.contains(":selected=\"vrow.unit === 'mm' || !vrow.unit\""),
            "{html}"
        );
        assert!(html.contains("vrow.unit = vrow.unit || 'kg'"), "{html}");
        assert!(html.contains(">%</span>"), "{html}");
        assert!(html.contains("x-model=\"vrow.value\""), "{html}");
        assert!(
            html.contains("@input.debounce.300ms=\"refreshPreTax(line)\""),
            "{html}"
        );
        assert!(html.contains("x-model=\"vrow.unit\""), "{html}");
        assert!(html.contains("vrow.unit = vrow.unit || 'mm'"), "{html}");
        assert!(html.contains("x-if=\"vrow.type === 'quantity'\""), "{html}");
        assert!(html.contains("x-if=\"vrow.type === 'duration'\""), "{html}");
        assert!(html.contains("x-if=\"vrow.type === 'decimal'\""), "{html}");
        assert!(html.contains("x-if=\"vrow.type === 'percent'\""), "{html}");
        assert!(
            !html.contains("x-show=\"vrow.type === 'percent'\""),
            "percent unit init must not run for other kinds: {html}"
        );
        assert!(html.contains(r#"value="mm""#), "{html}");
        assert!(!html.contains("bindLengthInput"), "{html}");
        assert!(!html.contains("d.display"), "{html}");
    }
}
