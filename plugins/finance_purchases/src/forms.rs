use lariv_core::html_form::{
    FieldRender, FormCtx, FormWidget, html_form,
    widgets::{Checkbox, Date, Datetime, Decimal, Number, Text, Textarea},
};
use lariv_core::html_form::FormFieldKey;
use maud::{Markup, html};

use lariv_plugin_contacts::routes::{CompanyFkSelectRouteTag, ContactFkSelectRouteTag};
use lariv_plugin_filesystem::routes::VNodeFileSelectRouteTag;
use lariv_plugin_finance_accounts::routes::{AccountSelectRouteTag, JournalSelectRouteTag};
use lariv_plugin_finance_products::routes::ProductFkSelectRouteTag;
use lariv_plugin_finance_taxes::routes::TaxMultiSelectRouteTag;

use crate::components::{
    InputPurchaseLinesDraft, InputPaymentTermLinesDraft, input_purchase_lines_draft,
    input_payment_term_lines_draft,
};
/// Custom widget for draft purchase lines (Alpine editor + hidden JSON).
pub struct PurchaseLinesDraft;
impl FormWidget for PurchaseLinesDraft {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_purchase_lines_draft(InputPurchaseLinesDraft {
            name: field.name,
            defaults: field.value,
            preview: ctx.display_of("purchase_lines_preview"),
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

    #[form(label = "Vendor is an individual", widget = Checkbox, model = "billToIndividual")]
    pub vendor_is_individual: String,

    #[form(
        label = "Contact",
        widget = ForeignKey,
        route = ContactFkSelectRouteTag,
        swap_key = "fk-purchase-vendor-individual",
        display = "vendor_contact_id",
        placeholder = "Select contact…",
        show = "billToIndividual"
    )]
    pub vendor_contact_id: i64,

    #[form(
        label = "Company",
        widget = ForeignKey,
        route = CompanyFkSelectRouteTag,
        swap_key = "fk-purchase-vendor-company",
        display = "vendor_company_id",
        placeholder = "Select company…"
    )]
    pub vendor_company_id: i64,

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
        label = "Lines",
        required,
        widget = PurchaseLinesDraft,
        display = "purchase_lines_preview",
        hint = "Quantity multiplies the unit price of one product."
    )]
    pub purchase_lines_json: String,
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

    #[form(label = "Vendor is an individual", widget = Checkbox, model = "billToIndividual")]
    pub vendor_is_individual: String,

    #[form(
        label = "Contact",
        widget = ForeignKey,
        route = ContactFkSelectRouteTag,
        swap_key = "fk-purchase-vendor-individual",
        display = "vendor_contact_id",
        placeholder = "Leave empty to keep existing…",
        show = "billToIndividual"
    )]
    pub vendor_contact_id: i64,

    #[form(
        label = "Company",
        widget = ForeignKey,
        route = CompanyFkSelectRouteTag,
        swap_key = "fk-purchase-vendor-company",
        display = "vendor_company_id",
        placeholder = "Leave empty to keep existing…"
    )]
    pub vendor_company_id: i64,

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
        label = "Lines",
        widget = PurchaseLinesDraft,
        display = "purchase_lines_preview",
        hint = "Quantity multiplies the unit price of one product. Leave lines empty to keep existing lines."
    )]
    pub purchase_lines_json: String,
}

/// Alpine state for the vendor-is-individual checkbox (`x-model="billToIndividual"`).
pub fn individual_x_data(raw: &str) -> String {
    format!(
        "{{ billToIndividual: {} }}",
        if crate::logic::bill_to::checkbox_on(raw) {
            "true"
        } else {
            "false"
        }
    )
}

pub fn individual_is_on(raw: &str) -> bool {
    crate::logic::bill_to::checkbox_on(raw)
}

/// When a contact is picked, copy that contact's company into the company field.
pub fn with_contact_company_prefill(inputs: Markup) -> Markup {
    let handler = contact_company_prefill_js(
        DraftPurchaseFormField::VendorContactId.html_name(),
        DraftPurchaseFormField::VendorCompanyId.html_name(),
    );
    html! {
        div x-data="{}" "@fk-select.window"=(handler.clone()) "@lariv-fk-created.window"=(handler) {
            (inputs)
        }
    }
}

fn contact_company_prefill_js(contact: &str, company: &str) -> String {
    format!(
        "const d=$event.detail;if(!d||d.name!=='{contact}')return;const id=d.company_id;if(id==null||String(id).trim()===''||String(id)==='0')return;$dispatch('fk-select',{{name:'{company}',value:String(id),display:d.company_name||''}})"
    )
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

#[html_form]
pub struct PurchasePreferencesForm {
    #[form(
        label = "Accounts payable",
        widget = ForeignKey,
        route = AccountSelectRouteTag,
        swap_key = "pref-purchase-ap",
        display = "account_payable",
        placeholder = "Select credit account…"
    )]
    pub account_payable_id: String,

    #[form(
        label = "Expense account",
        widget = ForeignKey,
        route = AccountSelectRouteTag,
        swap_key = "pref-purchase-expense",
        display = "account_expense",
        placeholder = "Select debit account…"
    )]
    pub account_expense_id: String,

    #[form(
        label = "Input tax",
        widget = ForeignKey,
        route = AccountSelectRouteTag,
        swap_key = "pref-purchase-tax",
        display = "account_input_tax",
        placeholder = "Select debit account…"
    )]
    pub account_input_tax_id: String,

    #[form(
        label = "Journal (purchases)",
        name = "PurchaseJournalId",
        widget = ForeignKey,
        route = JournalSelectRouteTag,
        swap_key = "pref-purchase-journal",
        display = "purchase_journal",
        placeholder = "Select journal…"
    )]
    pub journal_id: String,
}

#[html_form]
pub struct CancelPurchaseForm {
    #[form(label = "Reason", required, widget = Textarea, rows = 3)]
    pub reason: String,
}

/// Column filters for the purchase hub table. Field order matches the table.
#[html_form]
pub struct PurchaseHubFilterForm {
    #[form(label = "ID", widget = Text, placeholder = "Contains")]
    pub id: String,

    #[form(label = "Number", widget = Text, placeholder = "Contains")]
    pub number: String,

    #[form(label = "Vendor", widget = Text, placeholder = "Contains", when = "posted")]
    pub vendor: String,

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
    fn draft_purchase_create_form_keeps_alpine_in_attributes() {
        let html = DraftPurchaseForm::render_inputs(
            &FormCtx::form::<DraftPurchaseForm>(CsrfToken::current())
                .value(DraftPurchaseFormField::VendorIsIndividual, "")
                .value(DraftPurchaseFormField::VendorCompanyId, "1")
                .display(DraftPurchaseFormField::VendorCompanyId, "Acme Co")
                .value(DraftPurchaseFormField::Datetime, "2025-06-01")
                .value(DraftPurchaseFormField::PaymentTermLinesJson, "")
                .value(DraftPurchaseFormField::PurchaseLinesJson, ""),
        )
        .into_string();
        assert!(html.contains("Vendor is an individual"));
        assert!(html.contains("x-model=\"billToIndividual\""));
        assert!(html.contains("x-show=\"billToIndividual\""));
        assert!(html.contains("Company") || html.contains("Contact"));
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
    fn purchase_hub_filter_form_names_match_table_columns() {
        let posted = PurchaseHubFilterForm::render_inputs(
            &FormCtx::form::<PurchaseHubFilterForm>(CsrfToken::current())
                .flag(PurchaseHubFilterFormFlag::Posted, true),
        )
        .into_string();
        for name in [
            "ID",
            "Number",
            "Vendor",
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

        let drafts = PurchaseHubFilterForm::render_inputs(
            &FormCtx::form::<PurchaseHubFilterForm>(CsrfToken::current())
                .flag(PurchaseHubFilterFormFlag::Posted, false),
        )
        .into_string();
        assert!(!drafts.contains(r#"name="Vendor""#));
        assert!(!drafts.contains(r#"name="OpenBalanceMin""#));
        assert!(drafts.contains(r#"name="Number""#));
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
            PurchasePreferencesFormField::JournalId.html_name(),
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
JournalId=11&PurchaseJournalId=22";
        let company: PurchaseCompanyPreferencesForm =
            lariv_core::html_form::deserialize_urlencoded(body).expect("company form");
        assert_eq!(company.company_name, "Pur Co");
        assert_eq!(company.company_address, "pur-addr");
        assert_eq!(company.company_phone, "2");
        assert_eq!(company.company_gstin, "PUR-GST");
        assert_eq!(company.place_of_supply, "TN");
        assert_eq!(company.default_bank_account, "pur-bank");
        let prefs: PurchasePreferencesForm =
            lariv_core::html_form::deserialize_urlencoded(body).expect("gl form");
        assert_eq!(prefs.journal_id, "22");
    }
}
