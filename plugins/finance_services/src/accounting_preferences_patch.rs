//! Patches service GL preferences onto `/finance/preferences`.

use crate::preferences::load_service_preferences;
use chrono::Utc;
use lariv_plugin_finance_accounts::{
    accounting_preferences_patch::{
        AccountingPreferenceGroup, AccountingPreferencesAddon, opt_i64_form_value, str_to_opt_i64,
    },
    scope::load_account_parent_label,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection};

use crate::{
    entities::service_preferences::{self},
    forms::ServicePreferencesForm,
};

fn fk_value(id: Option<i64>) -> String {
    opt_i64_form_value(id)
}

pub(crate) struct ServicesAccountingPreferencesAddon;

#[async_trait::async_trait]
impl AccountingPreferencesAddon for ServicesAccountingPreferencesAddon {
    fn id(&self) -> &'static str {
        "finance-services"
    }

    async fn render_groups(&self, db: &DatabaseConnection) -> Vec<AccountingPreferenceGroup> {
        use crate::forms::ServicePreferencesFormField;
        use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
        use maud::html;

        let prefs = load_service_preferences(db).await;
        let credit_display = load_account_parent_label(db, prefs.cost_credit_account_id).await;
        let cos_display = load_account_parent_label(db, prefs.cost_of_sales_account_id).await;

        vec![AccountingPreferenceGroup {
            title: "Services",
            inputs: html! {
                (ServicePreferencesForm::render_inputs(
                    &FormCtx::form::<ServicePreferencesForm>(CsrfToken::current())
                        .value(
                            ServicePreferencesFormField::CostCreditAccountId,
                            fk_value(prefs.cost_credit_account_id),
                        )
                        .display(
                            ServicePreferencesFormField::CostCreditAccountId,
                            &credit_display,
                        )
                        .value(
                            ServicePreferencesFormField::CostOfSalesAccountId,
                            fk_value(prefs.cost_of_sales_account_id),
                        )
                        .display(
                            ServicePreferencesFormField::CostOfSalesAccountId,
                            &cos_display,
                        ),
                ))
            },
        }]
    }

    async fn save_from_form(
        &self,
        db: &DatabaseConnection,
        post: &lariv_plugin_finance_accounts::accounting_preferences_patch::AccountingPreferencesPost,
    ) -> Result<(), String> {
        let form = post
            .deserialize::<ServicePreferencesForm>()
            .map_err(|e| e.to_string())?;

        let prefs = load_service_preferences(db).await;
        let now = Utc::now();
        let mut am: service_preferences::ActiveModel = prefs.into();
        am.cost_credit_account_id = Set(str_to_opt_i64(&form.cost_credit_account_id));
        am.cost_of_sales_account_id = Set(str_to_opt_i64(&form.cost_of_sales_account_id));
        am.updated_at = Set(Some(now));
        am.update(db).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub(crate) static SERVICES_ADDON: ServicesAccountingPreferencesAddon =
    ServicesAccountingPreferencesAddon;
