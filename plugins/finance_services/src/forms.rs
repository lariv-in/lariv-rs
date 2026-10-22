use lariv_core::html_form::{
    html_form,
    widgets::{Number, Text, Textarea},
};
use lariv_formula::variable_schema_input::VariableSchemaList;

use lariv_plugin_finance_accounts::routes::AccountSelectRouteTag;
use lariv_plugin_finance_taxes::routes::TaxMultiSelectRouteTag;

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
        hint = "Inputs the price formulas can use, for one service. Invoice quantity multiplies the result."
    )]
    pub variables: Vec<String>,

    #[form(
        label = "Base price of one service",
        required,
        widget = Textarea,
        rows = 3,
        hint = "Rune expression for the cost of one service. Invoice quantity multiplies it. Example: 40 or length * decimal(\"2\")."
    )]
    pub base_price_formula: String,

    #[form(
        label = "Sales price of one service",
        required,
        widget = Textarea,
        rows = 3,
        hint = "Rune expression for the sales price of one service before tax. Invoice quantity multiplies it. Example: 100 or length * 85."
    )]
    pub sales_price_formula: String,

    #[form(label = "SAC code", required, widget = Number)]
    pub sac_code: i64,

    #[form(
        label = "Taxes",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "service-taxes",
        placeholder = "Select taxes…"
    )]
    pub tax_ids: Vec<i64>,
}

#[html_form]
pub struct ServicePreferencesForm {
    #[form(
        label = "Cost credit account",
        widget = ForeignKey,
        route = AccountSelectRouteTag,
        swap_key = "pref-service-cost-credit",
        display = "cost_credit_account",
        placeholder = "Select…",
        hint = "Credited for the service cost. This is the role the inventory account plays for products."
    )]
    pub cost_credit_account_id: String,

    #[form(
        label = "Cost of sales account",
        name = "CostOfSalesAcctID",
        widget = ForeignKey,
        route = AccountSelectRouteTag,
        swap_key = "pref-service-cos",
        display = "cost_of_sales_account",
        placeholder = "Select…"
    )]
    pub cost_of_sales_account_id: String,
}
