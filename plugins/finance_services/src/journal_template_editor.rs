//! Alpine editor for a service journal entry template.
//!
//! Posts a JSON array of `{account_id, formula, account_label, fk_slot}`.

use lariv_core::components::{InputForeignKey, attrs::escape_attr, input_foreign_key, label_hint};
use lariv_plugin_finance_accounts::routes::AccountSelectRouteTag;
use maud::{Markup, PreEscaped, html};

use crate::journal_template::empty_lines_json;

const ALPINE_METHODS: &str = r#"allocFkSlot() {
	if (typeof crypto !== 'undefined' && crypto.randomUUID) return 'template-line-' + crypto.randomUUID();
	return 'template-line-' + Math.random().toString(36).slice(2);
},
pickerData(el) {
	const root = el && el.querySelector('[x-data]');
	return root && window.Alpine ? Alpine.$data(root) : null;
},
bindAccountPicker(el, line) {
	const root = el.querySelector('[x-data]');
	const d = this.pickerData(el);
	if (!root || !d || typeof d.applySelect !== 'function') {
		const n = Number(el.dataset.pickerTries || 0);
		if (n > 40) return;
		el.dataset.pickerTries = String(n + 1);
		this.$nextTick(() => this.bindAccountPicker(el, line));
		return;
	}
	const teleport = root.querySelector('template[x-teleport]');
	const panel = teleport && teleport._x_teleport;
	if (teleport && !panel) {
		const n = Number(el.dataset.pickerTries || 0);
		if (n > 40) return;
		el.dataset.pickerTries = String(n + 1);
		this.$nextTick(() => this.bindAccountPicker(el, line));
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
	if (!d._lineClearWrapped) {
		d._lineClearWrapped = true;
		const origClear = d.clear.bind(d);
		const self = this;
		d.clear = function() {
			origClear.call(this);
			const row = (self.lines || []).find(l => l.fk_slot === this.fieldName);
			if (row) {
				row.account_id = 0;
				row.account_label = '';
			}
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
	const nextValue = line.account_id && Number(line.account_id) > 0 ? String(line.account_id) : '';
	const nextDisplay = nextValue ? (line.account_label || '') : '';
	if (String(d.value || '') !== nextValue) d.value = nextValue;
	if (!nextValue) {
		d.display = '';
		d.query = '';
		d.open = false;
		if (search) search.value = '';
	} else if (String(d.display || '') !== nextDisplay) {
		d.display = nextDisplay;
		d.query = nextDisplay;
		d.open = false;
		if (search) search.value = nextDisplay;
	}
},
blankLine() {
	return { account_id: 0, formula: '', account_label: '', fk_slot: this.allocFkSlot() };
},
addLine() {
	this.lines.push(this.blankLine());
	this.$nextTick(() => { if (window.htmx) window.htmx.process(this.$el); });
},
removeLine(i) {
	if (this.lines.length <= 1) return;
	this.lines.splice(i, 1);
}"#;

pub fn input_journal_template_lines(
    name: &str,
    label: &str,
    defaults: &str,
    hint: Option<&str>,
) -> Markup {
    let defaults = if defaults.trim().is_empty() {
        empty_lines_json()
    } else {
        defaults.trim().to_string()
    };
    let pick_url = AccountSelectRouteTag.url();
    let alpine = format!("{{ lines: {defaults}, {ALPINE_METHODS} }}");
    let name_json = serde_json::to_string(name).unwrap_or_else(|_| "\"Lines\"".into());
    let init_js = format!(
        r#"
(function () {{
	const d = Alpine.$data($el);
	if (!d || !Array.isArray(d.lines)) return;
	for (const line of d.lines) {{
		if (!line.fk_slot) line.fk_slot = d.allocFkSlot();
		if (line.account_label == null) line.account_label = '';
		if (line.formula == null) line.formula = '';
	}}
	if (d.lines.length === 0) d.lines.push(d.blankLine());
}})();
$nextTick(() => {{ if (window.htmx) window.htmx.process($el); }});
$el.closest('form').addEventListener('submit', (ev) => {{
	const d = Alpine.$data($el);
	if (!d || !Array.isArray(d.lines)) return;
	const h = $el.querySelector('input[type="hidden"][name={name_json}]');
	if (!h) return;
	h.value = JSON.stringify(d.lines.map((l) => ({{
		account_id: parseInt(String(l.account_id || '0'), 10) || 0,
		formula: l.formula == null ? '' : String(l.formula),
		account_label: l.account_label == null ? '' : String(l.account_label),
		fk_slot: l.fk_slot || '',
	}})));
}}, true);"#,
    );
    let fk_select = r#"if (!$event.detail) return;
	const n = $event.detail.name;
	for (const line of lines) {
		if (!line.fk_slot || line.fk_slot !== n) continue;
		line.account_id = parseInt(String($event.detail.value || '0'), 10) || 0;
		line.account_label = $event.detail.display ? String($event.detail.display) : '';
		break;
	}"#;

    let editor = html! {
            (PreEscaped(format!(
                r#"<div data-journal-template-lines-root="" x-data="{alpine}" x-init="{init}" @fk-select.window="{fk}">"#,
                alpine = escape_attr(&alpine),
                init = escape_attr(&init_js),
                fk = escape_attr(fk_select),
            )))
            (PreEscaped(format!(
                r#"<input type="hidden" name="{}" value="">"#,
                escape_attr(name),
            )))
            div class="overflow-x-auto rounded-box border border-base-300 bg-base-100" {
                table class="table table-sm w-full" {
                    thead {
                        tr {
                            th { "Account" }
                            th { "Formula" }
                            th class="w-24" { "" }
                        }
                    }
                    tbody {
                        template x-for="(line, i) in lines" x-bind:key="line.fk_slot" {
                            tr {
                                td class="align-top min-w-[16rem]" {
                                    div x-effect="bindAccountPicker($el, line)" {
                                        (embed_account_picker(&pick_url))
                                    }
                                }
                                td class="align-top min-w-[16rem]" {
                                    input type="text" class="input input-bordered input-md w-full" placeholder="pre_tax" x-model="line.formula";
                                }
                                td class="align-top" {
                                    (PreEscaped(r#"<button type="button" class="btn btn-ghost btn-sm" @click="removeLine(i)">Remove</button>"#))
                                }
                            }
                        }
                    }
                }
            }
            (PreEscaped(r#"<button type="button" class="btn btn-outline btn-sm mt-2" @click="addLine()">Add line</button>"#))
            (PreEscaped("</div>"))
    };
    if label.is_empty() {
        editor
    } else {
        label_hint(label, hint, editor)
    }
}

fn embed_account_picker(url: &str) -> Markup {
    input_foreign_key(InputForeignKey {
        label: "",
        name: "",
        value: "",
        display: "",
        placeholder: "Select account…",
        url,
        required: false,
        classes: "!my-0",
        ..Default::default()
    })
}
