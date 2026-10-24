//! Patches purchase presentation preferences onto `/finance/preferences`.

use chrono::Utc;
use lariv_core::components::{
    CodeEditorInput,
    attrs::escape_attr,
    code_editor_input,
    htmx::{HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL},
    label_hint,
};
use lariv_core::html_form::FormFieldKey;
use lariv_plugin_finance_accounts::accounting_preferences_patch::{
    AccountingPreferenceGroup, AccountingPreferencesAddon, opt_i64_form_value, str_to_opt_i64,
    str_to_opt_string,
};
use maud::{PreEscaped, html};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use crate::{
    entities::preferences::{self},
    forms::{
        PurchaseCompanyPreferencesForm, PurchaseCompanyPreferencesFormField,
        PurchasePdfAssetPreferencesForm, PurchasePdfAssetPreferencesFormField,
        PurchasePresentationPreferencesForm, PurchasePresentationPreferencesFormField,
    },
    logic::preferences::load_purchase_preferences,
    preferences_hints::{
        PURCHASE_DATE_FORMAT_HINT, PURCHASE_DATETIME_FORMAT_HINT, PURCHASE_NUMBER_FORMAT_HINT,
        PURCHASE_PDF_TEMPLATE_HINT,
    },
    purchase_pdf_template::DEFAULT_PURCHASE_PDF_TEMPLATE,
    purchase_products::forms::{ProductJournalPreferencesForm, ProductJournalPreferencesFormField},
    purchase_services::forms::{ServiceJournalPreferencesForm, ServiceJournalPreferencesFormField},
};
use lariv_plugin_filesystem::entities::filesystem_node::Entity as VNodeEntity;

async fn load_vnode_display(db: &DatabaseConnection, id: Option<i64>) -> String {
    let Some(id) = id.filter(|&id| id > 0) else {
        return String::new();
    };
    lariv_core::web::opt_or_log(VNodeEntity::find_by_id(id).one(db).await, "find by id")
        .map(|n| n.name)
        .unwrap_or_default()
}

fn fk_value(id: Option<i64>) -> String {
    opt_i64_form_value(id)
}

pub(crate) struct PurchasesAccountingPreferencesAddon;

#[async_trait::async_trait]
impl AccountingPreferencesAddon for PurchasesAccountingPreferencesAddon {
    fn id(&self) -> &'static str {
        "finance-purchases"
    }

    async fn render_groups(&self, db: &DatabaseConnection) -> Vec<AccountingPreferenceGroup> {
        use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};

        let inv = load_purchase_preferences(db).await;

        let number_format = inv.purchase_number_format.unwrap_or_default();
        let date_format = inv.purchase_date_format.unwrap_or_default();
        let datetime_format = inv.purchase_datetime_format.unwrap_or_default();
        let pdf_template = inv.purchase_pdf_template.unwrap_or_default();
        let logo_display = load_vnode_display(db, inv.purchase_logo_vnode_id).await;
        let signature_display = load_vnode_display(db, inv.purchase_signature_vnode_id).await;
        let product_journal = crate::purchase_products::journal_template::preference_form_fields(
            db,
            "purchase-product-pref-line",
        )
        .await;
        let service_journal = crate::purchase_services::journal_template::preference_form_fields(
            db,
            "purchase-service-pref-line",
        )
        .await;

        vec![
            AccountingPreferenceGroup {
                title: "Purchases",
                inputs: html! {
                (label_hint(
                    "Purchase number format",
                    Some(PURCHASE_NUMBER_FORMAT_HINT),
                    html! {
                        input type="text"
                            name=(PurchasePresentationPreferencesFormField::PurchaseNumberFormat.html_name())
                            class="input input-bordered w-full"
                            value=(number_format) {}
                    },
                ))
                (label_hint(
                    "Purchase date format",
                    Some(PURCHASE_DATE_FORMAT_HINT),
                    html! {
                        input type="text"
                            name=(PurchasePresentationPreferencesFormField::PurchaseDateFormat.html_name())
                            class="input input-bordered w-full"
                            value=(date_format)
                            placeholder="%d/%m/%Y" {}
                    },
                ))
                (label_hint(
                    "Purchase datetime format",
                    Some(PURCHASE_DATETIME_FORMAT_HINT),
                    html! {
                        input type="text"
                            name=(PurchasePresentationPreferencesFormField::PurchaseDatetimeFormat.html_name())
                            class="input input-bordered w-full"
                            value=(datetime_format)
                            placeholder="%d/%m/%Y" {}
                    },
                ))
                (PurchasePdfAssetPreferencesForm::render_inputs(
                    &FormCtx::form::<PurchasePdfAssetPreferencesForm>(CsrfToken::current())
                        .value(
                            PurchasePdfAssetPreferencesFormField::PurchaseLogoVnodeId,
                            fk_value(inv.purchase_logo_vnode_id),
                        )
                        .display(
                            PurchasePdfAssetPreferencesFormField::PurchaseLogoVnodeId,
                            &logo_display,
                        )
                        .value(
                            PurchasePdfAssetPreferencesFormField::PurchaseSignatureVnodeId,
                            fk_value(inv.purchase_signature_vnode_id),
                        )
                        .display(
                            PurchasePdfAssetPreferencesFormField::PurchaseSignatureVnodeId,
                            &signature_display,
                        ),
                ))
                (PurchaseCompanyPreferencesForm::render_inputs(
                    &FormCtx::form::<PurchaseCompanyPreferencesForm>(CsrfToken::current())
                        .value(
                            PurchaseCompanyPreferencesFormField::CompanyName,
                            inv.company_name.as_deref().unwrap_or_default(),
                        )
                        .value(
                            PurchaseCompanyPreferencesFormField::CompanyAddress,
                            inv.company_address.as_deref().unwrap_or_default(),
                        )
                        .value(
                            PurchaseCompanyPreferencesFormField::CompanyPhone,
                            inv.company_phone.as_deref().unwrap_or_default(),
                        )
                        .value(
                            PurchaseCompanyPreferencesFormField::CompanyGstin,
                            inv.company_gstin.as_deref().unwrap_or_default(),
                        )
                        .value(
                            PurchaseCompanyPreferencesFormField::PlaceOfSupply,
                            inv.place_of_supply.as_deref().unwrap_or_default(),
                        )
                        .value(
                            PurchaseCompanyPreferencesFormField::DefaultBankAccount,
                            inv.default_bank_account.as_deref().unwrap_or_default(),
                        ),
                ))
                (label_hint(
                    "Purchase PDF template (Typst + Minijinja)",
                    Some(PURCHASE_PDF_TEMPLATE_HINT),
                    html! {
                        (code_editor_input(CodeEditorInput {
                            label: "",
                            name: PurchasePresentationPreferencesFormField::PurchasePdfTemplate.html_name(),
                            value: &pdf_template,
                            id: "purchase-pdf-template-field",
                            language: "plaintext",
                            rows: 16,
                            max_height: "24rem",
                            ..Default::default()
                        }))
                        textarea id="default-purchase-pdf-template" hidden readonly {
                            (DEFAULT_PURCHASE_PDF_TEMPLATE)
                        }
                        div class="flex justify-end gap-2 mt-2" {
                            button type="button" class="btn btn-ghost btn-sm"
                                onclick="if (confirm('This will overwrite the template in the field with the default example template. Continue?')) { const ta = document.getElementById('purchase-pdf-template-field'); const def = document.getElementById('default-purchase-pdf-template'); if (!ta || !def) return; ta.value = def.value; const root = ta.closest('[data-code-editor-root]'); if (root) { root.dispatchEvent(new CustomEvent('code-editor:set', { detail: { value: def.value } })); } else { ta.dispatchEvent(new Event('change', { bubbles: true })); } }" {
                                "Use default template"
                            }
                            div class="fk-modal-host" {
                                (PreEscaped(format!(
                                    r#"<button type="button" class="btn btn-outline btn-sm" hx-post="{}" hx-target="{}" hx-swap="{}" hx-include="closest form" hx-push-url="false">Preview sample PDF</button>"#,
                                    crate::routes::PurchasePdfPreviewPostRouteTag.path(),
                                    escape_attr(HTMX_TARGET_BODY_MODAL),
                                    escape_attr(HTMX_SWAP_BODY_MODAL),
                                )))
                            }
                        }
                    },
                ))
                },
            },
            AccountingPreferenceGroup {
                title: "Purchase products",
                inputs: html! {
                    (ProductJournalPreferencesForm::render_inputs(
                        &FormCtx::form::<ProductJournalPreferencesForm>(CsrfToken::current())
                            .value(
                                ProductJournalPreferencesFormField::JournalId,
                                &product_journal.journal_id,
                            )
                            .display(
                                ProductJournalPreferencesFormField::JournalId,
                                &product_journal.journal_display,
                            )
                            .value(
                                ProductJournalPreferencesFormField::JournalLines,
                                &product_journal.journal_lines,
                            ),
                    ))
                },
            },
            AccountingPreferenceGroup {
                title: "Purchase services",
                inputs: html! {
                    (ServiceJournalPreferencesForm::render_inputs(
                        &FormCtx::form::<ServiceJournalPreferencesForm>(CsrfToken::current())
                            .value(
                                ServiceJournalPreferencesFormField::JournalId,
                                &service_journal.journal_id,
                            )
                            .display(
                                ServiceJournalPreferencesFormField::JournalId,
                                &service_journal.journal_display,
                            )
                            .value(
                                ServiceJournalPreferencesFormField::JournalLines,
                                &service_journal.journal_lines,
                            ),
                    ))
                },
            },
        ]
    }

    async fn save_from_form(
        &self,
        db: &DatabaseConnection,
        post: &lariv_plugin_finance_accounts::accounting_preferences_patch::AccountingPreferencesPost,
    ) -> Result<(), String> {
        let presentation = post
            .deserialize::<PurchasePresentationPreferencesForm>()
            .map_err(|e| e.to_string())?;
        let assets = post
            .deserialize::<PurchasePdfAssetPreferencesForm>()
            .map_err(|e| e.to_string())?;
        let company = post
            .deserialize::<PurchaseCompanyPreferencesForm>()
            .map_err(|e| e.to_string())?;

        let now = Utc::now();

        let inv_prefs = load_purchase_preferences(db).await;
        let mut inv_am: preferences::ActiveModel = inv_prefs.into();
        inv_am.purchase_number_format =
            Set(str_to_opt_string(&presentation.purchase_number_format));
        inv_am.purchase_date_format = Set(str_to_opt_string(&presentation.purchase_date_format));
        inv_am.purchase_datetime_format =
            Set(str_to_opt_string(&presentation.purchase_datetime_format));
        inv_am.purchase_logo_vnode_id = Set(str_to_opt_i64(&assets.purchase_logo_vnode_id));
        inv_am.purchase_signature_vnode_id =
            Set(str_to_opt_i64(&assets.purchase_signature_vnode_id));
        inv_am.company_name = Set(str_to_opt_string(&company.company_name));
        inv_am.company_address = Set(str_to_opt_string(&company.company_address));
        inv_am.company_phone = Set(str_to_opt_string(&company.company_phone));
        inv_am.company_gstin = Set(str_to_opt_string(&company.company_gstin));
        inv_am.place_of_supply = Set(str_to_opt_string(&company.place_of_supply));
        inv_am.default_bank_account = Set(str_to_opt_string(&company.default_bank_account));
        inv_am.purchase_pdf_template = Set(str_to_opt_string(&presentation.purchase_pdf_template));
        inv_am.updated_at = Set(Some(now));
        inv_am.update(db).await.map_err(|e| e.to_string())?;

        let product_journal = post
            .deserialize::<ProductJournalPreferencesForm>()
            .map_err(|e| e.to_string())?;
        crate::purchase_products::journal_template::save_preference_template(
            db,
            &product_journal.journal_id,
            &product_journal.journal_lines,
        )
        .await?;
        let service_journal = post
            .deserialize::<ServiceJournalPreferencesForm>()
            .map_err(|e| e.to_string())?;
        crate::purchase_services::journal_template::save_preference_template(
            db,
            &service_journal.journal_id,
            &service_journal.journal_lines,
        )
        .await?;

        Ok(())
    }
}

pub(crate) static PURCHASES_ADDON: PurchasesAccountingPreferencesAddon =
    PurchasesAccountingPreferencesAddon;
