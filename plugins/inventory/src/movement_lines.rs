//! Movement line editor (Alpine + hidden JSON), following the invoice line editor.

use lariv_core::components::{InputForeignKey, attrs::escape_attr, input_foreign_key};
use lariv_core::length::LengthUnit;
use lariv_formula::VariableType;
use maud::{Markup, PreEscaped, html};

use crate::logic::line::default_movement_lines_json;
use crate::logic::qty::WeightUnit;
use crate::routes::StockFkSelectRouteTag;

const MOVEMENT_LINES_ALPINE_METHODS: &str = r#"allocFkSlot() {
	if (typeof crypto !== 'undefined' && crypto.randomUUID) return 'MovementLineStock_' + crypto.randomUUID();
	return 'MovementLineStock_' + Math.random().toString(36).slice(2) + '_' + Date.now().toString(36);
},
blankLine() {
	return { stock_id: 0, stock_label: '', fk_slot: this.allocFkSlot(), qty: '', qty_unit: '', qty_type: '' };
},
typeLabel(line) {
	const key = String((line && line.qty_type) || '');
	if (!key) return '—';
	return (this.type_labels && this.type_labels[key]) || key;
},
applyStock(line, detail) {
	const id = parseInt(String(detail.value), 10) || 0;
	line.stock_id = id;
	line.stock_label = detail.display || '';
	if (!id) {
		line.qty_type = '';
		line.qty_unit = '';
		return;
	}
	const ty = String(detail.qty_type || '');
	const unit = String(detail.qty_unit || '');
	line.qty_type = ty;
	if (ty === 'length') {
		line.qty_unit = unit || 'mm';
	} else if (ty === 'weight') {
		line.qty_unit = unit || 'kg';
	} else {
		line.qty_unit = unit;
	}
},
stockPickerData(el) {
	const root = el && el.querySelector('[x-data]');
	return root && window.Alpine ? Alpine.$data(root) : null;
},
bindStockPicker(el, line) {
	const root = el.querySelector('[x-data]');
	const d = this.stockPickerData(el);
	if (!root || !d || typeof d.applySelect !== 'function') {
		const n = Number(el.dataset.pickerTries || 0);
		if (n > 40) return;
		el.dataset.pickerTries = String(n + 1);
		this.$nextTick(() => this.bindStockPicker(el, line));
		return;
	}
	const teleport = root.querySelector('template[x-teleport]');
	const panel = teleport && teleport._x_teleport;
	if (teleport && !panel) {
		const n = Number(el.dataset.pickerTries || 0);
		if (n > 40) return;
		el.dataset.pickerTries = String(n + 1);
		this.$nextTick(() => this.bindStockPicker(el, line));
		return;
	}
	el.dataset.pickerTries = '0';
	const slot = String(line.fk_slot || '');
	if (!slot) return;
	const uid = 'fk-dropdown-' + slot.replace(/[^A-Za-z0-9_-]/g, '-');
	const search = root.querySelector('input[type="search"]');
	const results = panel ? panel.querySelector('.fk-picker-results') : root.querySelector('.fk-picker-results');
	const tableBtn = root.querySelector('button[aria-label="Open selection table"]');
	const setTarget = (node) => {
		if (!node) return;
		const raw = node.getAttribute('hx-get') || '';
		try {
			const u = new URL(raw, window.location.href);
			u.searchParams.set('target_input', slot);
			node.setAttribute('hx-get', u.pathname + u.search + u.hash);
		} catch (e) {}
	};
	if (!d._moveClearWrapped) {
		d._moveClearWrapped = true;
		const origClear = d.clear.bind(d);
		const self = this;
		d.clear = function() {
			origClear.call(this);
			const name = this.fieldName;
			const row = (self.lines || []).find(l => l.fk_slot === name);
			if (row) self.applyStock(row, { value: '', display: '', qty_type: '', qty_unit: '' });
		};
	}
	if (search && search.id !== uid + '-q') {
		search.id = uid + '-q';
		search.setAttribute('hx-target', '#' + uid);
		search.setAttribute('aria-controls', uid);
	}
	if (results && results.id !== uid) results.id = uid;
	setTarget(search);
	setTarget(tableBtn);
	if (search && window.htmx) window.htmx.process(search);
	if (tableBtn && window.htmx) window.htmx.process(tableBtn);
	d.fieldName = slot;
	const nextValue = line.stock_id && Number(line.stock_id) > 0 ? String(line.stock_id) : '';
	const nextDisplay = nextValue ? (line.stock_label || '') : '';
	const valueChanged = String(d.value || '') !== nextValue;
	if (valueChanged) d.value = nextValue;
	if (!nextValue) {
		if (d.display || d.query) {
			d.display = '';
			d.query = '';
			d.open = false;
			if (search) search.value = '';
		}
	} else if (valueChanged || String(d.display || '') !== nextDisplay) {
		d.display = nextDisplay;
		if (!d.open || valueChanged) {
			d.query = nextDisplay;
			d.open = false;
			if (search) search.value = nextDisplay;
		}
	}
}"#;

fn embed_stock_picker(url: &str) -> Markup {
    input_foreign_key(InputForeignKey {
        label: "",
        name: "",
        value: "",
        display: "",
        placeholder: "Select stock…",
        url,
        required: false,
        classes: "!my-0",
        ..Default::default()
    })
}

fn unit_tokens<T: Copy>(units: &[T], token: impl Fn(T) -> &'static str) -> String {
    let names: Vec<&str> = units.iter().copied().map(token).collect();
    serde_json::to_string(&names).unwrap_or_else(|_| "[]".into())
}

fn type_labels_json() -> String {
    let labels: serde_json::Map<String, serde_json::Value> = VariableType::all()
        .iter()
        .map(|ty| {
            (
                ty.as_str().to_string(),
                serde_json::Value::String(ty.label().to_string()),
            )
        })
        .collect();
    serde_json::to_string(&labels).unwrap_or_else(|_| "{}".into())
}

fn unit_options(units: &[&str]) -> Markup {
    html! {
        @for unit in units {
            option value=(*unit) { (*unit) }
        }
    }
}

pub struct InputMovementLines<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub defaults: &'a str,
    pub classes: &'a str,
}

impl Default for InputMovementLines<'_> {
    fn default() -> Self {
        Self {
            label: "Lines",
            name: "lines_json",
            defaults: "[]",
            classes: "w-full",
        }
    }
}

/// Render the movement lines editor. The submitted value is a JSON array.
pub fn input_movement_lines(opts: InputMovementLines<'_>) -> Markup {
    let defaults = if opts.defaults.trim().is_empty()
        || serde_json::from_str::<serde_json::Value>(opts.defaults.trim()).is_err()
    {
        default_movement_lines_json()
    } else {
        opts.defaults.trim().to_string()
    };
    let length_units = unit_tokens(&LengthUnit::ALL, LengthUnit::as_str);
    let weight_units = unit_tokens(&WeightUnit::ALL, WeightUnit::as_str);
    let type_labels = type_labels_json();
    let alpine_data = format!(
        "{{ lines: {defaults}, length_units: {length_units}, weight_units: {weight_units}, type_labels: {type_labels}, {methods} }}",
        methods = MOVEMENT_LINES_ALPINE_METHODS.trim_end_matches(',')
    );
    let name_escaped = escape_attr(opts.name);
    let init_js = format!(
        r#"
(function () {{
	const d = Alpine.$data($el);
	if (!d || !Array.isArray(d.lines)) return;
	if (d.lines.length === 0) d.lines.push(d.blankLine());
	for (const line of d.lines) {{
		if (!line.fk_slot) line.fk_slot = d.allocFkSlot();
		if (line.stock_label == null) line.stock_label = '';
		if (line.qty == null) line.qty = '';
		if (line.qty_unit == null) line.qty_unit = '';
		if (line.qty_type == null) line.qty_type = '';
	}}
}})();
$nextTick(() => {{ if (window.htmx) window.htmx.process($el); }});
$el.closest('form').addEventListener('submit', (ev) => {{
	const d = Alpine.$data($el);
	if (!d || !Array.isArray(d.lines)) return;
	const h = $el.querySelector('input[type="hidden"][name={name_q}]');
	if (!h) return;
	const strip = (l) => ({{
		stock_id: l.stock_id || 0,
		stock_label: l.stock_label || '',
		fk_slot: l.fk_slot || '',
		qty: l.qty || '',
		qty_unit: l.qty_unit || '',
		qty_type: l.qty_type || '',
	}});
	h.value = JSON.stringify(d.lines.map(strip));
}}, true);"#,
        name_q = serde_json::to_string(opts.name).unwrap_or_else(|_| "\"lines_json\"".into())
    );
    let fk_select_handler = r#"if (!$event.detail) return;
	const n = $event.detail.name;
	for (const line of lines) {
		if (!line.fk_slot || line.fk_slot !== n) continue;
		applyStock(line, $event.detail);
		break;
	}"#;
    let x_effect = "lines.length; $nextTick(() => { if (window.htmx) window.htmx.process($el); })";
    let stock_url = StockFkSelectRouteTag.url();
    let length_names: Vec<&str> = LengthUnit::ALL.iter().map(|unit| unit.as_str()).collect();
    let weight_names: Vec<&str> = WeightUnit::ALL.iter().map(|unit| unit.as_str()).collect();
    let length_opts = unit_options(&length_names);
    let weight_opts = unit_options(&weight_names);

    html! {
        div class=(format!("my-1 {}", opts.classes)) {
            @if !opts.label.is_empty() {
                div class="label text-sm font-bold" { (opts.label) }
            }
            (PreEscaped(format!(
                r#"<div data-movement-lines-root="" x-data="{alpine}" x-init="{init}" x-effect="{effect}" @fk-select.window="{fk_sel}" @lariv-fk-created.window="{fk_created}">"#,
                alpine = escape_attr(&alpine_data),
                init = escape_attr(&init_js),
                effect = escape_attr(x_effect),
                fk_sel = escape_attr(fk_select_handler),
                fk_created = escape_attr(&format!(
                    "if ($event) {{ {{ {fk_select_handler} }} }}"
                )),
            )))
            div class="overflow-x-auto min-w-0 rounded-box border border-base-300 bg-base-100" {
                table class="table table-sm min-w-max w-full" {
                    thead {
                        tr {
                            th class="whitespace-nowrap min-w-[16rem]" { "Stock" }
                            th class="whitespace-nowrap min-w-[7rem]" { "Quantity" }
                            th class="whitespace-nowrap min-w-[6rem]" { "Unit" }
                            th class="whitespace-nowrap min-w-[6rem]" { "Type" }
                            th class="whitespace-nowrap min-w-[6rem]" { "Actions" }
                        }
                    }
                    tbody {
                        template x-for="(line, i) in lines" x-bind:key="line.fk_slot" {
                            tr {
                                td class="align-middle min-w-[16rem] max-w-md" {
                                    div class="min-w-0" data-stock-fkey="" x-effect="bindStockPicker($el, line)" {
                                        (embed_stock_picker(&stock_url))
                                    }
                                }
                                td class="align-middle min-w-[7rem]" {
                                    input type="text" class="input input-bordered input-sm w-full min-w-[5rem]"
                                        x-model="line.qty" inputmode="decimal" placeholder="e.g. 1" {}
                                }
                                td class="align-middle min-w-[6rem]" {
                                    template x-if="line.qty_type === 'length'" {
                                        select class="select select-bordered select-sm w-full" x-model="line.qty_unit" {
                                            (length_opts)
                                        }
                                    }
                                    template x-if="line.qty_type === 'weight'" {
                                        select class="select select-bordered select-sm w-full" x-model="line.qty_unit" {
                                            (weight_opts)
                                        }
                                    }
                                    span class="text-sm opacity-60" x-show="line.qty_type !== 'length' && line.qty_type !== 'weight'" { "—" }
                                }
                                td class="align-middle min-w-[6rem]" {
                                    span class="text-sm" x-text="typeLabel(line)" {}
                                }
                                td class="align-middle w-24" {
                                    (PreEscaped(r#"<button type="button" class="btn btn-ghost btn-sm" @click="lines.splice(i, 1); if (lines.length === 0) lines.push(blankLine())">Remove</button>"#))
                                }
                            }
                        }
                    }
                }
            }
            (PreEscaped(r#"<button type="button" class="btn btn-outline btn-sm mt-2 w-full sm:w-auto" @click="lines.push(blankLine())">Add line</button>"#))
            (PreEscaped(format!(
                r#"<input type="hidden" name="{name_escaped}">"#
            )))
            (PreEscaped("</div>"))
        }
    }
}

pub struct MovementLineDisplay<'a> {
    pub stock: &'a str,
    pub qty: &'a str,
    pub unit: &'a str,
    pub qty_type: &'a str,
}

/// Read-only movement lines for the detail page.
pub fn field_movement_lines(rows: &[MovementLineDisplay<'_>]) -> Markup {
    html! {
        div class="w-full min-w-0" {
            div class="overflow-x-auto min-w-0 rounded-box border border-base-300 bg-base-100" {
                table class="table table-sm min-w-max w-full" {
                    thead {
                        tr {
                            th class="whitespace-nowrap min-w-[12rem]" { "Stock" }
                            th class="whitespace-nowrap min-w-[7rem]" { "Quantity" }
                            th class="whitespace-nowrap min-w-[6rem]" { "Unit" }
                            th class="whitespace-nowrap min-w-[6rem]" { "Type" }
                        }
                    }
                    tbody {
                        @if rows.is_empty() {
                            tr {
                                td class="opacity-60" colspan="4" { "No lines" }
                            }
                        } @else {
                            @for row in rows {
                                tr {
                                    td { (row.stock) }
                                    td { (row.qty) }
                                    td { (row.unit) }
                                    td { (row.qty_type) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_lines_editor_lists_length_and_weight_units() {
        let html = input_movement_lines(InputMovementLines::default()).into_string();
        assert!(html.contains("Add line"), "{html}");
        assert!(html.contains("bindStockPicker"), "{html}");
        assert!(html.contains("Select stock"), "{html}");
        assert!(html.contains("value=\"mm\""), "{html}");
        assert!(html.contains("value=\"mg\""), "{html}");
        assert!(html.contains("value=\"lb\""), "{html}");
        assert!(html.contains("value=\"t\""), "{html}");
        assert!(
            !lariv_core::components::attrs::alpine_js_leaked_as_text(&html),
            "Alpine JS rendered as text: {html}"
        );
    }
}
