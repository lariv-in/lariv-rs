//! Draft payment term line editor (Alpine + hidden JSON).

use maud::{Markup, html};

use crate::components::attrs::escape_attr;
use crate::plugins::finance_invoices::{PaymentTermAmountKind, PaymentTermDateKind};

#[derive(Clone, Copy)]
pub struct PaymentTermDateKindOption {
    pub value: &'static str,
    pub label: &'static str,
}

pub const INVOICE_PAYMENT_TERM_DATE_KINDS: &[PaymentTermDateKindOption] = &[
    PaymentTermDateKindOption {
        value: PaymentTermDateKind::Relative.as_str(),
        label: "Relative (duration)",
    },
    PaymentTermDateKindOption {
        value: PaymentTermDateKind::Absolute.as_str(),
        label: "Absolute (date)",
    },
];

fn alpine_methods(default_date_kind: &str) -> String {
    let kind = serde_json::to_string(default_date_kind).unwrap_or_else(|_| "\"relative\"".into());
    format!(
        r#"addLine() {{
	this.lines.push({{
		date_kind: {kind},
		due_date: '',
		due_duration: '15 days',
		amount_kind: 'relative',
		amount: '',
		amount_percentage: '100',
	}});
}},
removeLine(idx) {{
	if (!Array.isArray(this.lines) || this.lines.length <= 1) return;
	this.lines.splice(idx, 1);
}},
formatPct(line) {{
	const p = parseFloat(String(line.amount_percentage ?? '').replace(/,/g, '.'));
	return isNaN(p) ? '—' : String(p) + '%';
}},
dueDateIso(line) {{
	const s = String(line.due_date ?? '').trim();
	const dmy = s.match(/^(\d{{2}})\/(\d{{2}})\/(\d{{4}})$/);
	if (dmy) return dmy[3] + '-' + dmy[2] + '-' + dmy[1];
	const iso = s.match(/^(\d{{4}})-(\d{{2}})-(\d{{2}})$/);
	return iso ? s : '';
}},
setDueDateFromIso(line, iso) {{
	if (!iso) {{ line.due_date = ''; return; }}
	line.due_date = String(iso).split('-').reverse().join('/');
}},
openDueDatePicker(line, ev) {{
	const wrap = ev.currentTarget.closest('[data-lariv-date-wrap]');
	if (!wrap) return;
	const picker = wrap.querySelector('[data-lariv-picker]');
	if (!picker) return;
	picker.value = this.dueDateIso(line);
	try {{ picker.showPicker(); }} catch (err) {{ picker.click(); }}
}}"#
    )
}

pub struct InputPaymentTermLinesDraft<'a> {
    pub name: &'a str,
    pub defaults: &'a str,
    pub classes: &'a str,
    pub date_kinds: &'a [PaymentTermDateKindOption],
    pub default_date_kind: &'a str,
}

impl Default for InputPaymentTermLinesDraft<'_> {
    fn default() -> Self {
        Self {
            name: "PaymentTermLinesJSON",
            defaults: "[]",
            classes: "w-full",
            date_kinds: INVOICE_PAYMENT_TERM_DATE_KINDS,
            default_date_kind: PaymentTermDateKind::Relative.as_str(),
        }
    }
}

/// Render the draft payment term lines editor.
pub fn input_payment_term_lines_draft(opts: InputPaymentTermLinesDraft<'_>) -> Markup {
    let defaults = if opts.defaults.trim().is_empty() {
        crate::plugins::finance_invoices::logic::default_payment_term_lines_json()
    } else {
        opts.defaults.trim().to_string()
    };

    let alpine_data = format!(
        "{{ lines: {defaults}, {methods} }}",
        methods = alpine_methods(opts.default_date_kind).trim_end_matches(',')
    );

    let name_escaped = escape_attr(opts.name);
    let init_js = format!(
        r#"
(function () {{
	const d = Alpine.$data($el);
	if (!d || !Array.isArray(d.lines) || d.lines.length === 0) {{
		d.lines = {defaults};
	}}
}})();
$el.closest('form').addEventListener('submit', (ev) => {{
	const d = Alpine.$data($el);
	if (!d || !Array.isArray(d.lines)) return;
	const h = $el.querySelector('input[type="hidden"][name={name_q}]');
	if (!h) return;
	const strip = (l) => ({{
		date_kind: l.date_kind || {default_kind},
		due_date: l.due_date || '',
		due_duration: l.due_duration || '',
		amount_kind: l.amount_kind || 'relative',
		amount: l.amount || '',
		amount_percentage: l.amount_percentage || '',
	}});
	h.value = JSON.stringify(d.lines.map(strip));
}}, true);"#,
        name_q =
            serde_json::to_string(opts.name).unwrap_or_else(|_| "\"PaymentTermLinesJSON\"".into()),
        default_kind =
            serde_json::to_string(opts.default_date_kind).unwrap_or_else(|_| "\"relative\"".into()),
    );

    html! {
        div class=(opts.classes) x-data=(alpine_data) x-init=(init_js) {
            input type="hidden" name=(name_escaped) value="" {}
            div class="overflow-x-auto" {
                table class="table table-sm w-full [&_th]:pl-0 [&_td]:pl-0" {
                    thead {
                        tr {
                            th class="text-xs" { "Due date" }
                            th class="text-xs" { "Amount" }
                            th class="w-12" {}
                        }
                    }
                    tbody {
                        template x-for="(line, idx) in lines" x-bind:key="idx" {
                            tr {
                                td {
                                    div class="flex flex-col gap-2" {
                                        select class="select select-bordered select-sm w-full max-w-xs"
                                            x-model="line.date_kind" {
                                            @for kind in opts.date_kinds {
                                                option value=(kind.value) { (kind.label) }
                                            }
                                        }
                                        div class="join relative w-full max-w-xs" data-lariv-date-wrap="" x-show="line.date_kind === 'absolute'" {
                                            input class="input input-bordered input-sm join-item min-w-0 flex-1"
                                                type="text"
                                                placeholder="DD/MM/YYYY"
                                                autocomplete="off"
                                                x-model="line.due_date" {}
                                            button type="button" class="btn btn-square btn-sm join-item" x-on:click="openDueDatePicker(line, $event)" aria-label="Open date picker" {
                                                (crate::components::text::icon("calendar", "heroicon-sm"))
                                            }
                                            input class="pointer-events-none absolute right-0 top-0 bottom-0 w-10 opacity-0"
                                                type="date"
                                                tabindex="-1"
                                                aria-hidden="true"
                                                data-lariv-picker=""
                                                x-bind:value="dueDateIso(line)"
                                                x-on:change="setDueDateFromIso(line, $event.target.value)" {}
                                        }
                                        input class="input input-bordered input-sm w-full max-w-xs"
                                            type="text"
                                            placeholder="e.g. 15 days"
                                            x-model="line.due_duration"
                                            x-show="line.date_kind !== 'absolute'" {}
                                    }
                                }
                                td {
                                    div class="flex flex-col gap-2" {
                                        select class="select select-bordered select-sm w-full max-w-xs"
                                            x-model="line.amount_kind" {
                                            option value=(PaymentTermAmountKind::Relative.as_str()) { "Relative (%)" }
                                            option value=(PaymentTermAmountKind::Absolute.as_str()) { "Absolute" }
                                        }
                                        input class="input input-bordered input-sm w-full max-w-xs"
                                            type="text"
                                            placeholder="Percentage"
                                            x-model="line.amount_percentage"
                                            x-show="line.amount_kind === 'relative'" {}
                                        input class="input input-bordered input-sm w-full max-w-xs"
                                            type="text"
                                            placeholder="Amount"
                                            x-model="line.amount"
                                            x-show="line.amount_kind === 'absolute'" {}
                                    }
                                }
                                td {
                                    button type="button" class="btn btn-ghost btn-sm"
                                        x-on:click="removeLine(idx)"
                                        x-show="lines.length > 1"
                                        title="Remove line" {
                                        "×"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            button type="button" class="btn btn-sm btn-outline mt-2" x-on:click="addLine()" {
                "+ Add line"
            }
        }
    }
}

pub fn field_payment_term_lines(name: &str, defaults: &str) -> Markup {
    input_payment_term_lines_draft(InputPaymentTermLinesDraft {
        name,
        defaults,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::attrs::alpine_js_leaked_as_text;

    #[test]
    fn payment_term_lines_keeps_alpine_in_attributes() {
        let html = input_payment_term_lines_draft(InputPaymentTermLinesDraft {
            name: "PaymentTermLinesJson",
            defaults: "",
            ..Default::default()
        })
        .into_string();
        assert!(html.contains("x-init="));
        assert!(html.contains("x-data="));
        assert!(
            html.contains("type=&quot;hidden&quot;") || html.contains("type=&#34;hidden&#34;"),
            "x-init quotes must be escaped, got: {html}"
        );
        assert!(
            !html.contains(r#"querySelector('input[type="hidden"]"#),
            "unescaped x-init leaked as text: {html}"
        );
        assert!(
            !alpine_js_leaked_as_text(&html),
            "Alpine JS rendered as text: {html}"
        );
    }

    #[test]
    fn payment_term_lines_renders_custom_date_kinds() {
        const KINDS: &[PaymentTermDateKindOption] = &[
            PaymentTermDateKindOption {
                value: PaymentTermDateKind::RelativeDelivery.as_str(),
                label: "Relative (delivery date)",
            },
            PaymentTermDateKindOption {
                value: PaymentTermDateKind::Absolute.as_str(),
                label: "Absolute (date)",
            },
        ];
        let html = input_payment_term_lines_draft(InputPaymentTermLinesDraft {
            name: "PaymentTermLinesJson",
            defaults: "",
            date_kinds: KINDS,
            default_date_kind: PaymentTermDateKind::RelativeDelivery.as_str(),
            ..Default::default()
        })
        .into_string();
        assert!(html.contains("relative_delivery"));
        assert!(html.contains("Relative (delivery date)"));
    }
}
