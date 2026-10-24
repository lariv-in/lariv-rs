use lariv_core::html_form::{
    FieldRender, FormCtx, FormWidget, html_form,
    widgets::{Select, Text},
};
use maud::Markup;

use lariv_plugin_finance_accounts::routes::JournalSelectRouteTag;

use crate::entities::TaxKind;
use crate::journal_template::{FORMULA_VARIABLES_HINT, empty_lines_json};
use crate::journal_template_editor::input_journal_template_lines;

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

pub fn tax_type_choices() -> Vec<(String, String)> {
    vec![
        (
            TaxKind::Levied.as_str().into(),
            TaxKind::Levied.label().into(),
        ),
        (
            TaxKind::Withholding.as_str().into(),
            TaxKind::Withholding.label().into(),
        ),
    ]
}

pub fn tax_type_label(kind: &TaxKind) -> String {
    kind.label().to_string()
}

#[html_form]
pub struct TaxForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Type", required, widget = Select, choices = "tax_type")]
    pub tax_type: String,

    #[form(label = "Percentage", required, widget = Text)]
    pub percentage: String,

    #[form(
        label = "Journal",
        widget = ForeignKey,
        route = JournalSelectRouteTag,
        swap_key = "tax-journal",
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};

    #[test]
    fn journal_template_label_lists_injected_variables() {
        let html =
            TaxForm::render_inputs(&FormCtx::form::<TaxForm>(CsrfToken::current())).into_string();
        assert!(html.contains("Journal entry template"));
        assert!(html.contains("percentage — percentage stored on this tax"));
        assert!(html.contains("base — amount this tax is applied to"));
        assert!(html.contains("hintOpen"));
    }
}
