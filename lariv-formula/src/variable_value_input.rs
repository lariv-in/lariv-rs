//! One formula-variable value row: name, typed input, and unit.
//!
//! The row object has `.name`, `.type`, `.value`, and `.unit`. `value` is the
//! magnitude in `.unit`. Length and weight keep that pair on the row. The other
//! kinds set their single unit.

use maud::{Markup, PreEscaped, html};

use lariv_core::components::attrs::escape_attr;
use lariv_core::components::{InputLength, input_length};
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

fn embed_length() -> Markup {
    input_length(InputLength {
        label: "",
        name: "",
        value: "",
        unit: "mm",
        required: false,
        classes: "!my-0",
        attrs: Default::default(),
    })
}

fn field_class(compact: bool, width: &str, align_right: bool) -> String {
    if compact {
        let align = if align_right { " text-right" } else { "" };
        format!("input input-xs input-bordered {width}{align} font-mono h-7 min-h-0")
    } else {
        "input input-bordered w-full min-w-[5rem]".to_string()
    }
}

fn length_sync(row: &str, on_input: &str) -> (String, String) {
    let init = format!(
        r#"(() => {{
            const row = {row};
            const boot = () => {{
                const root = $el.querySelector('[x-data]');
                const d = root && window.Alpine ? Alpine.$data(root) : null;
                if (!d || typeof d.syncMm !== 'function') {{
                    const n = Number($el.dataset.prefillTries || 0);
                    if (n > 40) return;
                    $el.dataset.prefillTries = String(n + 1);
                    setTimeout(boot, 16);
                    return;
                }}
                $el.dataset.prefillTries = '0';
                d.unit = row.unit || 'mm';
                d.display = row.value == null ? '' : String(row.value);
                const select = $el.querySelector('select');
                if (select) select.value = d.unit;
                d.syncMm(false);
            }};
            boot();
        }})()"#,
        row = row,
    );
    let pull = format!(
        r#"(() => {{
            const row = {row};
            const root = $el.querySelector('[x-data]');
            const d = root && window.Alpine ? Alpine.$data(root) : null;
            if (!d) return;
            row.value = (d.display == null) ? '' : String(d.display);
            row.unit = d.unit || 'mm';
            {on_input};
        }})()"#,
        row = row,
        on_input = on_input,
    );
    (init, pull)
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
    let length_wrap = if opts.compact {
        "min-w-0"
    } else {
        "min-w-0 flex-1"
    };
    let weight_class = escape_attr(&field_class(opts.compact, "w-24", true));
    let qty_class = escape_attr(&field_class(opts.compact, "w-20", true));
    let duration_class = escape_attr(&field_class(opts.compact, "w-28", false));
    let decimal_class = escape_attr(&field_class(opts.compact, "w-24", true));
    let percent_class = escape_attr(&field_class(opts.compact, "w-20", true));
    let weight_ph = escape_attr(VariableType::Weight.default_placeholder());
    let qty_ph = escape_attr(VariableType::Quantity.default_placeholder());
    let duration_ph = escape_attr(VariableType::Duration.default_placeholder());
    let decimal_ph = escape_attr(VariableType::Decimal.default_placeholder());
    let percent_ph = escape_attr(VariableType::Percent.default_placeholder());
    let (length_init, length_pull) = length_sync(&row, &on_input);
    let weight_options: String = WeightUnit::ALL
        .iter()
        .map(|unit| {
            let selected = if *unit == WeightUnit::Kilogram {
                " selected"
            } else {
                ""
            };
            format!(r#"<option value="{unit}"{selected}>{unit}</option>"#)
        })
        .collect();

    html! {
        (PreEscaped(format!(
            r#"<div class="flex items-center gap-2"><span class="text-xs font-mono font-medium opacity-80 w-24 text-right shrink-0 truncate" x-text="{row}.name + ':'"></span><div class="{length_wrap}" x-show="{row}.type === 'length'" x-cloak x-init="{length_init}" @input="{length_pull}" @change="{length_pull}">"#,
            row = row,
            length_wrap = length_wrap,
            length_init = length_init,
            length_pull = length_pull,
        )))
        (embed_length())
        (PreEscaped(format!(
            r#"</div><template x-if="{row}.type === 'weight'"><div class="flex items-center gap-1 flex-1 min-w-0" x-init="{row}.unit = {row}.unit || 'kg'"><input type="text" inputmode="decimal" class="{weight_class}" x-model="{value}" placeholder="{weight_ph}" {event}="{on_input}"><select class="select select-bordered select-xs w-16" x-model="{row}.unit" @change="{on_input}">{weight_options}</select></div></template><input type="text" inputmode="numeric" class="{qty_class} flex-1" x-show="{row}.type === 'quantity'" x-cloak x-init="{row}.unit = 'Number'" x-model="{value}" placeholder="{qty_ph}" {event}="{on_input}"><input type="text" class="{duration_class} flex-1" x-show="{row}.type === 'duration'" x-cloak x-init="{row}.unit = 's'" x-model="{value}" placeholder="{duration_ph}" {event}="{on_input}"><input type="text" inputmode="decimal" class="{decimal_class} flex-1" x-show="{row}.type === 'decimal'" x-cloak x-init="{row}.unit = 'Seconds'" x-model="{value}" placeholder="{decimal_ph}" {event}="{on_input}"><div class="flex items-center gap-1 flex-1 min-w-0" x-show="{row}.type === 'percent'" x-cloak x-init="{row}.unit = '%'"><input type="text" inputmode="decimal" class="{percent_class}" x-model="{value}" placeholder="{percent_ph}" {event}="{on_input}"><span class="text-xs opacity-60 shrink-0">%</span></div></div>"#,
            row = row,
            weight_class = weight_class,
            value = value,
            weight_ph = weight_ph,
            event = event,
            on_input = on_input,
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
        assert!(html.contains(r#"value="kg" selected>kg</option>"#), "{html}");
        assert!(html.contains("vrow.unit = vrow.unit || 'kg'"), "{html}");
        assert!(html.contains(">%</span>"), "{html}");
        assert!(html.contains("x-model=\"vrow.value\""), "{html}");
        assert!(
            html.contains("@input.debounce.300ms=\"refreshPreTax(line)\""),
            "{html}"
        );
        assert!(html.contains("d.display"), "{html}");
        assert!(!html.contains("bindLengthInput"), "{html}");
    }
}
