//! Batch payment allocation editor (Alpine + hidden JSON).

use maud::{Markup, PreEscaped, html};

use crate::components::{
    attrs::escape_attr,
    htmx::{HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL},
    text::icon,
};

const PAYMENT_BATCH_ALLOCATIONS_ALPINE: &str = r#"taxPickHref(rowIdx) {
	const b = this.tax_pick_base || '';
	if (!b) return '#';
	const sep = b.indexOf('?') >= 0 ? '&' : '?';
	const slot = 'BatchAllocTaxes_' + String(rowIdx);
	return b + sep + 'target_input=' + encodeURIComponent(slot);
},
formatDec(n) {
	if (typeof n !== 'number' || !isFinite(n)) return '—';
	let s = n.toFixed(6);
	s = s.replace(/0+$/, '');
	s = s.replace(/\.$/, '');
	return s || '0';
},
parseAmount(s) {
	return parseFloat(String(s || '').trim().replace(/,/g, '.')) || 0;
},
rowWithholdingNumber(row) {
	const base = this.parseAmount(row.amount);
	if (!Array.isArray(row.tax_items) || row.tax_items.length === 0) return 0;
	let sum = 0;
	for (const item of row.tax_items) {
		const id = String(item.Key);
		const pctStr = this.tax_pct_by_id[id];
		const pct = pctStr != null && pctStr !== '' ? parseFloat(String(pctStr)) : NaN;
		if (!isNaN(pct)) sum += base * (pct / 100);
	}
	return sum;
},
rowBankNumber(row) {
	return this.parseAmount(row.amount) - this.rowWithholdingNumber(row);
},
totalSettlementNumber() {
	if (!Array.isArray(this.allocations)) return 0;
	return this.allocations.reduce((s, row) => s + this.parseAmount(row.amount), 0);
},
totalBankNumber() {
	if (!Array.isArray(this.allocations)) return 0;
	return this.allocations.reduce((s, row) => s + this.rowBankNumber(row), 0);
},
syncHidden() {
	const payload = (this.allocations || []).map(row => ({
		posted_invoice_id: row.posted_invoice_id,
		amount: String(row.amount || ''),
		tax_ids: (row.tax_items || []).map(t => parseInt(String(t.Key), 10)).filter(id => !isNaN(id) && id > 0),
	}));
	this.hidden_json = JSON.stringify(payload);
},
taxLabel(item) {
	if (item.Value != null && String(item.Value).trim() !== '') return String(item.Value);
	return 'Tax #' + String(item.Key);
}"#;

pub struct InputPaymentBatchAllocations<'a> {
    pub name: &'a str,
    pub defaults: &'a str,
    pub tax_pick_url: &'a str,
    pub tax_pct_json: &'a str,
    pub all_taxes_json: &'a str,
    pub classes: &'a str,
}

impl Default for InputPaymentBatchAllocations<'_> {
    fn default() -> Self {
        Self {
            name: "allocations_json",
            defaults: "[]",
            tax_pick_url: "/finance-taxes/multi-select",
            tax_pct_json: "{}",
            all_taxes_json: "[]",
            classes: "w-full",
        }
    }
}

pub fn input_payment_batch_allocations(opts: InputPaymentBatchAllocations<'_>) -> Markup {
    let name_escaped = escape_attr(opts.name);
    let cls = escape_attr(opts.classes);
    let tax_pick_base = serde_json::to_string(opts.tax_pick_url).unwrap_or_else(|_| "\"\"".into());
    let alpine_data = format!(
        "{{ allocations: {}, hidden_json: {}, tax_pick_base: {tax_pick_base}, tax_pct_by_id: {}, all_taxes: {}, {methods} }}",
        opts.defaults,
        serde_json::to_string(opts.defaults).unwrap_or_else(|_| "[]".into()),
        opts.tax_pct_json,
        opts.all_taxes_json,
        methods = PAYMENT_BATCH_ALLOCATIONS_ALPINE.trim_end_matches(','),
    );

    let init_js = format!(
        r#"
if (typeof Alpine !== 'undefined' && Alpine.store && !Alpine.store('m2mSelections')) {{
	Alpine.store('m2mSelections', {{}});
}}
(function () {{
	const d = Alpine.$data($el);
	if (!d || !Array.isArray(d.allocations)) return;
	for (const row of d.allocations) {{
		if (!Array.isArray(row.tax_items)) row.tax_items = [];
		const ids = row.tax_ids;
		if (Array.isArray(ids) && ids.length > 0 && row.tax_items.length === 0 && Array.isArray(d.all_taxes)) {{
			for (const tid of ids) {{
				const t = d.all_taxes.find(x => x.id === tid);
				if (t) row.tax_items.push({{ Key: String(t.id), Value: t.name }});
			}}
		}}
		delete row.tax_ids;
	}}
	d.syncHidden();
}})();
$nextTick(() => {{ if (window.htmx) window.htmx.process($el); }});
$el.closest('form').addEventListener('submit', () => {{
	const d = Alpine.$data($el);
	if (d && typeof d.syncHidden === 'function') d.syncHidden();
}}, true);"#,
    );

    let fk_multi_handler = r#"if (!$event.detail) return;
	const n = String($event.detail.name || '');
	const v = $event.detail.value;
	const disp = $event.detail.display || '';
	const m = n.match(/^BatchAllocTaxes_(\d+)$/);
	if (!m) return;
	const idx = parseInt(m[1], 10);
	const row = allocations[idx];
	if (!row) return;
	const value = String(v);
	const items = row.tax_items || (row.tax_items = []);
	const i = items.findIndex(x => x.Key === value);
	if (i >= 0) items.splice(i, 1);
	else items.push({ Key: value, Value: String(disp || value) });
	syncHidden();"#;

    let x_effect =
        "allocations.length; $nextTick(() => { if (window.htmx) window.htmx.process($el); })";

    html! {
        div class=(format!("my-1 {}", cls)) {
            (PreEscaped(format!(
                r#"<div data-batch-alloc-root="" x-data="{alpine}" x-init="{init}" x-effect="{effect}" @fk-multi-select.window="{fk_m2m}">"#,
                alpine = escape_attr(&alpine_data),
                init = escape_attr(&init_js),
                effect = escape_attr(x_effect),
                fk_m2m = escape_attr(fk_multi_handler),
            )))
                input type="hidden" name=(name_escaped) x-model="hidden_json";
                div class="overflow-x-auto min-w-0 rounded-box border border-base-300 bg-base-100" {
                    table class="table table-sm min-w-max w-full" {
                        thead {
                            tr {
                                th { "Invoice" }
                                th { "Customer" }
                                th class="text-end" { "Open balance" }
                                th class="text-end" { "Amount" }
                                th { "Withholding taxes" }
                                th class="text-end" { "Bank net" }
                            }
                        }
                        tbody {
                            template x-for="(row, idx) in allocations" x-bind:key="row.posted_invoice_id" {
                                tr {
                                    td { span x-text="row.invoice_number || ('#' + row.posted_invoice_id)" {} }
                                    td { span x-text="row.customer_name || '—'" {} }
                                    td class="text-end tabular-nums" { span x-text="row.open_balance || '—'" {} }
                                    td class="text-end" {
                                        (PreEscaped(r#"<input type="text" class="input input-bordered input-sm w-28 text-right" x-model="row.amount" @input="syncHidden()" />"#))
                                    }
                                    td {
                                        div class="flex flex-wrap items-center gap-1 min-w-[10rem]" {
                                            template x-for="tItem in (row.tax_items || [])" x-bind:key="tItem.Key" {
                                                (PreEscaped(r#"<div class="flex items-center gap-1 rounded-lg bg-base-200 pl-2 pr-1 py-0.5 max-w-full" @click="$event.stopPropagation()">"#))
                                                span class="text-xs truncate max-w-[8rem]" x-text="taxLabel(tItem)" {}
                                                (PreEscaped(r#"<button type="button" class="btn btn-ghost btn-square btn-xs shrink-0" @click.stop="row.tax_items = (row.tax_items || []).filter(it => it.Key !== tItem.Key); syncHidden()" aria-label="Remove tax">"#))
                                                (icon("x-mark", ""))
                                                (PreEscaped("</button></div>"))
                                            }
                                            (PreEscaped(format!(
                                                r#"<div class="input input-bordered min-h-8 flex items-center cursor-pointer px-2 py-0.5 opacity-70" x-bind:hx-get="taxPickHref(idx)" hx-target="{}" hx-swap="{}" hx-push-url="false"><span class="text-xs">Add tax…</span></div>"#,
                                                HTMX_TARGET_BODY_MODAL,
                                                HTMX_SWAP_BODY_MODAL,
                                            )))
                                        }
                                    }
                                    td class="text-end tabular-nums" {
                                        span x-text="formatDec(rowBankNumber(row))" {}
                                    }
                                }
                            }
                        }
                        tfoot {
                            tr class="font-semibold" {
                                td colspan="3" { "Totals" }
                                td class="text-end tabular-nums" { span x-text="formatDec(totalSettlementNumber())" {} }
                                td {}
                                td class="text-end tabular-nums" { span x-text="formatDec(totalBankNumber())" {} }
                            }
                        }
                    }
                }
            (PreEscaped("</div>"))
        }
    }
}
