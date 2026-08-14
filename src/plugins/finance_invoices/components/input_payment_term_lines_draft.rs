//! Draft payment term line editor (Alpine + hidden JSON).

use maud::{Markup, html};

use crate::components::attrs::escape_attr;

const PAYMENT_TERM_LINES_ALPINE_METHODS: &str = r#"addLine() {
	this.lines.push({
		date_kind: 'relative',
		due_date: '',
		due_duration: '15 days',
		amount_kind: 'relative',
		amount: '',
		amount_percentage: '100',
	});
},
removeLine(idx) {
	if (!Array.isArray(this.lines) || this.lines.length <= 1) return;
	this.lines.splice(idx, 1);
},
formatPct(line) {
	const p = parseFloat(String(line.amount_percentage ?? '').replace(/,/g, '.'));
	return isNaN(p) ? '—' : String(p) + '%';
}"#;

pub struct InputPaymentTermLinesDraft<'a> {
    pub name: &'a str,
    pub defaults: &'a str,
    pub classes: &'a str,
}

impl Default for InputPaymentTermLinesDraft<'_> {
    fn default() -> Self {
        Self {
            name: "PaymentTermLinesJSON",
            defaults: "[]",
            classes: "w-full",
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
        methods = PAYMENT_TERM_LINES_ALPINE_METHODS.trim_end_matches(',')
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
		date_kind: l.date_kind || 'relative',
		due_date: l.due_date || '',
		due_duration: l.due_duration || '',
		amount_kind: l.amount_kind || 'relative',
		amount: l.amount || '',
		amount_percentage: l.amount_percentage || '',
	}});
	h.value = JSON.stringify(d.lines.map(strip));
}}, true);"#,
        name_q =
            serde_json::to_string(opts.name).unwrap_or_else(|_| "\"PaymentTermLinesJSON\"".into())
    );

    html! {
        div class=(opts.classes) x-data=(alpine_data) x-init=(init_js) {
            input type="hidden" name=(name_escaped) value="" {}
            div class="overflow-x-auto" {
                table class="table table-sm w-full" {
                    thead {
                        tr {
                            th { "Due date" }
                            th { "Amount" }
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
                                            option value="relative" { "Relative (duration)" }
                                            option value="absolute" { "Absolute (date)" }
                                        }
                                        input class="input input-bordered input-sm w-full max-w-xs"
                                            type="text"
                                            placeholder="DD/MM/YYYY"
                                            autocomplete="off"
                                            x-model="line.due_date"
                                            x-show="line.date_kind === 'absolute'" {}
                                        input class="input input-bordered input-sm w-full max-w-xs"
                                            type="text"
                                            placeholder="e.g. 15 days"
                                            x-model="line.due_duration"
                                            x-show="line.date_kind === 'relative'" {}
                                    }
                                }
                                td {
                                    div class="flex flex-col gap-2" {
                                        select class="select select-bordered select-sm w-full max-w-xs"
                                            x-model="line.amount_kind" {
                                            option value="relative" { "Relative (%)" }
                                            option value="absolute" { "Absolute" }
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
}
