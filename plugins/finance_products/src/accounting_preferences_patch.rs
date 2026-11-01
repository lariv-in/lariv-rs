//! Default product journal template on `/finance/preferences`.

use lariv_plugin_finance_accounts::accounting_preferences_patch::{
    AccountingPreferenceGroup, AccountingPreferencesAddon,
};
use maud::html;
use sea_orm::DatabaseConnection;

use crate::forms::{ProductJournalPreferencesForm, ProductJournalPreferencesFormField};

pub(crate) struct ProductsAccountingPreferencesAddon;

#[async_trait::async_trait]
impl AccountingPreferencesAddon for ProductsAccountingPreferencesAddon {
    fn id(&self) -> &'static str {
        "finance-products"
    }

    async fn render_groups(&self, db: &DatabaseConnection) -> Vec<AccountingPreferenceGroup> {
        use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};

        let defaults =
            crate::journal_template::preference_form_fields(db, "product-pref-line").await;
        let inputs = html! {
            (ProductJournalPreferencesForm::render_inputs(
                &FormCtx::form::<ProductJournalPreferencesForm>(CsrfToken::current())
                    .value(ProductJournalPreferencesFormField::JournalId, &defaults.journal_id)
                    .display(
                        ProductJournalPreferencesFormField::JournalId,
                        &defaults.journal_display,
                    )
                    .value(
                        ProductJournalPreferencesFormField::JournalLines,
                        &defaults.journal_lines,
                    ),
            ))
        };
        vec![AccountingPreferenceGroup {
            title: "Products",
            inputs,
        }]
    }

    async fn save_from_form(
        &self,
        db: &DatabaseConnection,
        post: &lariv_plugin_finance_accounts::accounting_preferences_patch::AccountingPreferencesPost,
    ) -> Result<(), String> {
        let form = post
            .deserialize::<ProductJournalPreferencesForm>()
            .map_err(|e| e.to_string())?;
        crate::journal_template::save_preference_template(db, &form.journal_id, &form.journal_lines)
            .await
    }
}

pub(crate) static PRODUCTS_ADDON: ProductsAccountingPreferencesAddon =
    ProductsAccountingPreferencesAddon;
