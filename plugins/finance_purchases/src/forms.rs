use lariv_core::html_form::{
    FieldRender, FormCtx, FormWidget, html_form,
    widgets::{Date, Text, Textarea},
};
use maud::Markup;

use lariv_plugin_contacts::routes::{CompanyFkSelectRouteTag, ContactFkSelectRouteTag};
use lariv_plugin_filesystem::routes::VNodeFileSelectRouteTag;

use crate::routes::VendorFkSelectRouteTag;
use lariv_plugin_finance_taxes::routes::TaxMultiSelectRouteTag;

use crate::components::{
    InputPaymentTermLinesDraft, InputPurchaseGoodsLinesDraft, InputPurchaseServiceLinesDraft,
    input_payment_term_lines_draft, input_purchase_goods_lines_draft,
    input_purchase_service_lines_draft,
};
/// Custom widget for draft purchase goods lines (Alpine editor + hidden JSON).
pub struct PurchaseGoodsLinesDraft;
impl FormWidget for PurchaseGoodsLinesDraft {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_purchase_goods_lines_draft(InputPurchaseGoodsLinesDraft {
            name: field.name,
            defaults: field.value,
            preview: ctx.display_of("purchase_goods_lines_preview"),
            ..Default::default()
        })
    }
}

/// Custom widget for draft purchase service lines (Alpine editor + hidden JSON).
pub struct PurchaseServiceLinesDraft;
impl FormWidget for PurchaseServiceLinesDraft {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_purchase_service_lines_draft(InputPurchaseServiceLinesDraft {
            name: field.name,
            defaults: field.value,
            preview: ctx.display_of("purchase_service_lines_preview"),
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
pub struct DraftPurchaseForm {
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
        label = "Vendor",
        required,
        widget = ForeignKey,
        route = VendorFkSelectRouteTag,
        swap_key = "fk-purchase-vendor",
        display = "vendor",
        placeholder = "Select vendor…"
    )]
    pub vendor_id: i64,

    #[form(label = "Payment schedule", required, widget = PaymentTermLinesDraft)]
    pub payment_term_lines_json: String,

    #[form(
        label = "Taxes",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "purchase-header-taxes",
        placeholder = "Select taxes…"
    )]
    pub taxes: Vec<i64>,

    #[form(
        label = "Goods lines",
        widget = PurchaseGoodsLinesDraft,
        display = "purchase_goods_lines_preview",
        hint = "Quantity multiplies the unit price of one product."
    )]
    pub purchase_goods_lines_json: String,

    #[form(
        label = "Service lines",
        widget = PurchaseServiceLinesDraft,
        display = "purchase_service_lines_preview",
        hint = "Quantity multiplies the unit price of one service."
    )]
    pub purchase_service_lines_json: String,
}

/// Blank bulk-edit form: only non-empty fields are applied to selected drafts.
#[html_form]
pub struct DraftPurchaseBulkEditForm {
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
        label = "Vendor",
        widget = ForeignKey,
        route = VendorFkSelectRouteTag,
        swap_key = "fk-purchase-vendor",
        display = "vendor",
        placeholder = "Leave empty to keep existing…"
    )]
    pub vendor_id: i64,

    #[form(label = "Payment schedule", widget = PaymentTermLinesDraft)]
    pub payment_term_lines_json: String,

    #[form(
        label = "Taxes",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "purchase-header-taxes",
        placeholder = "Leave empty to keep existing…"
    )]
    pub taxes: Vec<i64>,

    #[form(
        label = "Goods lines",
        widget = PurchaseGoodsLinesDraft,
        display = "purchase_goods_lines_preview",
        hint = "Quantity multiplies the unit price of one product. Leave both line sections empty to keep existing lines."
    )]
    pub purchase_goods_lines_json: String,

    #[form(
        label = "Service lines",
        widget = PurchaseServiceLinesDraft,
        display = "purchase_service_lines_preview",
        hint = "Quantity multiplies the unit price of one service. Leave both line sections empty to keep existing lines."
    )]
    pub purchase_service_lines_json: String,
}

/// Purchase number format + PDF template field names (custom UI on preferences page).
#[html_form]
pub struct PurchasePresentationPreferencesForm {
    #[form(label = "Purchase number format", widget = Text)]
    pub purchase_number_format: String,

    #[form(label = "Purchase date format", widget = Text)]
    pub purchase_date_format: String,

    #[form(label = "Purchase datetime format", widget = Text)]
    pub purchase_datetime_format: String,

    #[form(label = "Purchase PDF template (Typst + Minijinja)", widget = Textarea, rows = 16)]
    pub purchase_pdf_template: String,
}

/// Logo and signature files for purchase PDFs (filesystem VNodes).
#[html_form]
pub struct PurchasePdfAssetPreferencesForm {
    #[form(
        label = "Purchase logo",
        widget = ForeignKey,
        route = VNodeFileSelectRouteTag,
        swap_key = "pref-purchase-logo-vnode",
        display = "purchase_logo_vnode",
        placeholder = "Select logo file…"
    )]
    pub purchase_logo_vnode_id: String,

    #[form(
        label = "Purchase signature",
        widget = ForeignKey,
        route = VNodeFileSelectRouteTag,
        swap_key = "pref-purchase-signature-vnode",
        display = "purchase_signature_vnode",
        placeholder = "Select signature file…"
    )]
    pub purchase_signature_vnode_id: String,
}

/// Company text shown on purchase PDFs (name, address, footer, place of supply).
///
/// HTML names are prefixed so they do not collide with the invoice company fields
/// on the shared `/finance/preferences` form. Duplicate names become JSON arrays
/// and the purchase form then fails to deserialize, so nothing is written.
#[html_form]
pub struct PurchaseCompanyPreferencesForm {
    #[form(label = "Company name", name = "PurchaseCompanyName", widget = Text)]
    pub company_name: String,

    #[form(
        label = "Company address (Typst)",
        name = "PurchaseCompanyAddress",
        widget = Textarea,
        rows = 4
    )]
    pub company_address: String,

    #[form(label = "Company phone", name = "PurchaseCompanyPhone", widget = Text)]
    pub company_phone: String,

    #[form(label = "Company GSTIN", name = "PurchaseCompanyGstin", widget = Text)]
    pub company_gstin: String,

    #[form(
        label = "Default place of supply",
        name = "PurchasePlaceOfSupply",
        widget = Text
    )]
    pub place_of_supply: String,

    #[form(
        label = "Default account",
        name = "PurchaseDefaultBankAccount",
        widget = Textarea,
        rows = 6
    )]
    pub default_bank_account: String,
}

/// A vendor is one company or one contact. Exactly one of the two is set.
#[html_form]
pub struct VendorForm {
    #[form(
        label = "Company",
        widget = ForeignKey,
        route = CompanyFkSelectRouteTag,
        swap_key = "purchase-vendor-company",
        display = "company",
        placeholder = "Select company…"
    )]
    pub company_id: i64,

    #[form(
        label = "Contact",
        widget = ForeignKey,
        route = ContactFkSelectRouteTag,
        swap_key = "purchase-vendor-contact",
        display = "individual",
        placeholder = "Select contact…"
    )]
    pub individual_id: i64,
}

#[html_form]
pub struct CancelPurchaseForm {
    #[form(label = "Reason", required, widget = Textarea, rows = 3)]
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use lariv_core::components::attrs::alpine_js_leaked_as_text;
    use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};

    #[test]
    fn draft_purchase_create_form_keeps_alpine_in_attributes() {
        let html = DraftPurchaseForm::render_inputs(
            &FormCtx::form::<DraftPurchaseForm>(CsrfToken::current())
                .value(DraftPurchaseFormField::VendorId, "1")
                .display(DraftPurchaseFormField::VendorId, "Acme Co")
                .value(DraftPurchaseFormField::Datetime, "2025-06-01")
                .value(DraftPurchaseFormField::PaymentTermLinesJson, "")
                .value(DraftPurchaseFormField::PurchaseGoodsLinesJson, ""),
        )
        .into_string();
        assert!(html.contains("Vendor"));
        assert!(html.contains("fk-purchase-vendor"));
        assert!(!html.contains("billToIndividual"));
        assert!(
            !html.contains(r#"querySelector('input[type="hidden"]"#),
            "Alpine JS leaked as text on the draft purchase form: {html}"
        );
        assert!(
            !alpine_js_leaked_as_text(&html),
            "Alpine JS rendered as text on the draft purchase form: {html}"
        );
    }

    #[test]
    fn purchase_preference_fields_do_not_share_invoice_html_names() {
        use lariv_core::html_form::FormFieldKey;

        // Invoice preferences on the same page already submit these names.
        // A second value is folded into a JSON array and `String` fields fail to parse.
        let invoice_names = [
            "CompanyName",
            "CompanyAddress",
            "CompanyPhone",
            "CompanyGstin",
            "PlaceOfSupply",
            "DefaultBankAccount",
            "JournalId",
        ];
        let purchase_names = [
            PurchaseCompanyPreferencesFormField::CompanyName.html_name(),
            PurchaseCompanyPreferencesFormField::CompanyAddress.html_name(),
            PurchaseCompanyPreferencesFormField::CompanyPhone.html_name(),
            PurchaseCompanyPreferencesFormField::CompanyGstin.html_name(),
            PurchaseCompanyPreferencesFormField::PlaceOfSupply.html_name(),
            PurchaseCompanyPreferencesFormField::DefaultBankAccount.html_name(),
        ];
        for name in purchase_names {
            assert!(
                !invoice_names.contains(&name),
                "{name} collides with an invoice preference field"
            );
        }

        let body = b"CompanyName=Inv%20Co&PurchaseCompanyName=Pur%20Co&\
CompanyAddress=inv-addr&PurchaseCompanyAddress=pur-addr&\
CompanyPhone=1&PurchaseCompanyPhone=2&\
CompanyGstin=INV-GST&PurchaseCompanyGstin=PUR-GST&\
PlaceOfSupply=KA&PurchasePlaceOfSupply=TN&\
DefaultBankAccount=inv-bank&PurchaseDefaultBankAccount=pur-bank&\
JournalId=11";
        let company: PurchaseCompanyPreferencesForm =
            lariv_core::html_form::deserialize_urlencoded(body).expect("company form");
        assert_eq!(company.company_name, "Pur Co");
        assert_eq!(company.company_address, "pur-addr");
        assert_eq!(company.company_phone, "2");
        assert_eq!(company.company_gstin, "PUR-GST");
        assert_eq!(company.place_of_supply, "TN");
        assert_eq!(company.default_bank_account, "pur-bank");
    }
}
