use lariv_core::components::attrs::escape_attr;
use lariv_core::html_form::{
    FieldRender, FormCtx, FormWidget, html_form,
    widgets::{Datetime, Text},
};
use lariv_plugin_finance_accounts::journal_lines_editor::input_journal_lines;
use maud::{Markup, PreEscaped, html};

use lariv_plugin_crm::routes::CustomerFkSelectRouteTag;
use lariv_plugin_finance_accounts::routes::JournalSelectRouteTag;
use lariv_plugin_finance_taxes::journal_template_editor::input_journal_template_lines;
use lariv_plugin_finance_taxes::routes::TaxMultiSelectRouteTag;

use crate::payment_journal_template::{FORMULA_VARIABLES_HINT, empty_lines_json};
use crate::routes::PaymentJournalLinesRouteTag;

/// Account and formula lines stored as JSON.
pub struct PaymentJournalTemplateLines;

impl FormWidget for PaymentJournalTemplateLines {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let defaults = if field.value.trim().is_empty() {
            empty_lines_json()
        } else {
            field.value.to_string()
        };
        input_journal_template_lines(field.name, field.label, &defaults, ctx.hint_of(field.spec))
    }
}

/// Debit and credit lines for one payment. Prefills from the payment template.
pub struct PaymentJournalLines;

impl FormWidget for PaymentJournalLines {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let url = PaymentJournalLinesRouteTag.path();
        html! {
            div id="payment-journal-lines" {
                (input_journal_lines(field.name, field.label, field.value))
            }
            (PreEscaped(format!(
                r##"<button type="button" class="btn btn-outline btn-sm" hx-post="{}" hx-target="#payment-journal-lines" hx-swap="innerHTML" hx-include="closest form">Fill from template</button>"##,
                escape_attr(&url),
            )))
            p class="text-sm text-base-content/70 mt-2" {
                "Saved lines are whatever is in this editor. Fill from template replaces them from the payment preferences for the amount and withholding taxes above."
            }
        }
    }
}

#[html_form]
pub struct PaymentForm {
    #[form(
        label = "Customer",
        required,
        widget = ForeignKey,
        route = CustomerFkSelectRouteTag,
        swap_key = "fk-payment-customer",
        display = "customer",
        placeholder = "Select customer…"
    )]
    pub customer_id: i64,

    #[form(label = "Amount", required, widget = Text)]
    pub amount: String,

    #[form(label = "Payment date & time", required, widget = Datetime)]
    pub datetime: String,

    #[form(
        label = "Withholding taxes",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "payment-withholding-taxes",
        placeholder = "Optional withholding at collection…"
    )]
    pub taxes: Vec<i64>,

    #[form(label = "Journal lines", required, widget = PaymentJournalLines)]
    pub journal_lines: String,
}

#[html_form]
pub struct PaymentPreferencesForm {
    #[form(
        label = "Journal",
        widget = ForeignKey,
        route = JournalSelectRouteTag,
        swap_key = "pref-payment-journal",
        display = "journal",
        placeholder = "Select journal…"
    )]
    pub journal_id: String,

    #[form(
        label = "Journal entry template",
        widget = PaymentJournalTemplateLines,
        hint = FORMULA_VARIABLES_HINT
    )]
    pub journal_lines: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};

    #[test]
    fn payment_template_label_lists_injected_variables() {
        let html = PaymentPreferencesForm::render_inputs(&FormCtx::form::<PaymentPreferencesForm>(
            CsrfToken::current(),
        ))
        .into_string();
        assert!(html.contains("Journal entry template"));
        assert!(html.contains("amount — settlement on this payment"));
        assert!(html.contains("withholding — withheld tax on this payment"));
        assert!(html.contains("hintOpen"));
    }

    #[test]
    fn payment_form_offers_fill_from_template() {
        let html = PaymentForm::render_inputs(&FormCtx::form::<PaymentForm>(CsrfToken::current()))
            .into_string();
        assert!(html.contains("id=\"payment-journal-lines\""));
        assert!(html.contains("Fill from template"));
        assert!(html.contains("/finance-payments/payment-journal-lines"));
        assert!(html.contains("Customer"));
    }
}
