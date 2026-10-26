//! Quotation preferences singleton and the accounting preferences addon.

use chrono::Utc;
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_plugin_filesystem::entities::filesystem_node::Entity as VNodeEntity;
use lariv_plugin_finance_accounts::accounting_preferences_patch::{
    AccountingPreferenceGroup, AccountingPreferencesAddon, AccountingPreferencesPost,
    opt_i64_form_value, str_to_opt_i64, str_to_opt_string,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use crate::entities::preferences::{self, Entity as PreferencesEntity};
use crate::forms::{QuotationPreferencesForm, QuotationPreferencesFormField};
use crate::logic::number::DEFAULT_QUOTATION_NUMBER_FORMAT;

pub async fn load_preferences(db: &DatabaseConnection) -> preferences::Model {
    if let Ok(Some(row)) = PreferencesEntity::find_by_id(1i64).one(db).await {
        return row;
    }
    let now = Utc::now();
    let format = DEFAULT_QUOTATION_NUMBER_FORMAT.to_string();
    let am = preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        quotation_number_format: Set(Some(format.clone())),
        ..Default::default()
    };
    am.insert(db).await.unwrap_or(preferences::Model {
        id: 1,
        created_at: Some(now),
        updated_at: Some(now),
        quotation_number_format: Some(format),
        quotation_pdf_template: None,
        quotation_email_subject: None,
        quotation_email_body: None,
        company_name: None,
        company_address: None,
        company_phone: None,
        company_gstin: None,
        place_of_supply: None,
        default_bank_account: None,
        company_logo_vnode_id: None,
        company_signature_vnode_id: None,
    })
}

fn opt_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

async fn vnode_name(db: &DatabaseConnection, id: Option<i64>) -> String {
    let Some(id) = id.filter(|id| *id > 0) else {
        return String::new();
    };
    VNodeEntity::find_by_id(id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|node| node.name)
        .unwrap_or_default()
}

pub(crate) struct QuotationsAccountingPreferencesAddon;

#[async_trait::async_trait]
impl AccountingPreferencesAddon for QuotationsAccountingPreferencesAddon {
    fn id(&self) -> &'static str {
        "finance-quotations"
    }

    async fn render_groups(&self, db: &DatabaseConnection) -> Vec<AccountingPreferenceGroup> {
        let prefs = load_preferences(db).await;
        let logo = vnode_name(db, prefs.company_logo_vnode_id).await;
        let signature = vnode_name(db, prefs.company_signature_vnode_id).await;
        let inputs = QuotationPreferencesForm::render_inputs(
            &FormCtx::form::<QuotationPreferencesForm>(CsrfToken::current())
                .value(
                    QuotationPreferencesFormField::QuotationNumberFormat,
                    prefs.quotation_number_format.as_deref().unwrap_or_default(),
                )
                .value(
                    QuotationPreferencesFormField::CompanyName,
                    prefs.company_name.as_deref().unwrap_or_default(),
                )
                .value(
                    QuotationPreferencesFormField::CompanyAddress,
                    prefs.company_address.as_deref().unwrap_or_default(),
                )
                .value(
                    QuotationPreferencesFormField::CompanyPhone,
                    prefs.company_phone.as_deref().unwrap_or_default(),
                )
                .value(
                    QuotationPreferencesFormField::CompanyGstin,
                    prefs.company_gstin.as_deref().unwrap_or_default(),
                )
                .value(
                    QuotationPreferencesFormField::PlaceOfSupply,
                    prefs.place_of_supply.as_deref().unwrap_or_default(),
                )
                .value(
                    QuotationPreferencesFormField::DefaultBankAccount,
                    prefs.default_bank_account.as_deref().unwrap_or_default(),
                )
                .value(
                    QuotationPreferencesFormField::CompanyLogoVnodeId,
                    opt_i64_form_value(prefs.company_logo_vnode_id),
                )
                .display(QuotationPreferencesFormField::CompanyLogoVnodeId, &logo)
                .value(
                    QuotationPreferencesFormField::CompanySignatureVnodeId,
                    opt_i64_form_value(prefs.company_signature_vnode_id),
                )
                .display(
                    QuotationPreferencesFormField::CompanySignatureVnodeId,
                    &signature,
                )
                .value(
                    QuotationPreferencesFormField::QuotationEmailSubject,
                    prefs.quotation_email_subject.as_deref().unwrap_or_default(),
                )
                .value(
                    QuotationPreferencesFormField::QuotationEmailBody,
                    prefs.quotation_email_body.as_deref().unwrap_or_default(),
                )
                .value(
                    QuotationPreferencesFormField::QuotationPdfTemplate,
                    prefs.quotation_pdf_template.as_deref().unwrap_or_default(),
                ),
        );
        vec![AccountingPreferenceGroup {
            title: "Quotations",
            inputs,
        }]
    }

    async fn save_from_form(
        &self,
        db: &DatabaseConnection,
        post: &AccountingPreferencesPost,
    ) -> Result<(), String> {
        let form: QuotationPreferencesForm = post.deserialize().map_err(|err| err.to_string())?;
        let now = Utc::now();
        let existing = PreferencesEntity::find_by_id(1i64)
            .one(db)
            .await
            .map_err(|err| err.to_string())?;
        let number_format = opt_text(&form.quotation_number_format)
            .or_else(|| Some(DEFAULT_QUOTATION_NUMBER_FORMAT.to_string()));
        if let Some(row) = existing {
            let mut am: preferences::ActiveModel = row.into();
            am.updated_at = Set(Some(now));
            am.quotation_number_format = Set(number_format);
            am.quotation_pdf_template = Set(str_to_opt_string(&form.quotation_pdf_template));
            am.quotation_email_subject = Set(opt_text(&form.quotation_email_subject));
            am.quotation_email_body = Set(opt_text(&form.quotation_email_body));
            am.company_name = Set(opt_text(&form.company_name));
            am.company_address = Set(str_to_opt_string(&form.company_address));
            am.company_phone = Set(opt_text(&form.company_phone));
            am.company_gstin = Set(opt_text(&form.company_gstin));
            am.place_of_supply = Set(opt_text(&form.place_of_supply));
            am.default_bank_account = Set(str_to_opt_string(&form.default_bank_account));
            am.company_logo_vnode_id = Set(str_to_opt_i64(&form.company_logo_vnode_id));
            am.company_signature_vnode_id = Set(str_to_opt_i64(&form.company_signature_vnode_id));
            am.update(db).await.map_err(|err| err.to_string())?;
        } else {
            preferences::ActiveModel {
                id: Set(1),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                quotation_number_format: Set(number_format),
                quotation_pdf_template: Set(str_to_opt_string(&form.quotation_pdf_template)),
                quotation_email_subject: Set(opt_text(&form.quotation_email_subject)),
                quotation_email_body: Set(opt_text(&form.quotation_email_body)),
                company_name: Set(opt_text(&form.company_name)),
                company_address: Set(str_to_opt_string(&form.company_address)),
                company_phone: Set(opt_text(&form.company_phone)),
                company_gstin: Set(opt_text(&form.company_gstin)),
                place_of_supply: Set(opt_text(&form.place_of_supply)),
                default_bank_account: Set(str_to_opt_string(&form.default_bank_account)),
                company_logo_vnode_id: Set(str_to_opt_i64(&form.company_logo_vnode_id)),
                company_signature_vnode_id: Set(str_to_opt_i64(&form.company_signature_vnode_id)),
            }
            .insert(db)
            .await
            .map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}

pub(crate) static PREFERENCES_ADDON: QuotationsAccountingPreferencesAddon =
    QuotationsAccountingPreferencesAddon;
