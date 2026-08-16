use crate::html_form::{
    FormFieldKey, html_form,
    widgets::{Checkbox, Number, Select, Text},
};

use crate::plugins::finance_accounts::routes::{
    AccountSelectRouteTag, CurrencySelectRouteTag, SourceDocSelectRouteTag,
};

pub fn balance_type_choices() -> Vec<(String, String)> {
    vec![
        ("Credit".into(), "Credit".into()),
        ("Debit".into(), "Debit".into()),
    ]
}

pub fn balance_type_filter_choices() -> Vec<(String, String)> {
    let mut v = vec![("".into(), "Any".into())];
    v.extend(balance_type_choices());
    v
}

pub fn journal_type_choices() -> Vec<(String, String)> {
    vec![
        ("Credit".into(), "Credit".into()),
        ("Debit".into(), "Debit".into()),
    ]
}

pub fn journal_type_filter_choices() -> Vec<(String, String)> {
    let mut v = vec![("".into(), "Any".into())];
    v.extend(journal_type_choices());
    v
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
pub struct AccountFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,

    #[form(label = "Code", widget = Text)]
    pub code: String,

    #[form(label = "Group account", widget = Checkbox)]
    pub is_group: String,

    #[form(label = "Balance type", widget = Select, choices = "balance_type")]
    pub balance_type: String,
}

#[html_form]
pub struct AccountSelectionFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,

    #[form(label = "Code", widget = Text)]
    pub code: String,

    #[form(label = "Balance type", widget = Select, choices = "balance_type")]
    pub balance_type: String,

    #[form(label = "Parent ID", widget = Text)]
    pub parent_id: String,
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
pub struct CurrencyFilterForm {
    #[form(label = "Numeric code", widget = Text)]
    pub code: String,

    #[form(label = "Name", widget = Text)]
    pub name: String,

    #[form(label = "Symbol", widget = Text)]
    pub symbol: String,

    #[form(label = "Minor unit", widget = Text)]
    pub minor_unit: String,
}

#[html_form]
pub struct CurrencySelectionFilterForm {
    #[form(label = "Numeric code", widget = Text)]
    pub code: String,

    #[form(label = "Name", widget = Text)]
    pub name: String,

    #[form(label = "Symbol", widget = Text)]
    pub symbol: String,
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

    #[form(label = "Type", required, widget = Select, choices = "journal_type")]
    pub journal_type: String,
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

    #[form(label = "Type", required, widget = Select, choices = "journal_type")]
    pub journal_type: String,
}

#[html_form]
pub struct JournalFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,

    #[form(label = "Active", widget = Checkbox)]
    pub is_active: String,

    #[form(label = "Currency ID", widget = Text)]
    pub currency_id: String,

    #[form(label = "Type", widget = Select, choices = "journal_type")]
    pub journal_type: String,
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
