use crate::html_form::{
    html_form,
    widgets::{Select, Text},
};

use crate::plugins::finance_accounts::routes::AccountSelectRouteTag;

use crate::plugins::finance_taxes::entities::TaxKind;

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

pub fn tax_type_filter_choices() -> Vec<(String, String)> {
    let mut v = vec![("".into(), "Any".into())];
    v.extend(tax_type_choices());
    v
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
        label = "Account",
        widget = ForeignKey,
        route = AccountSelectRouteTag,
        swap_key = "fk-tax-account",
        display = "account",
        placeholder = "Select…"
    )]
    pub account_id: String,
}

#[html_form]
pub struct TaxFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,

    #[form(label = "Type", widget = Select, choices = "tax_type")]
    pub tax_type: String,
}
