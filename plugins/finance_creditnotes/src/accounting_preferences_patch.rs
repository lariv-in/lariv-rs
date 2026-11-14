//! Patches the credit note number format onto `/finance/preferences`.

use chrono::Utc;
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_plugin_finance_accounts::accounting_preferences_patch::{
    AccountingPreferenceGroup, AccountingPreferencesAddon, str_to_opt_string,
};
use maud::html;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection};

use crate::{
    entities::credit_note_preferences,
    forms::{CreditNotePreferencesForm, CreditNotePreferencesFormField},
    logic::preferences::load_credit_note_preferences,
};

pub(crate) struct CreditNotesAccountingPreferencesAddon;

#[async_trait::async_trait]
impl AccountingPreferencesAddon for CreditNotesAccountingPreferencesAddon {
    fn id(&self) -> &'static str {
        "finance-creditnotes"
    }

    async fn render_groups(&self, db: &DatabaseConnection) -> Vec<AccountingPreferenceGroup> {
        let prefs = load_credit_note_preferences(db).await;
        let number_format = prefs.credit_note_number_format.unwrap_or_default();
        let inputs = html! {
            (CreditNotePreferencesForm::render_inputs(
                &FormCtx::form::<CreditNotePreferencesForm>(CsrfToken::current())
                    .value(
                        CreditNotePreferencesFormField::CreditNoteNumberFormat,
                        &number_format,
                    ),
            ))
        };
        vec![AccountingPreferenceGroup {
            title: "Credit notes",
            inputs,
        }]
    }

    async fn save_from_form(
        &self,
        db: &DatabaseConnection,
        post: &lariv_plugin_finance_accounts::accounting_preferences_patch::AccountingPreferencesPost,
    ) -> Result<(), String> {
        let form = post
            .deserialize::<CreditNotePreferencesForm>()
            .map_err(|e| e.to_string())?;
        let prefs = load_credit_note_preferences(db).await;
        let mut am: credit_note_preferences::ActiveModel = prefs.into();
        am.credit_note_number_format = Set(str_to_opt_string(&form.credit_note_number_format));
        am.updated_at = Set(Some(Utc::now()));
        am.update(db).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub(crate) static CREDIT_NOTES_ADDON: CreditNotesAccountingPreferencesAddon =
    CreditNotesAccountingPreferencesAddon;
