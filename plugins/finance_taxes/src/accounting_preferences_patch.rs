//! Default tax journal template on `/finance/preferences`.

use lariv_plugin_finance_accounts::accounting_preferences_patch::{
    AccountingPreferenceGroup, AccountingPreferencesAddon,
};
use maud::html;
use sea_orm::DatabaseConnection;

use crate::forms::{TaxJournalPreferencesForm, TaxJournalPreferencesFormField};

pub(crate) struct TaxesAccountingPreferencesAddon;

#[async_trait::async_trait]
impl AccountingPreferencesAddon for TaxesAccountingPreferencesAddon {
    fn id(&self) -> &'static str {
        "finance-taxes"
    }

    async fn render_groups(&self, db: &DatabaseConnection) -> Vec<AccountingPreferenceGroup> {
        use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};

        let fields = crate::journal_template::preference_form_fields(db, "tax-pref-line").await;
        let inputs = html! {
            (TaxJournalPreferencesForm::render_inputs(
                &FormCtx::form::<TaxJournalPreferencesForm>(CsrfToken::current())
                    .value(TaxJournalPreferencesFormField::JournalId, &fields.journal_id)
                    .display(TaxJournalPreferencesFormField::JournalId, &fields.journal_display)
                    .value(
                        TaxJournalPreferencesFormField::JournalLines,
                        &fields.journal_lines,
                    ),
            ))
        };
        vec![AccountingPreferenceGroup {
            title: "Taxes",
            inputs,
        }]
    }

    async fn save_from_form(
        &self,
        db: &DatabaseConnection,
        post: &lariv_plugin_finance_accounts::accounting_preferences_patch::AccountingPreferencesPost,
    ) -> Result<(), String> {
        let form = post
            .deserialize::<TaxJournalPreferencesForm>()
            .map_err(|e| e.to_string())?;
        crate::journal_template::save_preference_template(db, &form.journal_id, &form.journal_lines)
            .await
    }
}

pub(crate) static TAXES_ADDON: TaxesAccountingPreferencesAddon = TaxesAccountingPreferencesAddon;
