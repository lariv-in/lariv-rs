//! Draft invoice line editor (Alpine + hidden JSON), ported from Go `InputInvoiceLinesDraft`.

use maud::{Markup, PreEscaped, html};

use lariv_core::components::{
    InputForeignKey,
    attrs::escape_attr,
    htmx::{HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL},
    input_foreign_key,
    text::icon,
};
use lariv_formula::variable_value_input::{VariableValueInput, variable_value_input};

use crate::logic::invoice_line_editor::InvoiceLineDisplayRow;

/// Searchable product picker. `target_input` is set per line in `bindProductPicker`.
fn embed_product_picker(url: &str) -> Markup {
    input_foreign_key(InputForeignKey {
        label: "",
        name: "",
        value: "",
        display: "",
        placeholder: "Select product…",
        url,
        required: false,
        classes: "!my-0",
        ..Default::default()
    })
}

const INVOICE_LINES_DRAFT_ALPINE_METHODS: &str = r#"allocFkSlot() {
	if (typeof crypto !== 'undefined' && crypto.randomUUID) return 'InvoiceLineProduct_' + crypto.randomUUID();
	return 'InvoiceLineProduct_' + Math.random().toString(36).slice(2) + '_' + Date.now().toString(36);
},
productPickerData(el) {
	const root = el && el.querySelector('[x-data]');
	return root && window.Alpine ? Alpine.$data(root) : null;
},
bindProductPicker(el, line) {
	const root = el.querySelector('[x-data]');
	const d = this.productPickerData(el);
	if (!root || !d || typeof d.applySelect !== 'function') {
		const n = Number(el.dataset.pickerTries || 0);
		if (n > 40) return;
		el.dataset.pickerTries = String(n + 1);
		this.$nextTick(() => this.bindProductPicker(el, line));
		return;
	}
	const teleport = root.querySelector('template[x-teleport]');
	const panel = teleport && teleport._x_teleport;
	if (teleport && !panel) {
		const n = Number(el.dataset.pickerTries || 0);
		if (n > 40) return;
		el.dataset.pickerTries = String(n + 1);
		this.$nextTick(() => this.bindProductPicker(el, line));
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
	if (!d._invClearWrapped) {
		d._invClearWrapped = true;
		const origClear = d.clear.bind(d);
		const self = this;
		d.clear = function() {
			origClear.call(this);
			const name = this.fieldName;
			const row = (self.lines || []).find(l => l.fk_slot === name);
			if (row) self.applyProduct(row, { value: '', display: '', name: name });
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
	const nextValue = line.product_id && Number(line.product_id) > 0 ? String(line.product_id) : '';
	const nextDisplay = nextValue ? (line.product_label || '') : '';
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
blankLine() {
	return { product_id: 0, quantity: '1', rate: '', product_label: '', fk_slot: this.allocFkSlot(), line_taxes: [], has_formula: false, variable_rows: [], pre_tax: '', price_error: '', remarks: '' };
},
lengthData(el) {
	const root = el && el.querySelector('[x-data]');
	return root && window.Alpine ? Alpine.$data(root) : null;
},
bindLengthInput(el, line, key) {
	this.$nextTick(() => {
		const d = this.lengthData(el);
		if (!d) return;
		const row = (line.variable_rows || []).find(r => r.name === key);
		const typed = row && row.value != null ? String(row.value) : '';
		d.unit = (row && row.unit) || 'mm';
		d.display = typed;
		if (typeof d.syncMm === 'function') d.syncMm(false);
	});
},
pullLengthInput(el, line, key) {
	const d = this.lengthData(el);
	if (!d) return;
	const row = (line.variable_rows || []).find(r => r.name === key);
	if (!row) return;
	row.value = (d.display == null || d.display === undefined) ? '' : String(d.display);
	row.unit = d.unit || 'mm';
	if (line._priceTimer) clearTimeout(line._priceTimer);
	const self = this;
	line._priceTimer = setTimeout(() => self.refreshPreTax(line), 300);
},
productSchema(detail, prod) {
	if (prod && Array.isArray(prod.variables)) {
		return { has: !!prod.has_formula, rows: prod.variables };
	}
	let rows = [];
	const raw = detail && detail.variables;
	if (typeof raw === 'string' && raw.trim()) {
		try { rows = JSON.parse(raw); } catch (e) { rows = []; }
	} else if (Array.isArray(raw)) {
		rows = raw;
	}
	const flag = detail && detail.has_formula;
	const has = flag === true || flag === '1' || flag === 1;
	return { has: !!has, rows };
},
applyProduct(line, detail) {
	const pid = parseInt(String(detail.value), 10) || 0;
	line.product_id = pid;
	line.product_label = detail.display || '';
	if (!pid) {
		line.rate = '';
		line.has_formula = false;
		line.variable_rows = [];
		line.pre_tax = '';
		line.price_error = '';
		line.line_taxes = [];
		line.remarks = '';
		return;
	}
	const prod = (this.products || []).find(p => Number(p.id) === pid);
	let sp = detail.sales_price != null && String(detail.sales_price).trim() !== ''
		? String(detail.sales_price).trim() : '';
	if (!sp && prod && prod.sales_price != null && String(prod.sales_price).trim() !== '') {
		sp = String(prod.sales_price).trim();
	}
	const schema = this.productSchema(detail, prod);
	const has = prod ? !!prod.has_formula : schema.has;
	line.has_formula = has;
	if (has) {
		const prev = {};
		for (const row of (line.variable_rows || [])) prev[row.name] = { value: row.value, unit: row.unit };
		const rows = (prod && Array.isArray(prod.variables)) ? prod.variables : schema.rows;
		line.variable_rows = (rows || []).map(v => ({
			name: v.name,
			type: v.type,
			placeholder: v.placeholder || '',
			value: prev[v.name] && prev[v.name].value != null ? prev[v.name].value : '',
			unit: (prev[v.name] && prev[v.name].unit) || (v.type === 'weight' ? 'kg' : (v.type === 'length' ? 'mm' : '')),
		}));
		this.refreshPreTax(line);
	} else {
		line.variable_rows = [];
		line.pre_tax = '';
		line.price_error = '';
		line.rate = sp;
	}
	if (prod && Array.isArray(prod.tax_ids) && Array.isArray(this.all_taxes)) {
		line.line_taxes = [];
		for (const tid of prod.tax_ids) {
			const t = this.all_taxes.find(x => Number(x.id) === Number(tid));
			if (t) line.line_taxes.push({ Key: String(t.id), Value: t.name });
		}
	}
	line.remarks = this.productRemarks(detail, prod);
},
productRemarks(detail, prod) {
	if (detail && Object.prototype.hasOwnProperty.call(detail, 'remarks')) {
		return detail.remarks == null ? '' : String(detail.remarks);
	}
	if (prod && prod.remarks != null) return String(prod.remarks);
	return '';
},
async refreshPreTax(line) {
	if (!line || !line.has_formula || !line.product_id) {
		if (line) { line.pre_tax = ''; line.price_error = ''; }
		return;
	}
	const values = this.variablePayload(line);
	const base = this.price_url || '';
	if (!base) return;
	const sep = base.indexOf('?') >= 0 ? '&' : '?';
	const url = base + sep + 'product_id=' + encodeURIComponent(line.product_id) + '&values=' + encodeURIComponent(JSON.stringify(values));
	try {
		const res = await fetch(url, { headers: { 'Accept': 'application/json' }, credentials: 'same-origin' });
		const body = await res.json();
		if (body && !body.error && body.pre_tax != null && String(body.pre_tax) !== '') {
			line.rate = String(body.pre_tax);
			line.pre_tax = line.rate;
		} else {
			line.pre_tax = '';
		}
		line.price_error = body && body.error ? String(body.error) : '';
	} catch (e) {
		line.price_error = 'Could not calculate price';
	}
},
lineUntaxedNumber(line) {
	const q = parseFloat(String(line.quantity ?? '').replace(/,/g, '.')) || 0;
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
variablePayload(line) {
	const values = {};
	for (const row of (line && line.variable_rows) || []) {
		const value = row.value == null ? '' : String(row.value);
		if (row.type === 'length' || row.type === 'weight') {
			const fallback = row.type === 'weight' ? 'kg' : 'mm';
			values[row.name] = { value: value, unit: row.unit || fallback };
		} else {
			values[row.name] = value;
		}
	}
	return values;
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
            product_pick_url: "",
            tax_pick_url: "",
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
    let product_pick_fallback =
        lariv_plugin_finance_products::routes::ProductFkSelectRouteTag.url();
    let tax_pick_fallback = lariv_plugin_finance_taxes::routes::TaxMultiSelectRouteTag.url();
    let product_pick_url = if opts.product_pick_url.is_empty() {
        product_pick_fallback.as_str()
    } else {
        opts.product_pick_url
    };
    let tax_pick_url = if opts.tax_pick_url.is_empty() {
        tax_pick_fallback.as_str()
    } else {
        opts.tax_pick_url
    };
    let defaults = if opts.defaults.trim().is_empty() {
        r#"[{"product_id":0,"quantity":"1","rate":"","product_label":"","fk_slot":"line-slot-0","tax_ids":[],"remarks":""}]"#
    } else {
        opts.defaults.trim()
    };
    let (products_json, tax_pct_json, tax_kind_json, all_taxes_json) = preview_parts(opts.preview);
    let product_pick_base =
        serde_json::to_string(product_pick_url).unwrap_or_else(|_| "\"\"".into());
    let tax_pick_base = serde_json::to_string(tax_pick_url).unwrap_or_else(|_| "\"\"".into());

    let price_url = serde_json::to_string(&crate::routes::InvoiceLinePriceRouteTag.url())
        .unwrap_or_else(|_| "\"\"".into());
    let alpine_data = format!(
        "{{ lines: {defaults}, products: {products_json}, tax_pct_by_id: {tax_pct_json}, tax_kind_by_id: {tax_kind_json}, all_taxes: {all_taxes_json}, product_pick_base: {product_pick_base}, tax_pick_base: {tax_pick_base}, price_url: {price_url}, {methods} }}",
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
		if (line.quantity == null || line.quantity === '') line.quantity = '1';
		if (line.product_label == null) line.product_label = '';
		if (!line.fk_slot) line.fk_slot = d.allocFkSlot();
		if (!Array.isArray(line.line_taxes)) line.line_taxes = [];
		if (!Array.isArray(line.variable_rows)) line.variable_rows = [];
		if (line.has_formula == null) line.has_formula = false;
		if (line.pre_tax == null) line.pre_tax = '';
		if (line.price_error == null) line.price_error = '';
		if (line.remarks == null) line.remarks = '';
		if (line.has_formula) d.refreshPreTax(line);
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
		variables: d.variablePayload(l),
		tax_ids: (l.line_taxes || []).map(t => parseInt(String(t.Key), 10)).filter(id => !isNaN(id) && id > 0),
		remarks: l.remarks == null ? '' : String(l.remarks),
	}});
	h.value = JSON.stringify(d.lines.map(strip));
}}, true);"#,
        name_q = serde_json::to_string(opts.name).unwrap_or_else(|_| "\"InvoiceLinesJSON\"".into())
    );

    let fk_select_handler = r#"if (!$event.detail) return;
	const n = $event.detail.name;
	for (const line of lines) {
		if (!line.fk_slot || line.fk_slot !== n) continue;
		applyProduct(line, $event.detail);
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
                        r#"<div data-invoice-lines-root="" x-data="{alpine}" x-init="{init}" x-effect="{effect}" @fk-select.window="{fk_sel}" @fk-multi-select.window="{fk_m2m}" @lariv-fk-created.window="{fk_created}">"#,
                        alpine = escape_attr(&alpine_data),
                        init = escape_attr(&init_js),
                        effect = escape_attr(x_effect),
                        fk_sel = escape_attr(fk_select_handler),
                        fk_m2m = escape_attr(fk_multi_handler),
                        fk_created = escape_attr(&format!(
                            "if ($event) {{ {{ {fk_select_handler} }}; {{ {fk_multi_handler} }}; document.querySelectorAll('dialog.fk-modal-container').forEach((d) => {{ if (d.querySelector('.fk-picker-results')) return; if (d.querySelector('.data-table-container')) d.remove() }}) }}"
                        )),
                    )))
                        div class="overflow-x-auto min-w-0 rounded-box border border-base-300 bg-base-100" {
                            table class="table table-sm min-w-max w-full" {
                                thead {
                                    tr {
                                        th class="whitespace-nowrap min-w-[16rem]" { "Product" }
                                        th class="whitespace-nowrap min-w-[12rem]" { "Remarks" }
                                        th class="whitespace-nowrap min-w-[14rem]" { "Inputs" }
                                        th class="whitespace-nowrap min-w-[7rem]" { "Quantity" }
                                        th class="whitespace-nowrap min-w-[8rem]" { "Unit price" }
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
                                            td class="align-middle min-w-[16rem] max-w-md" {
                                                div class="min-w-0" data-product-fkey=""
                                                    x-effect="bindProductPicker($el, line)" {
                                                    (embed_product_picker(product_pick_url))
                                                }
                                            }
                                            td class="align-middle min-w-[12rem]" {
                                                textarea class="textarea textarea-bordered w-full min-w-[10rem] text-sm" rows="2"
                                                    x-model="line.remarks" placeholder="Remarks" {}
                                            }
                                            td class="align-middle min-w-[14rem]" {
                                                div class="flex flex-col gap-1.5 py-1" x-show="line.has_formula" {
                                                    template x-for="vrow in (line.variable_rows || [])" x-bind:key="vrow.name" {
                                                        (variable_value_input(VariableValueInput {
                                                            row: "vrow",
                                                            value: "vrow.value",
                                                            on_input: "refreshPreTax(line)",
                                                            length_host: "line",
                                                            debounce_ms: Some(300),
                                                            compact: false,
                                                        }))
                                                    }
                                                    span class="text-sm opacity-60" x-show="!(line.variable_rows || []).length" { "Fixed by formula" }
                                                    span class="text-xs text-error" x-show="line.price_error" x-text="line.price_error" {}
                                                }
                                                span class="text-sm opacity-60" x-show="!line.has_formula" { "—" }
                                            }
                                            td class="align-middle min-w-[7rem]" {
                                                input type="text" class="input input-bordered input-sm w-full min-w-[5rem]"
                                                    x-model="line.quantity" inputmode="decimal" placeholder="e.g. 1" {}
                                            }
                                            td class="align-middle min-w-[8rem]" {
                                                input type="text" class="input input-bordered input-sm w-full min-w-[5rem]"
                                                    x-model="line.rate" inputmode="decimal" placeholder="Price of one"
                                                    x-show="!line.has_formula" {}
                                                span class="text-sm tabular-nums" x-show="line.has_formula" x-text="line.rate || '—'" {}
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
                                                (PreEscaped(r#"<button type="button" class="btn btn-ghost btn-sm" @click="lines.splice(i, 1); if (lines.length === 0) lines.push(blankLine()); $nextTick(() => { const r = $el.closest('[data-invoice-lines-root]'); if (r && window.htmx) window.htmx.process(r) })">Remove</button>"#))
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        (PreEscaped(r#"<button type="button" class="btn btn-outline btn-sm mt-2 w-full sm:w-auto" @click="lines.push(blankLine()); $nextTick(() => { const r = $el.closest('[data-invoice-lines-root]'); if (r && window.htmx) window.htmx.process(r) })">Add line</button>"#))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoice_lines_keeps_alpine_in_attributes() {
        let html = input_invoice_lines_draft(InputInvoiceLinesDraft {
            name: "InvoiceLinesJson",
            defaults: "",
            ..Default::default()
        })
        .into_string();
        assert!(html.contains("x-init="));
        assert!(
            !html.contains(r#"querySelector('input[type="hidden"]"#),
            "unescaped x-init leaked as text: {html}"
        );
        assert!(
            html.contains("type=&quot;hidden&quot;"),
            "x-init quotes must be escaped"
        );
        assert!(
            html.contains(">Remarks<"),
            "line editor must include a remarks column"
        );
        assert!(html.contains("line.remarks"));
        assert!(
            html.contains("productRemarks"),
            "selecting a product must prefill line remarks"
        );
        assert!(
            html.contains("Open selection table"),
            "product field must use the foreign-key picker"
        );
        assert!(
            html.contains("bindProductPicker"),
            "each line must bind the product picker to its slot"
        );
        assert!(
            !html.contains("productPickHref"),
            "product field must not use the click-to-open picker"
        );
        assert!(html.contains("vrow.name + ':'"), "{html}");
        assert!(
            html.contains("Length unit"),
            "length inputs need a unit selector"
        );
        assert!(html.contains(">kg<"), "weight inputs need a kg unit");
        assert!(
            html.contains("variablePayload"),
            "length and weight must be stored with their unit"
        );
        assert!(html.contains("row.unit || fallback"), "{html}");
        assert!(html.contains(">Quantity<"), "line editor must include a quantity column");
        assert!(
            html.contains("x-model=\"line.quantity\""),
            "quantity column must be an editable input"
        );
        assert!(html.contains(">Unit price<"));
        assert!(
            html.contains("applyProduct(line, $event.detail)"),
            "fk-select must call applyProduct through Alpine scope"
        );
        assert!(
            !html.contains("this.applyProduct"),
            "this is not the component inside Alpine if-handlers"
        );
        assert_eq!(
            html.matches("{ if (!$event.detail) return;").count(),
            2,
            "lariv-fk-created must block-scope each handler so const n is not redeclared"
        );
        assert!(
            html.contains("@lariv-fk-created.window=\"if ($event)"),
            "lariv-fk-created must start with if so Alpine wraps the handler; a leading brace is parsed as an object literal"
        );
        assert!(
            !lariv_core::components::attrs::alpine_js_leaked_as_text(&html),
            "Alpine JS rendered as text: {html}"
        );
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
                            th class="whitespace-nowrap min-w-[12rem]" { "Remarks" }
                            th class="whitespace-nowrap min-w-[14rem]" { "Inputs" }
                            th class="whitespace-nowrap min-w-[5rem] text-end" { "Quantity" }
                            th class="whitespace-nowrap min-w-[7rem] text-end" { "Unit price" }
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
                                td colspan="10" class="text-center opacity-50 py-4" { "No lines" }
                            }
                        } @else {
                            @for r in rows {
                                tr {
                                    td class="whitespace-nowrap max-w-md min-w-[12rem]" { (r.product) }
                                    td class="min-w-[12rem] max-w-md text-sm whitespace-pre-wrap" { (r.remarks) }
                                    td class="min-w-[14rem] text-sm" { (r.inputs) }
                                    td class="whitespace-nowrap text-end tabular-nums" { (r.quantity) }
                                    td class="whitespace-nowrap text-end tabular-nums" { (r.rate) }
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
