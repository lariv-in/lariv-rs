use lariv_core::html_form::{
    FieldRender, FormCtx, FormWidget, html_form, widgets::Date, widgets::Text, widgets::Textarea,
};
use lariv_plugin_crm::routes::CustomerFkSelectRouteTag;
use lariv_plugin_filesystem::routes::VNodeFileSelectRouteTag;
use lariv_plugin_finance_invoices::components::{
    InputInvoiceGoodsLinesDraft, InputInvoiceServiceLinesDraft, input_invoice_goods_lines_draft,
    input_invoice_service_lines_draft,
};

use crate::routes::{QuotationGoodsLinePriceRouteTag, QuotationServiceLinePriceRouteTag};

pub struct QuotationGoodsLines;
impl FormWidget for QuotationGoodsLines {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> maud::Markup {
        let price_url = QuotationGoodsLinePriceRouteTag.url();
        input_invoice_goods_lines_draft(InputInvoiceGoodsLinesDraft {
            name: field.name,
            defaults: field.value,
            preview: ctx.display_of("quotation_goods_lines_preview"),
            price_url: &price_url,
            ..Default::default()
        })
    }
}

pub struct QuotationServiceLines;
impl FormWidget for QuotationServiceLines {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> maud::Markup {
        let price_url = QuotationServiceLinePriceRouteTag.url();
        input_invoice_service_lines_draft(InputInvoiceServiceLinesDraft {
            name: field.name,
            defaults: field.value,
            preview: ctx.display_of("quotation_service_lines_preview"),
            price_url: &price_url,
            ..Default::default()
        })
    }
}

#[html_form]
pub struct QuotationForm {
    #[form(label = "Number", widget = Text, hint = "Leave blank to use the number format from preferences.")]
    pub number: String,

    #[form(label = "Date", required, widget = Date)]
    pub date: String,

    #[form(
        label = "Customer",
        required,
        widget = ForeignKey,
        route = CustomerFkSelectRouteTag,
        swap_key = "fk-quotation-customer",
        display = "customer",
        placeholder = "Select customer…"
    )]
    pub customer_id: i64,

    #[form(label = "Account", widget = Textarea, rows = 6)]
    pub bank_account: String,

    #[form(label = "Remarks", widget = Textarea, rows = 4)]
    pub remarks: String,

    #[form(
        label = "Goods lines",
        widget = QuotationGoodsLines,
        display = "quotation_goods_lines_preview",
        hint = "Quantity multiplies the unit price of one product. A typed unit price is kept."
    )]
    pub goods_lines_json: String,

    #[form(
        label = "Service lines",
        widget = QuotationServiceLines,
        display = "quotation_service_lines_preview",
        hint = "Quantity multiplies the unit price of one service. A typed unit price is kept."
    )]
    pub service_lines_json: String,
}

#[html_form]
pub struct QuotationPreferencesForm {
    #[form(
        label = "Quotation number format",
        widget = Text,
        hint = "Placeholders: {{FISCAL_CODE}}, {{YY}}, {{YYYY}}, {{POSTED_SEQ}}, {{FISCAL_POSTED_SEQ}}. Blank uses QT-{{YYYY}}-{{POSTED_SEQ}}."
    )]
    pub quotation_number_format: String,

    #[form(label = "Company name", widget = Text)]
    pub company_name: String,

    #[form(label = "Company address", widget = Textarea, rows = 4, hint = "Typst markup printed on the quotation PDF.")]
    pub company_address: String,

    #[form(label = "Company phone", widget = Text)]
    pub company_phone: String,

    #[form(label = "Company GSTIN", widget = Text)]
    pub company_gstin: String,

    #[form(label = "Place of supply", widget = Text)]
    pub place_of_supply: String,

    #[form(label = "Default bank account", widget = Textarea, rows = 4)]
    pub default_bank_account: String,

    #[form(
        label = "Logo",
        widget = ForeignKey,
        route = VNodeFileSelectRouteTag,
        swap_key = "fk-quotation-logo",
        display = "logo",
        placeholder = "Select logo…"
    )]
    pub company_logo_vnode_id: String,

    #[form(
        label = "Signature",
        widget = ForeignKey,
        route = VNodeFileSelectRouteTag,
        swap_key = "fk-quotation-signature",
        display = "signature",
        placeholder = "Select signature…"
    )]
    pub company_signature_vnode_id: String,

    #[form(label = "Email subject", widget = Text, hint = "Jinja2. Blank uses the default. Placeholders: invoice_number, date, customer_name, customer_email, grand_total, company_name.")]
    pub quotation_email_subject: String,

    #[form(label = "Email body", widget = Textarea, rows = 8)]
    pub quotation_email_body: String,

    #[form(label = "PDF template", widget = Textarea, rows = 16, hint = "Minijinja template whose output is Typst. Blank uses the built-in quotation template.")]
    pub quotation_pdf_template: String,
}
