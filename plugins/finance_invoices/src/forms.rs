use lariv_core::components::attrs::escape_attr;
use lariv_core::html_form::{
    FieldRender, FormCtx, FormWidget, html_form,
    widgets::{Date, Datetime, Text, Textarea},
};
use lariv_plugin_finance_accounts::journal_lines_editor::input_journal_lines;
use maud::{Markup, PreEscaped, html};

use lariv_plugin_crm::routes::CustomerFkSelectRouteTag;
use lariv_plugin_filesystem::routes::VNodeFileSelectRouteTag;
use lariv_plugin_finance_accounts::routes::JournalSelectRouteTag;
use lariv_plugin_finance_taxes::journal_template_editor::input_journal_template_lines;
use lariv_plugin_finance_taxes::routes::TaxMultiSelectRouteTag;

use crate::components::{
    InputInvoiceGoodsLinesDraft, InputInvoiceServiceLinesDraft, InputPaymentTermLinesDraft,
    input_invoice_goods_lines_draft, input_invoice_service_lines_draft,
    input_payment_term_lines_draft,
};
use crate::payment_journal_template::{FORMULA_VARIABLES_HINT, empty_lines_json};
use crate::routes::PostedInvoiceFkSelectRouteTag;

/// Account and formula lines stored as JSON.
pub struct PaymentJournalTemplateLines;

impl FormWidget for PaymentJournalTemplateLines {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let defaults = if field.value.trim().is_empty() {
            empty_lines_json()
        } else {
            field.value.to_string()
        };
        input_journal_template_lines(field.name, field.label, &defaults, ctx.hint_of(field.spec))
    }
}

/// Custom widget for draft invoice goods lines (Alpine editor + hidden JSON).
pub struct InvoiceGoodsLinesDraft;
impl FormWidget for InvoiceGoodsLinesDraft {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_invoice_goods_lines_draft(InputInvoiceGoodsLinesDraft {
            name: field.name,
            defaults: field.value,
            preview: ctx.display_of("invoice_goods_lines_preview"),
            ..Default::default()
        })
    }
}

/// Custom widget for draft invoice service lines (Alpine editor + hidden JSON).
pub struct InvoiceServiceLinesDraft;
impl FormWidget for InvoiceServiceLinesDraft {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_invoice_service_lines_draft(InputInvoiceServiceLinesDraft {
            name: field.name,
            defaults: field.value,
            preview: ctx.display_of("invoice_service_lines_preview"),
            ..Default::default()
        })
    }
}

/// Custom widget for draft payment term lines (Alpine editor + hidden JSON).
pub struct PaymentTermLinesDraft;
impl FormWidget for PaymentTermLinesDraft {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_payment_term_lines_draft(InputPaymentTermLinesDraft {
            name: field.name,
            defaults: field.value,
            ..Default::default()
        })
    }
}

#[html_form]
pub struct DraftInvoiceForm {
    #[form(label = "Number", widget = Text)]
    pub number: String,

    #[form(label = "Reference", widget = Text)]
    pub reference: String,

    #[form(label = "E-Way bill number", widget = Text)]
    pub eway_bill_number: String,

    #[form(label = "Payment reference", widget = Text)]
    pub payment_reference: String,

    #[form(label = "Account", widget = Textarea, rows = 6)]
    pub bank_account: String,

    #[form(label = "Remarks", widget = Textarea, rows = 4)]
    pub remarks: String,

    #[form(label = "Date", required, widget = Date)]
    pub datetime: String,

    #[form(label = "Delivery date", widget = Date)]
    pub delivery_date: String,

    #[form(
        label = "Customer",
        required,
        widget = ForeignKey,
        route = CustomerFkSelectRouteTag,
        swap_key = "fk-invoice-customer",
        display = "customer",
        placeholder = "Select customer…"
    )]
    pub customer_id: i64,

    #[form(label = "Payment schedule", required, widget = PaymentTermLinesDraft)]
    pub payment_term_lines_json: String,

    #[form(
        label = "Taxes",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "invoice-header-taxes",
        placeholder = "Select taxes…"
    )]
    pub taxes: Vec<i64>,

    #[form(
        label = "Goods lines",
        widget = InvoiceGoodsLinesDraft,
        display = "invoice_goods_lines_preview",
        hint = "Quantity multiplies the unit price of one product."
    )]
    pub invoice_goods_lines_json: String,

    #[form(
        label = "Service lines",
        widget = InvoiceServiceLinesDraft,
        display = "invoice_service_lines_preview",
        hint = "Quantity multiplies the unit price of one service."
    )]
    pub invoice_service_lines_json: String,
}

/// Blank bulk-edit form: only non-empty fields are applied to selected drafts.
#[html_form]
pub struct DraftInvoiceBulkEditForm {
    #[form(label = "Number", widget = Text)]
    pub number: String,

    #[form(label = "Reference", widget = Text)]
    pub reference: String,

    #[form(label = "E-Way bill number", widget = Text)]
    pub eway_bill_number: String,

    #[form(label = "Payment reference", widget = Text)]
    pub payment_reference: String,

    #[form(label = "Account", widget = Textarea, rows = 6)]
    pub bank_account: String,

    #[form(label = "Remarks", widget = Textarea, rows = 4)]
    pub remarks: String,

    #[form(label = "Date", widget = Date)]
    pub datetime: String,

    #[form(label = "Delivery date", widget = Date)]
    pub delivery_date: String,

    #[form(
        label = "Customer",
        widget = ForeignKey,
        route = CustomerFkSelectRouteTag,
        swap_key = "fk-invoice-customer",
        display = "customer",
        placeholder = "Leave empty to keep existing…"
    )]
    pub customer_id: i64,

    #[form(label = "Payment schedule", widget = PaymentTermLinesDraft)]
    pub payment_term_lines_json: String,

    #[form(
        label = "Taxes",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "invoice-header-taxes",
        placeholder = "Leave empty to keep existing…"
    )]
    pub taxes: Vec<i64>,

    #[form(
        label = "Goods lines",
        widget = InvoiceGoodsLinesDraft,
        display = "invoice_goods_lines_preview",
        hint = "Quantity multiplies the unit price of one product. Leave both line sections empty to keep existing lines."
    )]
    pub invoice_goods_lines_json: String,

    #[form(
        label = "Service lines",
        widget = InvoiceServiceLinesDraft,
        display = "invoice_service_lines_preview",
        hint = "Quantity multiplies the unit price of one service. Leave both line sections empty to keep existing lines."
    )]
    pub invoice_service_lines_json: String,
}

#[html_form]
pub struct PaymentForm {
    #[form(
        label = "Posted invoice",
        required,
        widget = ForeignKey,
        route = PostedInvoiceFkSelectRouteTag,
        swap_key = "posted-invoice-select",
        display = "posted_invoice",
        placeholder = "Select posted invoice…"
    )]
    pub posted_invoice_id: i64,

    #[form(label = "Settlement amount", required, widget = Text)]
    pub amount: String,

    #[form(label = "Payment date & time", required, widget = Datetime)]
    pub datetime: String,

    #[form(
        label = "Withholding taxes",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "payment-withholding-taxes",
        placeholder = "Optional withholding at collection…"
    )]
    pub taxes: Vec<i64>,

    #[form(label = "Journal lines", required, widget = PaymentJournalLines)]
    pub journal_lines: String,
}

/// Debit and credit lines for one payment. Prefills from the payment template.
pub struct PaymentJournalLines;

impl FormWidget for PaymentJournalLines {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let url = crate::routes::PaymentJournalLinesRouteTag.path();
        html! {
            div id="payment-journal-lines" {
                (input_journal_lines(field.name, field.label, field.value))
            }
            (PreEscaped(format!(
                r##"<button type="button" class="btn btn-outline btn-sm" hx-post="{}" hx-target="#payment-journal-lines" hx-swap="innerHTML" hx-include="closest form">Fill from template</button>"##,
                escape_attr(&url),
            )))
            p class="text-sm text-base-content/70 mt-2" {
                "Saved lines are whatever is in this editor. Fill from template replaces them from the payment preferences for the settlement amount and withholding taxes above."
            }
        }
    }
}

/// Header fields for batch payment (allocations use a custom JSON editor widget).
#[html_form]
pub struct PaymentBatchForm {
    #[form(label = "Payment date & time", required, widget = Datetime)]
    pub datetime: String,

    #[form(label = "Allocations", required, widget = PaymentBatchAllocations, display = "batch_allocations_preview")]
    pub allocations_json: String,

    #[form(label = "Journal lines", required, widget = PaymentBatchJournalLines)]
    pub journal_lines: String,
}

/// Combined debit and credit lines for a payment batch.
pub struct PaymentBatchJournalLines;

impl FormWidget for PaymentBatchJournalLines {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let url = crate::routes::PaymentBatchJournalLinesRouteTag.path();
        html! {
            div id="payment-batch-journal-lines" {
                (input_journal_lines(field.name, field.label, field.value))
            }
            (PreEscaped(format!(
                r##"<button type="button" class="btn btn-outline btn-sm" hx-post="{}" hx-target="#payment-batch-journal-lines" hx-swap="innerHTML" hx-include="closest form">Fill from template</button>"##,
                escape_attr(&url),
            )))
            p class="text-sm text-base-content/70 mt-2" {
                "Saved lines are whatever is in this editor. Fill from template replaces them from the payment preferences for the allocations above."
            }
        }
    }
}

/// Custom widget for batch payment allocations (Alpine editor + hidden JSON).
pub struct PaymentBatchAllocations;
impl FormWidget for PaymentBatchAllocations {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let preview = ctx.display_of("batch_allocations_preview");
        #[derive(serde::Deserialize, Default)]
        struct Preview {
            #[serde(default)]
            tax_pct_by_id: serde_json::Map<String, serde_json::Value>,
            #[serde(default)]
            all_taxes: Vec<serde_json::Value>,
        }
        let parsed: Preview = serde_json::from_str(preview).unwrap_or_default();
        let tax_pct_json =
            serde_json::to_string(&parsed.tax_pct_by_id).unwrap_or_else(|_| "{}".into());
        let all_taxes_json =
            serde_json::to_string(&parsed.all_taxes).unwrap_or_else(|_| "[]".into());
        crate::components::input_payment_batch_allocations(
            crate::components::InputPaymentBatchAllocations {
                name: field.name,
                defaults: field.value,
                tax_pct_json: &tax_pct_json,
                all_taxes_json: &all_taxes_json,
                ..Default::default()
            },
        )
    }
}

/// Invoice number format + PDF template field names (custom UI on preferences page).
#[html_form]
pub struct InvoicePresentationPreferencesForm {
    #[form(label = "Invoice number format", widget = Text)]
    pub invoice_number_format: String,

    #[form(label = "Invoice date format", widget = Text)]
    pub invoice_date_format: String,

    #[form(label = "Invoice datetime format", widget = Text)]
    pub invoice_datetime_format: String,

    #[form(label = "Invoice PDF template (Typst + Minijinja)", widget = Textarea, rows = 16)]
    pub invoice_pdf_template: String,
}

/// Logo and signature files for invoice PDFs (filesystem VNodes).
#[html_form]
pub struct InvoicePdfAssetPreferencesForm {
    #[form(
        label = "Invoice logo",
        widget = ForeignKey,
        route = VNodeFileSelectRouteTag,
        swap_key = "pref-invoice-logo-vnode",
        display = "invoice_logo_vnode",
        placeholder = "Select logo file…"
    )]
    pub invoice_logo_vnode_id: String,

    #[form(
        label = "Invoice signature",
        widget = ForeignKey,
        route = VNodeFileSelectRouteTag,
        swap_key = "pref-invoice-signature-vnode",
        display = "invoice_signature_vnode",
        placeholder = "Select signature file…"
    )]
    pub invoice_signature_vnode_id: String,
}

/// Company text shown on invoice PDFs (name, address, footer, place of supply).
#[html_form]
pub struct InvoiceCompanyPreferencesForm {
    #[form(label = "Company name", widget = Text)]
    pub company_name: String,

    #[form(label = "Company address (Typst)", widget = Textarea, rows = 4)]
    pub company_address: String,

    #[form(label = "Company phone", widget = Text)]
    pub company_phone: String,

    #[form(label = "Company GSTIN", widget = Text)]
    pub company_gstin: String,

    #[form(label = "Default place of supply", widget = Text)]
    pub place_of_supply: String,

    #[form(label = "Default account", widget = Textarea, rows = 6)]
    pub default_bank_account: String,
}

#[html_form]
pub struct PaymentPreferencesForm {
    #[form(
        label = "Journal",
        widget = ForeignKey,
        route = JournalSelectRouteTag,
        swap_key = "pref-payment-journal",
        display = "journal",
        placeholder = "Select journal…"
    )]
    pub journal_id: String,

    #[form(
        label = "Journal entry template",
        widget = PaymentJournalTemplateLines,
        hint = FORMULA_VARIABLES_HINT
    )]
    pub journal_lines: String,
}

#[html_form]
pub struct CancelInvoiceForm {
    #[form(label = "Reason", required, widget = Textarea, rows = 3)]
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use lariv_core::components::attrs::alpine_js_leaked_as_text;
    use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};

    #[test]
    fn draft_invoice_create_form_keeps_alpine_in_attributes() {
        let html = DraftInvoiceForm::render_inputs(
            &FormCtx::form::<DraftInvoiceForm>(CsrfToken::current())
                .value(DraftInvoiceFormField::CustomerId, "1")
                .display(DraftInvoiceFormField::CustomerId, "Acme Co")
                .value(DraftInvoiceFormField::Datetime, "2025-06-01")
                .value(DraftInvoiceFormField::PaymentTermLinesJson, "")
                .value(DraftInvoiceFormField::InvoiceGoodsLinesJson, ""),
        )
        .into_string();
        assert!(html.contains("Customer"));
        assert!(html.contains("E-Way bill number"));
        assert!(html.contains("Acme Co"));
        assert!(
            !html.contains(r#"querySelector('input[type="hidden"]"#),
            "Alpine JS leaked as text on the draft invoice form: {html}"
        );
        assert!(
            !alpine_js_leaked_as_text(&html),
            "Alpine JS rendered as text on the draft invoice form: {html}"
        );
    }

    #[test]
    fn payment_template_label_lists_injected_variables() {
        let html = PaymentPreferencesForm::render_inputs(&FormCtx::form::<PaymentPreferencesForm>(
            CsrfToken::current(),
        ))
        .into_string();
        assert!(html.contains("Journal entry template"));
        assert!(html.contains("amount — settlement on this payment"));
        assert!(html.contains("withholding — withheld tax on this payment"));
        assert!(html.contains("hintOpen"));
    }
}
