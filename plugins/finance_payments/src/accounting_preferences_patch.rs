//! Patches the payment journal template onto `/finance/preferences`.

use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_plugin_finance_accounts::{
    accounting_preferences_patch::{AccountingPreferenceGroup, AccountingPreferencesAddon},
    scope::{load_account_parent_label, load_default_journal_form, load_journal_display_label},
};
use maud::html;
use sea_orm::DatabaseConnection;

use crate::{
    forms::{PaymentPreferencesForm, PaymentPreferencesFormField},
    logic::preferences::load_payment_preferences,
};

async fn payment_template_form(db: &DatabaseConnection) -> (String, String, String) {
    let Ok(Some(template)) = crate::payment_journal_template::load_template(db).await else {
        let (journal_id, journal_display) = load_default_journal_form(db).await;
        return (
            journal_id,
            journal_display,
            crate::payment_journal_template::empty_lines_json(),
        );
    };
    let mut labels = Vec::with_capacity(template.items.len());
    for item in &template.items {
        let label = load_account_parent_label(db, Some(item.account_id)).await;
        labels.push(if label == "—" { String::new() } else { label });
    }
    let journal_display = load_journal_display_label(db, Some(template.journal_id)).await;
    (
        template.journal_id.to_string(),
        if journal_display == "—" {
            String::new()
        } else {
            journal_display
        },
        crate::payment_journal_template::lines_to_json(&template.items, &labels),
    )
}

pub(crate) struct PaymentsAccountingPreferencesAddon;

#[async_trait::async_trait]
impl AccountingPreferencesAddon for PaymentsAccountingPreferencesAddon {
    fn id(&self) -> &'static str {
        "finance-payments"
    }

    async fn render_groups(&self, db: &DatabaseConnection) -> Vec<AccountingPreferenceGroup> {
        let _pay = load_payment_preferences(db).await;
        let (journal_id, journal_display, journal_lines) = payment_template_form(db).await;
        let payments = html! {
            (PaymentPreferencesForm::render_inputs(
                &FormCtx::form::<PaymentPreferencesForm>(CsrfToken::current())
                    .value(PaymentPreferencesFormField::JournalId, &journal_id)
                    .display(PaymentPreferencesFormField::JournalId, &journal_display)
                    .value(PaymentPreferencesFormField::JournalLines, &journal_lines),
            ))
        };
        vec![AccountingPreferenceGroup {
            title: "Payments",
            inputs: payments,
        }]
    }

    async fn save_from_form(
        &self,
        db: &DatabaseConnection,
        post: &lariv_plugin_finance_accounts::accounting_preferences_patch::AccountingPreferencesPost,
    ) -> Result<(), String> {
        let payment = post
            .deserialize::<PaymentPreferencesForm>()
            .map_err(|e| e.to_string())?;
        let _pay_prefs = load_payment_preferences(db).await;
        crate::payment_journal_template::save_payment_template(
            db,
            &payment.journal_id,
            &payment.journal_lines,
        )
        .await
    }
}

pub(crate) static PAYMENTS_ADDON: PaymentsAccountingPreferencesAddon =
    PaymentsAccountingPreferencesAddon;
