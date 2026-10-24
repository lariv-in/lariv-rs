//! Alpine line editor for journal lines.
//!
//! Posts a JSON array of `{account_id, debit, credit, account_label, fk_slot}`
//! and keeps an optional `doc_line` anchor when the row was prefilled from a
//! document line.

use maud::{Markup, PreEscaped, html};

use lariv_core::components::{
    ButtonSubmit, InputForeignKey, Modal, attrs::escape_attr, button_submit, form_hx_post_selector,
    input_foreign_key, modal,
};
use lariv_core::html_form::{CsrfToken, csrf_hidden_field};

use crate::routes::AccountSelectRouteTag;

const ALPINE_METHODS: &str = r#"allocFkSlot() {
	if (typeof crypto !== 'undefined' && crypto.randomUUID) return 'journal-line-' + crypto.randomUUID();
	return 'journal-line-' + Math.random().toString(36).slice(2);
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
	return { account_id: 0, debit: '', credit: '', account_label: '', fk_slot: this.allocFkSlot() };
},
addLine() {
	this.lines.push(this.blankLine());
	this.$nextTick(() => { if (window.htmx) window.htmx.process(this.$el); });
},
removeLine(i) {
	if (this.lines.length <= 1) return;
	this.lines.splice(i, 1);
}"#;

/// Render the account / debit / credit editor. `defaults` is the JSON array.
pub fn input_journal_lines(name: &str, label: &str, defaults: &str) -> Markup {
    let defaults = if defaults.trim().is_empty() {
        crate::logic::custom_source_doc::empty_lines_json()
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
		if (line.debit == null) line.debit = '';
		if (line.credit == null) line.credit = '';
	}}
	if (d.lines.length === 0) d.lines.push(d.blankLine());
}})();
$nextTick(() => {{ if (window.htmx) window.htmx.process($el); }});
$el.closest('form').addEventListener('submit', (ev) => {{
	const d = Alpine.$data($el);
	if (!d || !Array.isArray(d.lines)) return;
	const h = $el.querySelector('input[type="hidden"][name={name_json}]');
	if (!h) return;
	h.value = JSON.stringify(d.lines.map((l) => {{
		const row = {{
			account_id: parseInt(String(l.account_id || '0'), 10) || 0,
			debit: l.debit == null ? '' : String(l.debit),
			credit: l.credit == null ? '' : String(l.credit),
			account_label: l.account_label == null ? '' : String(l.account_label),
			fk_slot: l.fk_slot || '',
		}};
		const doc = parseInt(String(l.doc_line), 10);
		if (l.doc_line != null && l.doc_line !== '' && !Number.isNaN(doc) && doc >= 0) row.doc_line = doc;
		return row;
	}}));
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

    html! {
        div class="my-1" {
            @if !label.is_empty() {
                label class="label text-sm font-bold" { (label) }
            }
            (PreEscaped(format!(
                r#"<div data-journal-lines-root="" x-data="{alpine}" x-init="{init}" @fk-select.window="{fk}">"#,
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
                            th class="w-36" { "Debit" }
                            th class="w-36" { "Credit" }
                            th class="w-24" { "" }
                        }
                    }
                    tbody {
                        template x-for="(line, i) in lines" x-bind:key="line.fk_slot" {
                            tr {
                                td class="align-middle min-w-[16rem]" {
                                    div x-effect="bindAccountPicker($el, line)" {
                                        (embed_account_picker(&pick_url))
                                    }
                                }
                                td class="align-middle" {
                                    input type="text" class="input input-bordered input-sm w-full" inputmode="decimal" placeholder="0" x-model="line.debit" {}
                                }
                                td class="align-middle" {
                                    input type="text" class="input input-bordered input-sm w-full" inputmode="decimal" placeholder="0" x-model="line.credit" {}
                                }
                                td class="align-middle" {
                                    (PreEscaped(r#"<button type="button" class="btn btn-ghost btn-sm" @click="removeLine(i)">Remove</button>"#))
                                }
                            }
                        }
                    }
                }
            }
            (PreEscaped(r#"<button type="button" class="btn btn-outline btn-sm mt-2" @click="addLine()">Add line</button>"#))
            p class="text-sm text-base-content/70 mt-2" { "Debits must equal credits. Use a leaf account on each line." }
            (PreEscaped("</div>"))
        }
    }
}

/// One draft inside a post modal.
pub struct PostJournalSection {
    pub title: String,
    pub field_name: String,
    pub lines_json: String,
    pub error: String,
}

/// Modal that reviews journal lines before they are saved.
///
/// `post_url` is the POST that receives each section's hidden JSON field.
pub fn post_journal_modal(
    uid: &str,
    title: &str,
    intro: &str,
    post_url: &str,
    sections: &[PostJournalSection],
    submit_label: &str,
    form_error: &str,
) -> Markup {
    let can_submit =
        !sections.is_empty() && sections.iter().all(|section| section.error.is_empty());
    let target = format!("#{uid}");
    let form_attrs = form_hx_post_selector(post_url, &target);
    let cancel = format!("document.getElementById('{}').remove()", uid);
    modal(Modal {
        uid,
        classes: "!max-w-6xl w-full",
        children: html! {
            h2 class="text-xl font-bold pr-8" { (title) }
            p class="my-2 text-sm text-base-content/80" { (intro) }
            @if !form_error.is_empty() {
                p class="text-error mb-2" { (form_error) }
            }
            (PreEscaped(format!(
                r#"<form class="flex flex-col gap-4"{}>"#,
                form_attrs.as_string(),
            )))
            (csrf_hidden_field(&CsrfToken::current()))
            @for section in sections {
                div class="rounded-box border border-base-300 bg-base-100 p-3" {
                    h3 class="font-semibold mb-2" { (section.title) }
                    @if section.error.is_empty() {
                        (input_journal_lines(&section.field_name, "Journal lines", &section.lines_json))
                    } @else {
                        p class="text-error" { (section.error) }
                    }
                }
            }
            div class="flex justify-end gap-2" {
                (PreEscaped(format!(
                    r#"<button type="button" class="btn btn-ghost" onclick="{}">Cancel</button>"#,
                    escape_attr(&cancel),
                )))
                @if can_submit {
                    (button_submit(ButtonSubmit {
                        label: submit_label,
                        classes: "btn-primary",
                        ..Default::default()
                    }))
                }
            }
            (PreEscaped("</form>"))
        },
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_posts_json_and_opens_account_picker() {
        let html = input_journal_lines("Lines", "Lines", "").into_string();
        assert!(html.contains(r#"name="Lines""#), "{html}");
        assert!(html.contains("Add line"), "{html}");
        assert!(html.contains("fk-select"), "{html}");
        assert!(html.contains("Select account"), "{html}");
        assert!(html.contains("doc_line"), "{html}");
    }
}
