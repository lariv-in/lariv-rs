use lariv_core::components::attrs::escape_attr;
use lariv_core::html_form::{
    FieldRender, FormCtx, FormFieldKey, FormWidget, html_form,
    widgets::{Checkbox, Number, Select, Text, Textarea},
};
use lariv_formula::variable_schema_input::VariableSchemaList;
use maud::{Markup, PreEscaped, html};

use crate::journal_lines_editor::input_journal_lines;
use crate::journal_template::{FORMULA_VARIABLES_HINT, empty_lines_json};
use crate::journal_template_editor::input_journal_template_lines;
use crate::routes::{
    AccountSelectRouteTag, CurrencySelectRouteTag, CustomSourceDocFillJournalLinesRouteTag,
    CustomSourceDocTemplateSelectRouteTag, JournalSelectRouteTag, SourceDocSelectRouteTag,
};
use crate::variable_values_editor::{empty_variable_values_fields, input_variable_values_fields};
use lariv_plugin_filesystem::routes::VNodeSelectRouteTag;

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

/// Account, debit, and credit lines stored as JSON.
pub struct JournalLines;

impl FormWidget for JournalLines {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_journal_lines(field.name, field.label, field.value)
    }
}

/// Journal lines wrapped for HTMX fill-from-template replacement.
pub struct CreateTransactionJournalLines;

impl FormWidget for CreateTransactionJournalLines {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        html! {
            div id="transaction-create-journal-lines" {
                (input_journal_lines(field.name, field.label, field.value))
            }
        }
    }
}

/// Slot for template variable value inputs, with the fill-lines control below.
pub struct TemplateVariableValuesSlot;

impl FormWidget for TemplateVariableValuesSlot {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let template_id = ctx.value_of("TemplateID").trim();
        let variables = if template_id.is_empty() {
            empty_variable_values_fields()
        } else {
            let schema_json = ctx.hint_of(field.spec).unwrap_or("{}");
            let values = if field.value.trim().is_empty() {
                "{}"
            } else {
                field.value
            };
            input_variable_values_fields(schema_json, values)
        };
        let fill_url = CustomSourceDocFillJournalLinesRouteTag.path();
        html! {
            (variables)
            (PreEscaped(format!(
                r##"<button type="button" class="btn btn-outline btn-sm mt-2" id="transaction-fill-journal-lines" hx-post="{url}" hx-target="#transaction-create-journal-lines" hx-swap="innerHTML" hx-include="closest form" disabled x-bind:disabled="!canFill">Fill journal lines from template</button>"##,
                url = escape_attr(&fill_url),
            )))
            p class="text-sm text-base-content/70 mt-1" {
                "Select a transaction template and enter variable values, then fill the lines. You can still edit the lines before saving."
            }
        }
    }
}

pub fn balance_type_choices() -> Vec<(String, String)> {
    vec![
        ("Credit".into(), "Credit".into()),
        ("Debit".into(), "Debit".into()),
    ]
}

#[html_form]
pub struct AccountForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Code", required, widget = Number)]
    pub code: String,

    #[form(label = "Group account (summary)", widget = Checkbox)]
    pub is_group: String,

    #[form(
        label = "Balance type",
        required,
        widget = Select,
        choices = "balance_type",
        model = "balanceType"
    )]
    pub balance_type: String,

    #[form(
        label = "Parent account",
        widget = ForeignKey,
        route = AccountSelectRouteTag,
        display = "parent_display",
        placeholder = "Optional parent…"
    )]
    pub parent_id: String,

    #[form(
        label = "Sub-accounts",
        name = "ChildIDs",
        widget = ManyToMany,
        route = AccountSelectRouteTag,
        when = "edit_children",
        placeholder = "Select sub-accounts…"
    )]
    pub child_ids: Vec<i64>,
}

impl AccountForm {
    /// Alpine `x-data` so Balance type can track parent picker selection.
    pub fn balance_type_sync_x_data(balance_type: &str) -> String {
        let escaped = balance_type.replace('\\', "\\\\").replace('\'', "\\'");
        format!("{{ balanceType: '{escaped}' }}")
    }

    /// `@fk-select.window` handler: copy parent account balance_type into the select.
    pub fn balance_type_sync_fk_handler() -> String {
        format!(
            "if ($event.detail && $event.detail.name === '{}' && $event.detail.balance_type != null && String($event.detail.balance_type).trim() !== '') balanceType = String($event.detail.balance_type)",
            AccountFormField::ParentId.html_name()
        )
    }
}

#[html_form]
pub struct CurrencyForm {
    #[form(label = "ISO 4217 numeric code", required, widget = Number)]
    pub code: String,

    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Symbol", required, widget = Text)]
    pub symbol: String,

    #[form(label = "Minor unit (decimal places)", required, widget = Number)]
    pub minor_unit: String,
}

#[html_form]
pub struct JournalCreateForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Active", widget = Checkbox)]
    pub is_active: String,

    #[form(
        label = "Currency",
        required,
        widget = ForeignKey,
        route = CurrencySelectRouteTag,
        display = "currency_display",
        placeholder = "Select currency…"
    )]
    pub currency_id: String,
}

#[html_form]
pub struct JournalForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Active", widget = Checkbox, row = "flags")]
    pub is_active: String,

    #[form(label = "Mutable", widget = Checkbox, row = "flags")]
    pub is_mutable: String,

    #[form(
        label = "Currency",
        required,
        widget = ForeignKey,
        route = CurrencySelectRouteTag,
        display = "currency_display",
        placeholder = "Select currency…"
    )]
    pub currency_id: String,
}

#[html_form]
pub struct JournalEntryForm {
    #[form(
        label = "Source document",
        required,
        widget = ForeignKey,
        route = SourceDocSelectRouteTag,
        display = "source_doc_display",
        placeholder = "Select source document…"
    )]
    pub source_doc_id: String,
}

#[html_form]
pub struct JournalEntryLinesForm {
    #[form(label = "Lines", required, widget = JournalLines)]
    pub lines: String,
}

#[html_form]
pub struct CustomSourceDocForm {
    #[form(label = "Title", required, widget = Text)]
    pub title: String,

    #[form(label = "Content", widget = Textarea, rows = 6)]
    pub content: String,

    #[form(
        label = "Attached file",
        widget = ForeignKey,
        route = VNodeSelectRouteTag,
        swap_key = "fk-custom-source-doc-attached-vnode",
        display = "attached_vnode",
        placeholder = "Optional file or folder…"
    )]
    pub attached_vnode_id: String,

    #[form(label = "Lines", required, widget = JournalLines)]
    pub lines: String,
}

/// Create form for the accounting sidebar list, where the journal is chosen here.
#[html_form]
pub struct CustomSourceDocIndexCreateForm {
    #[form(
        label = "Journal",
        required,
        widget = ForeignKey,
        route = JournalSelectRouteTag,
        display = "journal_display",
        placeholder = "Select journal…"
    )]
    pub journal_id: String,

    #[form(
        label = "Transaction template",
        widget = ForeignKey,
        route = CustomSourceDocTemplateSelectRouteTag,
        display = "template_display",
        placeholder = "Optional template…"
    )]
    pub template_id: String,

    #[form(label = "Title", required, widget = Text)]
    pub title: String,

    #[form(label = "Content", widget = Textarea, rows = 6)]
    pub content: String,

    #[form(
        label = "Attached file",
        widget = ForeignKey,
        route = VNodeSelectRouteTag,
        swap_key = "fk-custom-source-doc-create-attached-vnode",
        display = "attached_vnode",
        placeholder = "Optional file or folder…"
    )]
    pub attached_vnode_id: String,

    #[form(label = "", widget = TemplateVariableValuesSlot)]
    pub variable_values: String,

    #[form(label = "Lines", required, widget = CreateTransactionJournalLines)]
    pub lines: String,
}

#[html_form]
pub struct AccountingPreferencesForm {
    #[form(
        label = "Default currency",
        widget = ForeignKey,
        route = CurrencySelectRouteTag,
        display = "default_currency_display",
        placeholder = "Select currency…"
    )]
    pub default_currency_id: Option<i64>,
}

#[html_form]
pub struct CustomSourceDocTemplateForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Remarks", widget = Textarea, rows = 4)]
    pub remarks: String,

    #[form(
        label = "Attached file",
        widget = ForeignKey,
        route = VNodeSelectRouteTag,
        swap_key = "fk-custom-source-doc-template-attached-vnode",
        display = "attached_vnode",
        placeholder = "Optional file or folder…"
    )]
    pub attached_vnode_id: String,

    #[form(
        label = "Variables",
        widget = VariableSchemaList,
        placeholder = "Variable name",
        hint = "Inputs journal template formulas can use. Choose a type for each variable."
    )]
    pub variables: Vec<String>,

    #[form(
        label = "Journal entry template",
        widget = JournalTemplateLines,
        hint = FORMULA_VARIABLES_HINT
    )]
    pub journal_lines: String,
}
