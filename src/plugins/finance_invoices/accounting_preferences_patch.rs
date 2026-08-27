//! Patches invoice presentation + GL preferences onto `/finance/preferences`.

use crate::components::{
    CodeEditorInput,
    attrs::escape_attr,
    code_editor_input,
    htmx::{HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL},
    label_hint,
};
use crate::html_form::FormFieldKey;
use crate::plugins::finance_accounts::{
    account_select_route_url,
    accounting_preferences_patch::{AccountingPreferencesAddon, str_to_opt_i64, str_to_opt_string},
    logic::journal::{credit_balance_type, debit_balance_type},
    scope::{load_account_parent_label, load_journal_display_label},
};
use crate::plugins::finance_products::preferences::optional_i64;
use chrono::Utc;
use maud::{Markup, PreEscaped, html};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use crate::plugins::filesystem::entities::filesystem_node::Entity as VNodeEntity;
use crate::plugins::finance_invoices::{
    entities::{
        payment_preferences::{self},
        preferences::{self},
    },
    forms::{
        InvoiceCompanyPreferencesForm, InvoiceCompanyPreferencesFormField,
        InvoicePdfAssetPreferencesForm, InvoicePdfAssetPreferencesFormField,
        InvoicePreferencesForm, InvoicePreferencesFormField, InvoicePresentationPreferencesForm,
        InvoicePresentationPreferencesFormField, PaymentPreferencesForm,
        PaymentPreferencesFormField,
    },
    invoice_pdf_template::DEFAULT_INVOICE_PDF_TEMPLATE,
    logic::preferences::{load_invoice_preferences, load_payment_preferences},
    preferences_hints::{
        INVOICE_DATE_FORMAT_HINT, INVOICE_DATETIME_FORMAT_HINT, INVOICE_NUMBER_FORMAT_HINT,
        INVOICE_PDF_TEMPLATE_HINT,
    },
};

async fn load_vnode_display(db: &DatabaseConnection, id: Option<i64>) -> String {
    let Some(id) = id.filter(|&id| id > 0) else {
        return String::new();
    };
    crate::web::opt_or_log(VNodeEntity::find_by_id(id).one(db).await, "find by id")
        .map(|n| n.name)
        .unwrap_or_default()
}

fn fk_value(id: Option<i64>) -> String {
    optional_i64(id).to_string()
}

pub(crate) struct InvoicesAccountingPreferencesAddon;

#[async_trait::async_trait]
impl AccountingPreferencesAddon for InvoicesAccountingPreferencesAddon {
    fn id(&self) -> &'static str {
        "finance-invoices"
    }

    async fn render_inputs(&self, db: &DatabaseConnection) -> Markup {
        use crate::html_form::{FormCtx, HtmlForm};

        let inv = load_invoice_preferences(db).await;
        let pay = load_payment_preferences(db).await;

        let ar_display = load_account_parent_label(db, inv.account_receivable_id).await;
        let revenue_display = load_account_parent_label(db, inv.account_revenue_id).await;
        let tax_display = load_account_parent_label(db, inv.account_tax_payable_id).await;
        let journal_display = load_journal_display_label(db, inv.journal_id).await;
        let payment_display = load_account_parent_label(db, pay.payment_account_id).await;

        let debit_url = account_select_route_url(debit_balance_type().as_str());
        let credit_url = account_select_route_url(credit_balance_type().as_str());

        let number_format = inv.invoice_number_format.unwrap_or_default();
        let date_format = inv.invoice_date_format.unwrap_or_default();
        let datetime_format = inv.invoice_datetime_format.unwrap_or_default();
        let pdf_template = inv.invoice_pdf_template.unwrap_or_default();
        let logo_display = load_vnode_display(db, inv.invoice_logo_vnode_id).await;
        let signature_display = load_vnode_display(db, inv.invoice_signature_vnode_id).await;

        html! {
            (label_hint(
                "Invoice number format",
                Some(INVOICE_NUMBER_FORMAT_HINT),
                html! {
                    input type="text"
                        name=(InvoicePresentationPreferencesFormField::InvoiceNumberFormat.html_name())
                        class="input input-bordered w-full"
                        value=(number_format) {}
                },
            ))
            (label_hint(
                "Invoice date format",
                Some(INVOICE_DATE_FORMAT_HINT),
                html! {
                    input type="text"
                        name=(InvoicePresentationPreferencesFormField::InvoiceDateFormat.html_name())
                        class="input input-bordered w-full"
                        value=(date_format)
                        placeholder="%d/%m/%Y" {}
                },
            ))
            (label_hint(
                "Invoice datetime format",
                Some(INVOICE_DATETIME_FORMAT_HINT),
                html! {
                    input type="text"
                        name=(InvoicePresentationPreferencesFormField::InvoiceDatetimeFormat.html_name())
                        class="input input-bordered w-full"
                        value=(datetime_format)
                        placeholder="%d/%m/%Y" {}
                },
            ))
            (InvoicePdfAssetPreferencesForm::render_inputs(
                &FormCtx::form::<InvoicePdfAssetPreferencesForm>()
                    .value(
                        InvoicePdfAssetPreferencesFormField::InvoiceLogoVnodeId,
                        fk_value(inv.invoice_logo_vnode_id),
                    )
                    .display(
                        InvoicePdfAssetPreferencesFormField::InvoiceLogoVnodeId,
                        &logo_display,
                    )
                    .value(
                        InvoicePdfAssetPreferencesFormField::InvoiceSignatureVnodeId,
                        fk_value(inv.invoice_signature_vnode_id),
                    )
                    .display(
                        InvoicePdfAssetPreferencesFormField::InvoiceSignatureVnodeId,
                        &signature_display,
                    ),
            ))
            (InvoiceCompanyPreferencesForm::render_inputs(
                &FormCtx::form::<InvoiceCompanyPreferencesForm>()
                    .value(
                        InvoiceCompanyPreferencesFormField::CompanyName,
                        inv.company_name.as_deref().unwrap_or_default(),
                    )
                    .value(
                        InvoiceCompanyPreferencesFormField::CompanyAddress,
                        inv.company_address.as_deref().unwrap_or_default(),
                    )
                    .value(
                        InvoiceCompanyPreferencesFormField::CompanyPhone,
                        inv.company_phone.as_deref().unwrap_or_default(),
                    )
                    .value(
                        InvoiceCompanyPreferencesFormField::CompanyGstin,
                        inv.company_gstin.as_deref().unwrap_or_default(),
                    )
                    .value(
                        InvoiceCompanyPreferencesFormField::PlaceOfSupply,
                        inv.place_of_supply.as_deref().unwrap_or_default(),
                    ),
            ))
            (label_hint(
                "Invoice PDF template (Typst + Minijinja)",
                Some(INVOICE_PDF_TEMPLATE_HINT),
                html! {
                    (code_editor_input(CodeEditorInput {
                        label: "",
                        name: InvoicePresentationPreferencesFormField::InvoicePdfTemplate.html_name(),
                        value: &pdf_template,
                        id: "invoice-pdf-template-field",
                        language: "plaintext",
                        rows: 16,
                        max_height: "24rem",
                        ..Default::default()
                    }))
                    textarea id="default-invoice-pdf-template" hidden readonly {
                        (DEFAULT_INVOICE_PDF_TEMPLATE)
                    }
                    div class="flex justify-end gap-2 mt-2" {
                        button type="button" class="btn btn-ghost btn-sm"
                            onclick="if (confirm('This will overwrite the template in the field with the default example template. Continue?')) { const ta = document.getElementById('invoice-pdf-template-field'); const def = document.getElementById('default-invoice-pdf-template'); if (!ta || !def) return; ta.value = def.value; const root = ta.closest('[data-code-editor-root]'); if (root) { root.dispatchEvent(new CustomEvent('code-editor:set', { detail: { value: def.value } })); } else { ta.dispatchEvent(new Event('change', { bubbles: true })); } }" {
                            "Use default template"
                        }
                        div class="fk-modal-host" {
                            (PreEscaped(format!(
                                r#"<button type="button" class="btn btn-outline btn-sm" hx-post="/finance-invoices/invoice-pdf-preview" hx-target="{}" hx-swap="{}" hx-include="closest form" hx-push-url="false">Preview sample PDF</button>"#,
                                escape_attr(HTMX_TARGET_BODY_MODAL),
                                escape_attr(HTMX_SWAP_BODY_MODAL),
                            )))
                        }
                    }
                },
            ))
            (InvoicePreferencesForm::render_inputs(
                &FormCtx::form::<InvoicePreferencesForm>()
                    .value(
                        InvoicePreferencesFormField::AccountReceivableId,
                        fk_value(inv.account_receivable_id),
                    )
                    .display(InvoicePreferencesFormField::AccountReceivableId, &ar_display)
                    .url(InvoicePreferencesFormField::AccountReceivableId, &debit_url)
                    .value(
                        InvoicePreferencesFormField::AccountRevenueId,
                        fk_value(inv.account_revenue_id),
                    )
                    .display(InvoicePreferencesFormField::AccountRevenueId, &revenue_display)
                    .url(InvoicePreferencesFormField::AccountRevenueId, &credit_url)
                    .value(
                        InvoicePreferencesFormField::AccountTaxPayableId,
                        fk_value(inv.account_tax_payable_id),
                    )
                    .display(
                        InvoicePreferencesFormField::AccountTaxPayableId,
                        &tax_display,
                    )
                    .url(InvoicePreferencesFormField::AccountTaxPayableId, &credit_url)
                    .value(
                        InvoicePreferencesFormField::JournalId,
                        fk_value(inv.journal_id),
                    )
                    .display(InvoicePreferencesFormField::JournalId, &journal_display),
            ))
            (PaymentPreferencesForm::render_inputs(
                &FormCtx::form::<PaymentPreferencesForm>()
                    .value(
                        PaymentPreferencesFormField::PaymentAccountId,
                        fk_value(pay.payment_account_id),
                    )
                    .display(PaymentPreferencesFormField::PaymentAccountId, &payment_display)
                    .url(PaymentPreferencesFormField::PaymentAccountId, &debit_url),
            ))
        }
    }

    async fn save_from_form(
        &self,
        db: &DatabaseConnection,
        post: &crate::plugins::finance_accounts::accounting_preferences_patch::AccountingPreferencesPost,
    ) -> Result<(), String> {
        let inv_form = post
            .deserialize::<InvoicePreferencesForm>()
            .map_err(|e| e.to_string())?;
        let presentation = post
            .deserialize::<InvoicePresentationPreferencesForm>()
            .map_err(|e| e.to_string())?;
        let assets = post
            .deserialize::<InvoicePdfAssetPreferencesForm>()
            .map_err(|e| e.to_string())?;
        let company = post
            .deserialize::<InvoiceCompanyPreferencesForm>()
            .map_err(|e| e.to_string())?;
        let payment = post
            .deserialize::<PaymentPreferencesForm>()
            .map_err(|e| e.to_string())?;

        let now = Utc::now();

        let inv_prefs = load_invoice_preferences(db).await;
        let mut inv_am: preferences::ActiveModel = inv_prefs.into();
        inv_am.account_receivable_id = Set(str_to_opt_i64(&inv_form.account_receivable_id));
        inv_am.account_revenue_id = Set(str_to_opt_i64(&inv_form.account_revenue_id));
        inv_am.account_tax_payable_id = Set(str_to_opt_i64(&inv_form.account_tax_payable_id));
        inv_am.journal_id = Set(str_to_opt_i64(&inv_form.journal_id));
        inv_am.invoice_number_format = Set(str_to_opt_string(&presentation.invoice_number_format));
        inv_am.invoice_date_format = Set(str_to_opt_string(&presentation.invoice_date_format));
        inv_am.invoice_datetime_format =
            Set(str_to_opt_string(&presentation.invoice_datetime_format));
        inv_am.invoice_logo_vnode_id = Set(str_to_opt_i64(&assets.invoice_logo_vnode_id));
        inv_am.invoice_signature_vnode_id = Set(str_to_opt_i64(&assets.invoice_signature_vnode_id));
        inv_am.company_name = Set(str_to_opt_string(&company.company_name));
        inv_am.company_address = Set(str_to_opt_string(&company.company_address));
        inv_am.company_phone = Set(str_to_opt_string(&company.company_phone));
        inv_am.company_gstin = Set(str_to_opt_string(&company.company_gstin));
        inv_am.place_of_supply = Set(str_to_opt_string(&company.place_of_supply));
        inv_am.invoice_pdf_template = Set(str_to_opt_string(&presentation.invoice_pdf_template));
        inv_am.updated_at = Set(Some(now));
        inv_am.update(db).await.map_err(|e| e.to_string())?;

        let pay_prefs = load_payment_preferences(db).await;
        let mut pay_am: payment_preferences::ActiveModel = pay_prefs.into();
        pay_am.payment_account_id = Set(str_to_opt_i64(&payment.payment_account_id));
        pay_am.updated_at = Set(Some(now));
        pay_am.update(db).await.map_err(|e| e.to_string())?;

        Ok(())
    }
}

pub(crate) static INVOICES_ADDON: InvoicesAccountingPreferencesAddon =
    InvoicesAccountingPreferencesAddon;
