use crate::html_form::{
    FieldRender, FormCtx, FormWidget, html_form,
    widgets::{Date, Datetime, Text, Textarea},
};
use maud::Markup;

use crate::plugins::customer::routes::CustomerFkSelectRouteTag;
use crate::plugins::filesystem::routes::VNodeFileSelectRouteTag;
use crate::plugins::finance_accounts::routes::{AccountSelectRouteTag, JournalSelectRouteTag};
use crate::plugins::finance_taxes::routes::TaxMultiSelectRouteTag;

use crate::plugins::finance_invoices::components::{
    InputInvoiceLinesDraft, InputPaymentTermLinesDraft, input_invoice_lines_draft,
    input_payment_term_lines_draft,
};
use crate::plugins::finance_invoices::routes::PostedInvoiceFkSelectRouteTag;

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
    #[form(label = "Number (optional)", widget = Text)]
    pub number: String,

    #[form(label = "Reference (optional)", widget = Text)]
    pub reference: String,

    #[form(label = "Payment reference (optional)", widget = Text)]
    pub payment_reference: String,

    #[form(label = "Bank account (optional)", widget = Text)]
    pub bank_account: String,

    #[form(label = "Date", required, widget = Date)]
    pub datetime: String,

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

    #[form(label = "Lines", required, widget = InvoiceLinesDraft, display = "invoice_lines_preview")]
    pub invoice_lines_json: String,
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

    #[form(
        label = "Payment account (optional)",
        widget = ForeignKey,
        route = AccountSelectRouteTag,
        swap_key = "payment-account",
        display = "payment_account",
        placeholder = "Uses preference default…"
    )]
    pub account_id: String,

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
}

/// Header fields for batch payment (allocations use a custom JSON editor widget).
#[html_form]
pub struct PaymentBatchForm {
    #[form(label = "Payment date & time", required, widget = Datetime)]
    pub datetime: String,

    #[form(
        label = "Payment account (optional)",
        widget = ForeignKey,
        route = AccountSelectRouteTag,
        swap_key = "payment-batch-account",
        display = "payment_account",
        placeholder = "Uses preference default…"
    )]
    pub account_id: String,

    #[form(label = "Allocations", required, widget = PaymentBatchAllocations, display = "batch_allocations_preview")]
    pub allocations_json: String,
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
        crate::plugins::finance_invoices::components::input_payment_batch_allocations(
            crate::plugins::finance_invoices::components::InputPaymentBatchAllocations {
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
}

#[html_form]
pub struct InvoicePreferencesForm {
    #[form(
        label = "Accounts receivable (invoices)",
        widget = ForeignKey,
        route = AccountSelectRouteTag,
        swap_key = "pref-invoice-ar",
        display = "account_receivable",
        placeholder = "Select debit account…"
    )]
    pub account_receivable_id: String,

    #[form(
        label = "Revenue account (invoices)",
        widget = ForeignKey,
        route = AccountSelectRouteTag,
        swap_key = "pref-invoice-revenue",
        display = "account_revenue",
        placeholder = "Select credit account…"
    )]
    pub account_revenue_id: String,

    #[form(
        label = "Tax payable (invoices)",
        widget = ForeignKey,
        route = AccountSelectRouteTag,
        swap_key = "pref-invoice-tax",
        display = "account_tax_payable",
        placeholder = "Select credit account…"
    )]
    pub account_tax_payable_id: String,

    #[form(
        label = "Journal (invoices)",
        widget = ForeignKey,
        route = JournalSelectRouteTag,
        swap_key = "pref-invoice-journal",
        display = "journal",
        placeholder = "Select journal…"
    )]
    pub journal_id: String,
}

#[html_form]
pub struct PaymentPreferencesForm {
    #[form(
        label = "Payment account (receipts)",
        widget = ForeignKey,
        route = AccountSelectRouteTag,
        swap_key = "pref-payment-account",
        display = "payment_account",
        placeholder = "Bank or cash account…"
    )]
    pub payment_account_id: String,
}

#[html_form]
pub struct CancelInvoiceForm {
    #[form(label = "Reason", required, widget = Textarea, rows = 3)]
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::attrs::alpine_js_leaked_as_text;
    use crate::html_form::{FormCtx, HtmlForm};

    #[test]
    fn draft_invoice_create_form_keeps_alpine_in_attributes() {
        let html = DraftInvoiceForm::render_inputs(
            &FormCtx::form::<DraftInvoiceForm>()
                .value(DraftInvoiceFormField::CustomerId, "1")
                .display(DraftInvoiceFormField::CustomerId, "Acme Co")
                .value(DraftInvoiceFormField::Datetime, "2025-06-01")
                .value(DraftInvoiceFormField::PaymentTermLinesJson, "")
                .value(DraftInvoiceFormField::InvoiceLinesJson, ""),
        )
        .into_string();
        assert!(html.contains("Customer") || html.contains("customer"));
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
