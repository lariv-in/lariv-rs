use lariv_core::html_form::{
    FieldRender, FormCtx, FormWidget, html_form,
    widgets::{Date, Datetime, Decimal, Number, Text, Textarea},
};
use maud::Markup;

use lariv_plugin_customer::routes::CustomerFkSelectRouteTag;
use lariv_plugin_filesystem::routes::VNodeFileSelectRouteTag;
use lariv_plugin_finance_accounts::routes::{AccountSelectRouteTag, JournalSelectRouteTag};
use lariv_plugin_finance_products::routes::ProductFkSelectRouteTag;
use lariv_plugin_finance_taxes::routes::TaxMultiSelectRouteTag;

use crate::components::{
    InputInvoiceLinesDraft, InputPaymentTermLinesDraft, input_invoice_lines_draft,
    input_payment_term_lines_draft,
};
use crate::routes::PostedInvoiceFkSelectRouteTag;

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
        required,
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
        hint = "Quantity multiplies the unit price of one product. Leave lines empty to keep existing lines."
    )]
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
        label = "Payment account",
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
        label = "Payment account",
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

/// Column filters for the invoice hub table. Field order matches the table.
#[html_form]
pub struct InvoiceHubFilterForm {
    #[form(label = "ID", widget = Text, placeholder = "Contains")]
    pub id: String,

    #[form(label = "Number", widget = Text, placeholder = "Contains")]
    pub number: String,

    #[form(label = "Customer", widget = Text, placeholder = "Contains", when = "posted")]
    pub customer: String,

    #[form(
        label = "Open balance min",
        widget = Decimal,
        row = "open_balance",
        when = "posted"
    )]
    pub open_balance_min: String,

    #[form(
        label = "Open balance max",
        widget = Decimal,
        row = "open_balance",
        when = "posted"
    )]
    pub open_balance_max: String,

    #[form(label = "Date from", widget = Datetime, row = "datetime")]
    pub datetime_from: String,

    #[form(label = "Date to", widget = Datetime, row = "datetime")]
    pub datetime_to: String,

    #[form(label = "Delivery date from", widget = Date, row = "delivery")]
    pub delivery_date_from: String,

    #[form(label = "Delivery date to", widget = Date, row = "delivery")]
    pub delivery_date_to: String,

    #[form(label = "Untaxed amount min", widget = Decimal, row = "untaxed")]
    pub untaxed_min: String,

    #[form(label = "Untaxed amount max", widget = Decimal, row = "untaxed")]
    pub untaxed_max: String,

    #[form(label = "Total amount min", widget = Decimal, row = "total")]
    pub total_min: String,

    #[form(label = "Total amount max", widget = Decimal, row = "total")]
    pub total_max: String,

    #[form(label = "Tax levied min", widget = Decimal, row = "tax")]
    pub tax_min: String,

    #[form(label = "Tax levied max", widget = Decimal, row = "tax")]
    pub tax_max: String,

    #[form(
        label = "Product",
        widget = ForeignKey,
        route = ProductFkSelectRouteTag,
        swap_key = "hub-filter-product",
        display = "product",
        placeholder = "Select product…"
    )]
    pub product_id: String,

    #[form(label = "Number of products min", widget = Number, row = "products")]
    pub product_count_min: String,

    #[form(label = "Number of products max", widget = Number, row = "products")]
    pub product_count_max: String,

    #[form(label = "Final due date from", widget = Date, row = "due")]
    pub final_due_from: String,

    #[form(label = "Final due date to", widget = Date, row = "due")]
    pub final_due_to: String,
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

    #[test]
    fn invoice_hub_filter_form_names_match_table_columns() {
        let posted = InvoiceHubFilterForm::render_inputs(
            &FormCtx::form::<InvoiceHubFilterForm>(CsrfToken::current())
                .flag(InvoiceHubFilterFormFlag::Posted, true),
        )
        .into_string();
        for name in [
            "ID",
            "Number",
            "Customer",
            "OpenBalanceMin",
            "OpenBalanceMax",
            "DatetimeFrom",
            "DatetimeTo",
            "DeliveryDateFrom",
            "DeliveryDateTo",
            "UntaxedMin",
            "UntaxedMax",
            "TotalMin",
            "TotalMax",
            "TaxMin",
            "TaxMax",
            "ProductID",
            "ProductCountMin",
            "ProductCountMax",
            "FinalDueFrom",
            "FinalDueTo",
        ] {
            assert!(
                posted.contains(&format!(r#"name="{name}""#)),
                "missing {name} on posted filter form"
            );
        }
        assert!(
            posted.contains("pick-product"),
            "product filter should be a foreign-key picker"
        );

        let drafts = InvoiceHubFilterForm::render_inputs(
            &FormCtx::form::<InvoiceHubFilterForm>(CsrfToken::current())
                .flag(InvoiceHubFilterFormFlag::Posted, false),
        )
        .into_string();
        assert!(!drafts.contains(r#"name="Customer""#));
        assert!(!drafts.contains(r#"name="OpenBalanceMin""#));
        assert!(drafts.contains(r#"name="Number""#));
    }
}
