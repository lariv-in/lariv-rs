//! Default service journal template on `/finance/preferences`.

use lariv_plugin_finance_accounts::accounting_preferences_patch::{
    AccountingPreferenceGroup, AccountingPreferencesAddon,
};
use maud::html;
use sea_orm::DatabaseConnection;

use crate::forms::{ServiceJournalPreferencesForm, ServiceJournalPreferencesFormField};

pub(crate) struct ServicesAccountingPreferencesAddon;

#[async_trait::async_trait]
impl AccountingPreferencesAddon for ServicesAccountingPreferencesAddon {
    fn id(&self) -> &'static str {
        "finance-services"
    }

    async fn render_groups(&self, db: &DatabaseConnection) -> Vec<AccountingPreferenceGroup> {
        use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};

        let fields = crate::journal_template::preference_form_fields(db, "service-pref-line").await;
        let inputs = html! {
            (ServiceJournalPreferencesForm::render_inputs(
                &FormCtx::form::<ServiceJournalPreferencesForm>(CsrfToken::current())
                    .value(ServiceJournalPreferencesFormField::JournalId, &fields.journal_id)
                    .display(
                        ServiceJournalPreferencesFormField::JournalId,
                        &fields.journal_display,
                    )
                    .value(
                        ServiceJournalPreferencesFormField::JournalLines,
                        &fields.journal_lines,
                    ),
            ))
        };
        vec![AccountingPreferenceGroup {
            title: "Services",
            inputs,
        }]
    }

    async fn save_from_form(
        &self,
        db: &DatabaseConnection,
        post: &lariv_plugin_finance_accounts::accounting_preferences_patch::AccountingPreferencesPost,
    ) -> Result<(), String> {
        let form = post
            .deserialize::<ServiceJournalPreferencesForm>()
            .map_err(|e| e.to_string())?;
        crate::journal_template::save_preference_template(db, &form.journal_id, &form.journal_lines)
            .await
    }
}

pub(crate) static SERVICES_ADDON: ServicesAccountingPreferencesAddon =
    ServicesAccountingPreferencesAddon;
