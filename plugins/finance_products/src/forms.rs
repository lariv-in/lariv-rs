use lariv_core::html_form::{
    html_form,
    widgets::{Number, Select, Text, Textarea},
};
use lariv_formula::variable_schema_input::VariableSchemaList;

use lariv_plugin_finance_accounts::routes::AccountSelectRouteTag;
use lariv_plugin_finance_taxes::routes::TaxMultiSelectRouteTag;

use crate::entities::product::{PRODUCT_TYPE_BOTH, PRODUCT_TYPE_GOODS, PRODUCT_TYPE_SERVICES};

#[html_form]
pub struct ProductForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Type", required, widget = Select)]
    pub product_type: String,

    #[form(label = "Reference", widget = Text)]
    pub reference: String,

    #[form(label = "Remarks", widget = Textarea, rows = 4)]
    pub remarks: String,

    #[form(
        label = "Variables",
        widget = VariableSchemaList,
        placeholder = "Variable name",
        hint = "Inputs the price formulas can use, for one product. Invoice quantity multiplies the result."
    )]
    pub variables: Vec<String>,

    #[form(
        label = "Base price of one product",
        required,
        widget = Textarea,
        rows = 3,
        hint = "Rune expression for the cost of one product. Invoice quantity multiplies it. Example: 40 or length * decimal(\"2\")."
    )]
    pub base_price_formula: String,

    #[form(
        label = "Sales price of one product",
        required,
        widget = Textarea,
        rows = 3,
        hint = "Rune expression for the sales price of one product before tax. Invoice quantity multiplies it. Example: 100 or length * 85."
    )]
    pub sales_price_formula: String,

    #[form(label = "HSN code", required, widget = Number)]
    pub hsn_code: i64,

    #[form(
        label = "Taxes",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "product-taxes",
        placeholder = "Select taxes…"
    )]
    pub tax_ids: Vec<i64>,
}

impl ProductForm {
    pub fn product_type_choices() -> &'static [(&'static str, &'static str)] {
        &[
            (PRODUCT_TYPE_GOODS, "Goods"),
            (PRODUCT_TYPE_SERVICES, "Services"),
            (PRODUCT_TYPE_BOTH, "Both"),
        ]
    }
}

#[html_form]
pub struct ProductFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,

    #[form(label = "Reference", widget = Text)]
    pub reference: String,
}

#[html_form]
pub struct ProductPreferencesForm {
    #[form(
        label = "Inventory account (products)",
        widget = ForeignKey,
        route = AccountSelectRouteTag,
        swap_key = "pref-product-inventory",
        display = "inventory_account",
        placeholder = "Select…"
    )]
    pub inventory_account_id: String,

    #[form(
        label = "Cost of sales account (products)",
        name = "CostOfSalesAcctID",
        widget = ForeignKey,
        route = AccountSelectRouteTag,
        swap_key = "pref-product-cos",
        display = "cost_of_sales_account",
        placeholder = "Select…"
    )]
    pub cost_of_sales_account_id: String,
}
