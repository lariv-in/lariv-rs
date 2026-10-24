use lariv_core::html_form::{
    FieldRender, FormCtx, FormWidget, html_form,
    widgets::{Number, Text, Textarea},
};
use lariv_formula::variable_schema_input::VariableSchemaList;
use maud::Markup;

use lariv_plugin_finance_accounts::routes::JournalSelectRouteTag;
use lariv_plugin_finance_taxes::routes::TaxMultiSelectRouteTag;

use crate::purchase_services::journal_template::{FORMULA_VARIABLES_HINT, empty_lines_json};
use crate::purchase_services::journal_template_editor::input_journal_template_lines;

/// Account and formula lines stored as JSON.
pub struct JournalTemplateLines;

impl FormWidget for JournalTemplateLines {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let defaults = if field.value.trim().is_empty() {
            empty_lines_json()
        } else {
            field.value.to_string()
        };
        input_journal_template_lines(field.name, field.label, &defaults, ctx.hint_of(field.spec))
    }
}

#[html_form]
pub struct ServiceForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Reference", widget = Text)]
    pub reference: String,

    #[form(label = "Remarks", widget = Textarea, rows = 4)]
    pub remarks: String,

    #[form(
        label = "Variables",
        widget = VariableSchemaList,
        placeholder = "Variable name",
        hint = "Inputs the price and journal template formulas can use, for one service. Purchase quantity multiplies the result."
    )]
    pub variables: Vec<String>,

    #[form(
        label = "Base price of one service",
        required,
        widget = Textarea,
        rows = 3,
        hint = "Rune expression for the cost of one service. Purchase quantity multiplies it. Example: 40 or length * 2."
    )]
    pub base_price_formula: String,

    #[form(
        label = "Sales price of one service",
        required,
        widget = Textarea,
        rows = 3,
        hint = "Rune expression for the sales price of one service before tax. Purchase quantity multiplies it. Example: 100 or length * 85."
    )]
    pub sales_price_formula: String,

    #[form(
        label = "Journal",
        widget = ForeignKey,
        route = JournalSelectRouteTag,
        swap_key = "purchase-service-journal",
        display = "journal",
        placeholder = "Select journal…"
    )]
    pub journal_id: String,

    #[form(
        label = "Journal entry template",
        widget = JournalTemplateLines,
        hint = FORMULA_VARIABLES_HINT
    )]
    pub journal_lines: String,

    #[form(label = "SAC code", required, widget = Number)]
    pub sac_code: i64,

    #[form(
        label = "Taxes",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "purchase-service-taxes",
        placeholder = "Select taxes…"
    )]
    pub tax_ids: Vec<i64>,
}

#[html_form]
pub struct ServiceJournalPreferencesForm {
    #[form(
        label = "Journal",
        name = "PurchaseServiceJournalId",
        widget = ForeignKey,
        route = JournalSelectRouteTag,
        swap_key = "pref-purchase-service-journal",
        display = "journal",
        placeholder = "Select journal…"
    )]
    pub journal_id: String,

    #[form(
        label = "Journal entry template",
        name = "PurchaseServiceJournalLines",
        widget = JournalTemplateLines,
        hint = FORMULA_VARIABLES_HINT
    )]
    pub journal_lines: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};

    #[test]
    fn journal_template_label_lists_injected_variables() {
        let html = ServiceForm::render_inputs(&FormCtx::form::<ServiceForm>(CsrfToken::current()))
            .into_string();
        assert!(html.contains("Journal entry template"));
        assert!(html.contains("quantity — purchase line quantity"));
        assert!(html.contains("base_cost — base price of one service"));
        assert!(html.contains("sales_price — sales price of one service"));
        assert!(html.contains("hintOpen"));
    }
}
