use lariv_core::components::attrs::escape_attr;
use lariv_core::html_form::{
    FieldRender, FormCtx, FormWidget, html_form,
    widgets::{Date, Datetime, Text, Textarea},
};
use lariv_plugin_finance_accounts::journal_lines_editor::input_journal_lines;
use maud::{Markup, PreEscaped, html};

use lariv_plugin_crm::routes::CustomerFkSelectRouteTag;
use lariv_plugin_filesystem::routes::VNodeFileSelectRouteTag;
use lariv_plugin_finance_taxes::routes::TaxMultiSelectRouteTag;

use crate::components::{
    InputInvoiceLinesDraft, InputPaymentTermLinesDraft, input_invoice_lines_draft,
    input_payment_term_lines_draft,
};
use crate::routes::InvoicePayJournalLinesRouteTag;

/// Custom widget for draft invoice lines (Alpine editor + hidden JSON).
pub struct InvoiceLinesDraft;
impl FormWidget for InvoiceLinesDraft {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_invoice_lines_draft(InputInvoiceLinesDraft {
            name: field.name,
            defaults: field.value,
            preview: ctx.display_of("invoice_lines_preview"),
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
        label = "Lines",
        widget = InvoiceLinesDraft,
        display = "invoice_lines_preview",
        hint = "Quantity multiplies the unit price of one product."
    )]
    pub invoice_lines_json: String,
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
        label = "Lines",
        widget = InvoiceLinesDraft,
        display = "invoice_lines_preview",
        hint = "Quantity multiplies the unit price of one product. Leave empty to keep existing lines."
    )]
    pub invoice_lines_json: String,
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
pub struct CancelInvoiceForm {
    #[form(label = "Reason", required, widget = Textarea, rows = 3)]
    pub reason: String,
}

/// Hidden posted-invoice id carried by the pay modal.
pub struct HiddenPostedInvoiceId;

impl FormWidget for HiddenPostedInvoiceId {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        html! {
            input type="hidden" name=(field.name) value=(field.value);
        }
    }
}

/// Debit and credit lines when paying one posted invoice.
pub struct InvoicePayJournalLines;

impl FormWidget for InvoicePayJournalLines {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let url = InvoicePayJournalLinesRouteTag.path();
        html! {
            div id="invoice-pay-journal-lines" {
                (input_journal_lines(field.name, field.label, field.value))
            }
            (PreEscaped(format!(
                r##"<button type="button" class="btn btn-outline btn-sm" hx-post="{}" hx-target="#invoice-pay-journal-lines" hx-swap="innerHTML" hx-include="closest form">Fill from template</button>"##,
                escape_attr(&url),
            )))
        }
    }
}

#[html_form]
pub struct InvoicePaymentForm {
    #[form(widget = HiddenPostedInvoiceId)]
    pub posted_invoice_id: i64,

    #[form(label = "Settlement amount", required, widget = Text)]
    pub amount: String,

    #[form(label = "Payment date & time", required, widget = Datetime)]
    pub datetime: String,

    #[form(
        label = "Withholding taxes",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "invoice-pay-withholding-taxes",
        placeholder = "Optional withholding at collection…"
    )]
    pub taxes: Vec<i64>,

    #[form(label = "Journal lines", required, widget = InvoicePayJournalLines)]
    pub journal_lines: String,
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
                .value(DraftInvoiceFormField::InvoiceLinesJson, ""),
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
}
