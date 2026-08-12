//! Draft invoice line editor (Alpine + hidden JSON), ported from Go `InputInvoiceLinesDraft`.

use maud::{Markup, PreEscaped, html};

use crate::components::{
    attrs::escape_attr,
    htmx::{HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL},
    text::icon,
};

use crate::plugins::finance_invoices::logic::invoice_line_editor::InvoiceLineDisplayRow;

const INVOICE_LINES_DRAFT_ALPINE_METHODS: &str = r#"allocFkSlot() {
	if (typeof crypto !== 'undefined' && crypto.randomUUID) return 'InvoiceLineProduct_' + crypto.randomUUID();
	return 'InvoiceLineProduct_' + Math.random().toString(36).slice(2) + '_' + Date.now().toString(36);
},
productPickHref(slot) {
	const b = this.product_pick_base || '';
	if (!b || !slot) return b || '#';
	const sep = b.indexOf('?') >= 0 ? '&' : '?';
	return b + sep + 'target_input=' + encodeURIComponent(String(slot));
},
lineTaxPickHref(fkSlot) {
	const b = this.tax_pick_base || '';
	if (!b || !fkSlot) return b || '#';
	const sep = b.indexOf('?') >= 0 ? '&' : '?';
	const name = 'InvoiceLineTaxes_' + String(fkSlot);
	return b + sep + 'target_input=' + encodeURIComponent(name);
},
formatDec(n) {
	if (typeof n !== 'number' || !isFinite(n)) return '—';
	let s = n.toFixed(6);
	s = s.replace(/0+$/, '');
	s = s.replace(/\.$/, '');
	return s || '0';
},
lineUntaxedNumber(line) {
	const q = parseFloat(String(line.quantity).replace(/,/g, '.')) || 0;
	const rate = parseFloat(String(line.rate ?? '').trim().replace(/,/g, '.')) || 0;
	return q * rate;
},
taxKindForId(id) {
	const k = this.tax_kind_by_id && this.tax_kind_by_id[id];
	return k === 'withholding' ? 'withholding' : 'levied';
},
lineTaxAmountForKind(line, kind) {
	const base = this.lineUntaxedNumber(line);
	if (!Array.isArray(line.line_taxes) || line.line_taxes.length === 0) return 0;
	let sum = 0;
	for (const t of line.line_taxes) {
		const id = String(t.Key);
		if (this.taxKindForId(id) !== kind) continue;
		const pctStr = this.tax_pct_by_id[id];
		const pct = pctStr != null && pctStr !== '' ? parseFloat(String(pctStr)) : NaN;
		if (!isNaN(pct)) {
			sum += base * (pct / 100);
		}
	}
	return sum;
},
lineLeviedTaxNumber(line) {
	return this.lineTaxAmountForKind(line, 'levied');
},
lineWithholdingTaxNumber(line) {
	return this.lineTaxAmountForKind(line, 'withholding');
},
lineUntaxedDisplay(line) {
	const u = this.lineUntaxedNumber(line);
	if (!line.product_id && u === 0) return '—';
	return this.formatDec(u);
},
lineLeviedTaxDisplay(line) {
	const u = this.lineUntaxedNumber(line);
	const tax = this.lineLeviedTaxNumber(line);
	if (!line.product_id && u === 0 && tax === 0) return '—';
	return this.formatDec(tax);
},
lineWithholdingDisplay(line) {
	const wh = this.lineWithholdingTaxNumber(line);
	if (wh === 0) return '—';
	return '(' + this.formatDec(wh) + ')';
},
lineTotal(line) {
	const u = this.lineUntaxedNumber(line);
	const lev = this.lineLeviedTaxNumber(line);
	const wh = this.lineWithholdingTaxNumber(line);
	const tot = u + lev - wh;
	if (!line.product_id && tot === 0) return '—';
	return this.formatDec(tot);
},
lineTotalNumber(line) {
	return this.lineUntaxedNumber(line) + this.lineLeviedTaxNumber(line) - this.lineWithholdingTaxNumber(line);
},
linesUntaxedSubtotalNumber() {
	if (!Array.isArray(this.lines)) return 0;
	let sum = 0;
	for (const line of this.lines) {
		sum += this.lineUntaxedNumber(line);
	}
	return sum;
},
linesLeviedSubtotalNumber() {
	if (!Array.isArray(this.lines)) return 0;
	let sum = 0;
	for (const line of this.lines) {
		sum += this.lineLeviedTaxNumber(line);
	}
	return sum;
},
linesWithholdingSubtotalNumber() {
	if (!Array.isArray(this.lines)) return 0;
	let sum = 0;
	for (const line of this.lines) {
		sum += this.lineWithholdingTaxNumber(line);
	}
	return sum;
},
linesSubtotalNumber() {
	return this.linesUntaxedSubtotalNumber() + this.linesLeviedSubtotalNumber();
},
linesSubtotalDisplay() {
	if (!Array.isArray(this.lines) || this.lines.length === 0) return '—';
	return this.formatDec(this.linesSubtotalNumber());
},
invoiceTaxLabel(item) {
	if (item.Value != null && String(item.Value).trim() !== '') {
		return String(item.Value);
	}
	return 'Tax #' + String(item.Key);
},
invoiceHeaderTaxAmountForKind(kind) {
	const base = this.linesUntaxedSubtotalNumber();
	const st = typeof Alpine !== 'undefined' && Alpine.store && Alpine.store('m2mSelections');
	const sel = st && st.Taxes;
	if (!sel || !Array.isArray(sel)) {
		return 0;
	}
	let sum = 0;
	for (const item of sel) {
		const id = String(item.Key);
		if (this.taxKindForId(id) !== kind) continue;
		const pctStr = this.tax_pct_by_id[id];
		const pct = pctStr != null && pctStr !== '' ? parseFloat(String(pctStr)) : NaN;
		if (!isNaN(pct)) {
			sum += base * (pct / 100);
		}
	}
	return sum;
},
invoiceTaxAmountDisplay(item) {
	const base = this.linesUntaxedSubtotalNumber();
	const id = String(item.Key);
	const kind = this.taxKindForId(id);
	const pctStr = this.tax_pct_by_id[id];
	const pct = pctStr != null && pctStr !== '' ? parseFloat(String(pctStr)) : NaN;
	if (isNaN(pct)) {
		return '—';
	}
	const amt = base * (pct / 100);
	if (kind === 'withholding') {
		return '(' + this.formatDec(amt) + ')';
	}
	return this.formatDec(amt);
},
invoiceGrandTotalDisplay() {
	const sub = this.linesSubtotalNumber();
	const headerLevied = this.invoiceHeaderTaxAmountForKind('levied');
	const headerWithholding = this.invoiceHeaderTaxAmountForKind('withholding');
	const lineWh = this.linesWithholdingSubtotalNumber();
	const total = sub + headerLevied - lineWh - headerWithholding;
	const st = typeof Alpine !== 'undefined' && Alpine.store && Alpine.store('m2mSelections');
	const sel = st && st.Taxes;
	const hasTaxes = sel && Array.isArray(sel) && sel.length > 0;
	const hasLines = Array.isArray(this.lines) && this.lines.length > 0;
	if (!hasLines && !hasTaxes && total === 0) {
		return '—';
	}
	return this.formatDec(total);
}"#;

pub struct InputInvoiceLinesDraft<'a> {
    pub name: &'a str,
    pub defaults: &'a str,
    pub preview: &'a str,
    pub product_pick_url: &'a str,
    pub tax_pick_url: &'a str,
    pub classes: &'a str,
}

impl Default for InputInvoiceLinesDraft<'_> {
    fn default() -> Self {
        Self {
            name: "InvoiceLinesJSON",
            defaults: "[]",
            preview: "{}",
            product_pick_url: "/finance-products/pick-product",
            tax_pick_url: "/finance-taxes/multi-select",
            classes: "w-full",
        }
    }
}

fn preview_parts(preview: &str) -> (String, String, String, String) {
    #[derive(serde::Deserialize, Default)]
    struct Preview {
        #[serde(default)]
        products: Vec<serde_json::Value>,
        #[serde(default)]
        tax_pct_by_id: serde_json::Map<String, serde_json::Value>,
        #[serde(default)]
        tax_kind_by_id: serde_json::Map<String, serde_json::Value>,
        #[serde(default)]
        all_taxes: Vec<serde_json::Value>,
    }

    let parsed: Preview = serde_json::from_str(preview).unwrap_or_default();
    let products = serde_json::to_string(&parsed.products).unwrap_or_else(|_| "[]".into());
    let tax_pct = serde_json::to_string(&parsed.tax_pct_by_id).unwrap_or_else(|_| "{}".into());
    let tax_kind = serde_json::to_string(&parsed.tax_kind_by_id).unwrap_or_else(|_| "{}".into());
    let all_taxes = serde_json::to_string(&parsed.all_taxes).unwrap_or_else(|_| "[]".into());
    (products, tax_pct, tax_kind, all_taxes)
}

/// Render the draft invoice lines editor.
pub fn input_invoice_lines_draft(opts: InputInvoiceLinesDraft<'_>) -> Markup {
    let defaults = if opts.defaults.trim().is_empty() {
        r#"[{"product_id":0,"quantity":"1","rate":"","product_label":"","fk_slot":"line-slot-0","tax_ids":[]}]"#
    } else {
        opts.defaults.trim()
    };
    let (products_json, tax_pct_json, tax_kind_json, all_taxes_json) = preview_parts(opts.preview);
    let product_pick_base = serde_json::to_string(opts.product_pick_url).unwrap_or_else(|_| "\"\"".into());
    let tax_pick_base = serde_json::to_string(opts.tax_pick_url).unwrap_or_else(|_| "\"\"".into());

    let alpine_data = format!(
        "{{ lines: {defaults}, products: {products_json}, tax_pct_by_id: {tax_pct_json}, tax_kind_by_id: {tax_kind_json}, all_taxes: {all_taxes_json}, product_pick_base: {product_pick_base}, tax_pick_base: {tax_pick_base}, {methods} }}",
        methods = INVOICE_LINES_DRAFT_ALPINE_METHODS.trim_end_matches(',')
    );

    let name_escaped = escape_attr(opts.name);
    let init_js = format!(
        r#"
if (typeof Alpine !== 'undefined' && Alpine.store && !Alpine.store('m2mSelections')) {{
	Alpine.store('m2mSelections', {{}});
}}
(function () {{
	const d = Alpine.$data($el);
	if (!d || !Array.isArray(d.lines) || typeof d.allocFkSlot !== 'function') return;
	for (const line of d.lines) {{
		if (line.product_label == null) line.product_label = '';
		if (!line.fk_slot) line.fk_slot = d.allocFkSlot();
		if (!Array.isArray(line.line_taxes)) line.line_taxes = [];
		const ids = line.tax_ids;
		if (Array.isArray(ids) && ids.length > 0 && line.line_taxes.length === 0 && Array.isArray(d.all_taxes)) {{
			for (const tid of ids) {{
				const t = d.all_taxes.find(x => x.id === tid);
				if (t) line.line_taxes.push({{ Key: String(t.id), Value: t.name }});
			}}
		}}
		delete line.tax_ids;
	}}
}})();
$nextTick(() => {{ if (window.htmx) window.htmx.process($el); }});
$el.closest('form').addEventListener('submit', (ev) => {{
	const d = Alpine.$data($el);
	if (!d || !Array.isArray(d.lines)) return;
	const h = $el.querySelector('input[type="hidden"][name={name_q}]');
	if (!h) return;
	const strip = (l) => ({{
		product_id: l.product_id,
		quantity: l.quantity,
		rate: l.rate,
		product_label: l.product_label,
		fk_slot: l.fk_slot,
		tax_ids: (l.line_taxes || []).map(t => parseInt(String(t.Key), 10)).filter(id => !isNaN(id) && id > 0),
	}});
	h.value = JSON.stringify(d.lines.map(strip));
}}, true);"#,
        name_q = serde_json::to_string(opts.name).unwrap_or_else(|_| "\"InvoiceLinesJSON\"".into())
    );

    let fk_select_handler = r#"if (!$event.detail) return;
	const n = $event.detail.name;
	const v = $event.detail.value;
	const disp = $event.detail.display || '';
	for (const line of lines) {
		if (!line.fk_slot || line.fk_slot !== n) continue;
		const pid = parseInt(String(v), 10) || 0;
		line.product_id = pid;
		line.product_label = disp;
		if (!pid) { line.rate = ''; line.line_taxes = []; continue; }
		// Prefer sales_price from the picker event (works for newly created products
		// and when the form preview product list is stale/truncated).
		let sp = $event.detail.sales_price != null && String($event.detail.sales_price).trim() !== ''
			? String($event.detail.sales_price).trim() : '';
		const prod = (products || []).find(p => Number(p.id) === pid);
		if (!sp && prod && prod.sales_price != null && String(prod.sales_price).trim() !== '') {
			sp = String(prod.sales_price).trim();
		}
		line.rate = sp;
		if (prod && Array.isArray(prod.tax_ids) && Array.isArray(all_taxes)) {
			line.line_taxes = [];
			for (const tid of prod.tax_ids) {
				const t = all_taxes.find(x => Number(x.id) === Number(tid));
				if (t) line.line_taxes.push({ Key: String(t.id), Value: t.name });
			}
		}
		break;
	}"#;

    let fk_multi_handler = r#"if (!$event.detail) return;
	const n = String($event.detail.name || '');
	const v = $event.detail.value;
	const disp = $event.detail.display || '';
	for (const line of lines) {
		const expected = 'InvoiceLineTaxes_' + String(line.fk_slot || '');
		if (expected !== n) continue;
		const value = String(v);
		const items = line.line_taxes || (line.line_taxes = []);
		const idx = items.findIndex(x => x.Key === value);
		if (idx >= 0) items.splice(idx, 1);
		else items.push({ Key: value, Value: String(disp || value) });
		break;
	}"#;

    let x_effect = "lines.length; $nextTick(() => { if (window.htmx) window.htmx.process($el); })";

    html! {
        div class=(format!("my-1 {}", opts.classes)) {
            div class="w-full min-w-0" {
                    (PreEscaped(format!(
                        r#"<div data-invoice-lines-root="" x-data="{alpine}" x-init="{init}" x-effect="{effect}" @fk-select.window="{fk_sel}" @fk-multi-select.window="{fk_m2m}">"#,
                        alpine = escape_attr(&alpine_data),
                        init = escape_attr(&init_js),
                        effect = escape_attr(x_effect),
                        fk_sel = escape_attr(fk_select_handler),
                        fk_m2m = escape_attr(fk_multi_handler),
                    )))
                        div class="overflow-x-auto min-w-0 rounded-box border border-base-300 bg-base-100" {
                            table class="table table-sm min-w-max w-full" {
                                thead {
                                    tr {
                                        th class="whitespace-nowrap min-w-[12rem]" { "Product" }
                                        th class="whitespace-nowrap min-w-[6rem]" { "Quantity" }
                                        th class="whitespace-nowrap min-w-[10rem]" { "Rate" }
                                        th class="whitespace-nowrap min-w-[10rem]" { "Line taxes" }
                                        th class="whitespace-nowrap min-w-[7rem] text-end" { "Untaxed amount" }
                                        th class="whitespace-nowrap min-w-[7rem] text-end" { "Levied tax" }
                                        th class="whitespace-nowrap min-w-[7rem] text-end" { "Withholding" }
                                        th class="whitespace-nowrap min-w-[7rem] text-end" { "Line total" }
                                        th class="whitespace-nowrap min-w-[6rem]" { "Actions" }
                                    }
                                }
                                tbody {
                                    template x-for="(line, i) in lines" x-bind:key="line.fk_slot" {
                                        tr {
                                            td class="align-middle min-w-[12rem] max-w-md" {
                                                div class="my-1 relative w-full" {
                                                    div class="flex w-full items-stretch gap-1" {
                                                        (PreEscaped(format!(
                                                            r#"<div class="input input-bordered flex-1 flex items-center cursor-pointer min-w-0" :class="line.product_label ? '' : 'opacity-50'" x-bind:hx-get="productPickHref(line.fk_slot)" hx-target="{}" hx-swap="{}" hx-push-url="false">"#,
                                                            HTMX_TARGET_BODY_MODAL,
                                                            HTMX_SWAP_BODY_MODAL
                                                        )))
                                                        span class="text-sm truncate" x-text="line.product_label || 'Select…'" {}
                                                        (PreEscaped("</div>"))
                                                        (PreEscaped(r#"<button type="button" class="btn btn-ghost btn-square shrink-0" @click.stop="line.product_id = 0; line.product_label = ''; line.rate = ''; line.line_taxes = []" x-show="line.product_id" aria-label="Clear product selection">"#))
                                                        (icon("x-mark", ""))
                                                        (PreEscaped("</button>"))
                                                    }
                                                }
                                            }
                                            td class="align-middle min-w-[6rem]" {
                                                input type="text" class="input input-bordered w-full min-w-[5rem]"
                                                    x-model="line.quantity" inputmode="decimal" {}
                                            }
                                            td class="align-middle min-w-[10rem]" {
                                                input type="text" class="input input-bordered w-full min-w-[9rem]"
                                                    x-model="line.rate" inputmode="decimal" {}
                                            }
                                            td class="align-middle min-w-[10rem] max-w-xs" {
                                                div class="my-1" {
                                                    (PreEscaped(format!(
                                                        r#"<div class="input input-bordered min-h-10 w-full flex flex-wrap items-center gap-1 cursor-pointer py-1 px-2" :class="(line.line_taxes && line.line_taxes.length) ? '' : 'opacity-50'" x-bind:hx-get="lineTaxPickHref(line.fk_slot)" hx-target="{}" hx-swap="{}" hx-push-url="false">"#,
                                                        HTMX_TARGET_BODY_MODAL,
                                                        HTMX_SWAP_BODY_MODAL
                                                    )))
                                                    span class="text-sm" x-show="!line.line_taxes || line.line_taxes.length === 0" { "Select taxes…" }
                                                    template x-for="ltItem in (line.line_taxes || [])" x-bind:key="ltItem.Key" {
                                                        (PreEscaped(r#"<div class="flex items-center gap-1 rounded-lg bg-base-200 pl-2 pr-1 py-0.5 max-w-full" @click="$event.stopPropagation()">"#))
                                                        span class="text-xs truncate max-w-[8rem]" x-text="ltItem.Value" {}
                                                        (PreEscaped(r#"<button type="button" class="btn btn-ghost btn-square btn-xs shrink-0" @click.stop="line.line_taxes = (line.line_taxes || []).filter(it => it.Key !== ltItem.Key)" aria-label="Remove tax">"#))
                                                        (icon("x-mark", ""))
                                                        (PreEscaped("</button></div>"))
                                                    }
                                                    (PreEscaped("</div>"))
                                                }
                                            }
                                            td class="align-middle text-end tabular-nums whitespace-nowrap" {
                                                span class="text-sm" x-text="lineUntaxedDisplay(line)" {}
                                            }
                                            td class="align-middle text-end tabular-nums whitespace-nowrap" {
                                                span class="text-sm" x-text="lineLeviedTaxDisplay(line)" {}
                                            }
                                            td class="align-middle text-end tabular-nums whitespace-nowrap" {
                                                span class="text-sm" x-text="lineWithholdingDisplay(line)" {}
                                            }
                                            td class="align-middle text-end tabular-nums whitespace-nowrap" {
                                                span class="text-sm" x-text="lineTotal(line)" {}
                                            }
                                            td class="align-middle w-24" {
                                                (PreEscaped(r#"<button type="button" class="btn btn-ghost btn-sm" @click="lines.splice(i, 1); if (lines.length === 0) lines.push({ product_id: 0, quantity: '1', rate: '', product_label: '', fk_slot: allocFkSlot(), line_taxes: [] }); $nextTick(() => { const r = $el.closest('[data-invoice-lines-root]'); if (r && window.htmx) window.htmx.process(r) })">Remove</button>"#))
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        (PreEscaped(r#"<button type="button" class="btn btn-outline btn-sm mt-2 w-full sm:w-auto" @click="lines.push({ product_id: 0, quantity: '1', rate: '', product_label: '', fk_slot: allocFkSlot(), line_taxes: [] }); $nextTick(() => { const r = $el.closest('[data-invoice-lines-root]'); if (r && window.htmx) window.htmx.process(r) })">Add line</button>"#))
                        div class="mt-3 w-full rounded-box border border-base-300 bg-base-100 overflow-hidden divide-y divide-base-300" {
                            div class="grid grid-cols-[1fr_auto] gap-x-4 items-center px-4 py-3" {
                                div class="text-sm font-bold min-w-0 truncate" { "Lines subtotal" }
                                div class="text-sm tabular-nums text-end font-semibold shrink-0 min-w-[7rem]" x-text="linesSubtotalDisplay()" {}
                            }
                            template x-show="linesWithholdingSubtotalNumber() > 0" {
                                div class="grid grid-cols-[1fr_auto] gap-x-4 items-center px-4 py-3" {
                                    div class="text-sm font-bold min-w-0 truncate" { "Withholding (lines)" }
                                    div class="text-sm tabular-nums text-end font-semibold shrink-0 min-w-[7rem]"
                                        x-text="'(' + formatDec(linesWithholdingSubtotalNumber()) + ')'" {}
                                }
                            }
                            template x-for="invTaxItem in (($store.m2mSelections && $store.m2mSelections.Taxes) ? $store.m2mSelections.Taxes : [])" x-bind:key="invTaxItem.Key" {
                                div class="grid grid-cols-[1fr_auto] gap-x-4 items-center px-4 py-3" {
                                    div class="text-sm font-bold min-w-0 truncate" x-text="invoiceTaxLabel(invTaxItem)" {}
                                    div class="text-sm tabular-nums text-end font-semibold shrink-0 min-w-[7rem]" x-text="invoiceTaxAmountDisplay(invTaxItem)" {}
                                }
                            }
                            div class="grid grid-cols-[1fr_auto] gap-x-4 items-center px-4 py-3 bg-base-200/60" {
                                div class="text-sm font-bold min-w-0 truncate" { "Total" }
                                div class="text-sm tabular-nums text-end font-bold shrink-0 min-w-[7rem]" x-text="invoiceGrandTotalDisplay()" {}
                            }
                        }
                        (PreEscaped(format!(
                            r#"<input type="hidden" name="{name_escaped}">"#
                        )))
                    (PreEscaped("</div>"))
            }
        }
    }
}

/// Read-only invoice lines table for detail views.
pub fn field_invoice_lines(rows: &[InvoiceLineDisplayRow]) -> Markup {
    html! {
        div class="w-full min-w-0" {
            div class="overflow-x-auto min-w-0 rounded-box border border-base-300 bg-base-100" {
                table class="table table-sm min-w-max w-full" {
                    thead {
                        tr {
                            th class="whitespace-nowrap min-w-[12rem]" { "Product" }
                            th class="whitespace-nowrap min-w-[6rem] text-end" { "Quantity" }
                            th class="whitespace-nowrap min-w-[10rem] text-end" { "Rate" }
                            th class="whitespace-nowrap min-w-[10rem]" { "Line taxes" }
                            th class="whitespace-nowrap min-w-[7rem] text-end" { "Untaxed amount" }
                            th class="whitespace-nowrap min-w-[7rem] text-end" { "Levied tax" }
                            th class="whitespace-nowrap min-w-[7rem] text-end" { "Withholding" }
                            th class="whitespace-nowrap min-w-[7rem] text-end" { "Line total" }
                        }
                    }
                    tbody {
                        @if rows.is_empty() {
                            tr {
                                td colspan="8" class="text-center opacity-50 py-4" { "No lines" }
                            }
                        } @else {
                            @for r in rows {
                                tr {
                                    td class="whitespace-nowrap max-w-md min-w-[12rem]" { (r.product) }
                                    td class="whitespace-nowrap text-end tabular-nums min-w-[6rem]" { (r.quantity) }
                                    td class="whitespace-nowrap text-end tabular-nums min-w-[10rem]" { (r.rate) }
                                    td class="min-w-[10rem] max-w-md text-sm" { (r.line_taxes) }
                                    td class="whitespace-nowrap text-end tabular-nums" { (r.untaxed_amount) }
                                    td class="whitespace-nowrap text-end tabular-nums" { (r.levied_tax_amount) }
                                    td class="whitespace-nowrap text-end tabular-nums" { (r.withholding_amount) }
                                    td class="whitespace-nowrap text-end tabular-nums font-medium" { (r.line_total) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
