use crate::html_form::{
    html_form,
    widgets::{Number, Select, Text, Textarea},
};

use crate::plugins::finance_accounts::routes::AccountSelectRouteTag;
use crate::plugins::finance_taxes::routes::TaxMultiSelectRouteTag;

use crate::plugins::finance_products::entities::product::{
    PRODUCT_TYPE_BOTH, PRODUCT_TYPE_GOODS, PRODUCT_TYPE_SERVICES,
};

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

    #[form(label = "Base cost", required, widget = Text)]
    pub base_cost: String,

    #[form(label = "Sales price", required, widget = Text)]
    pub sales_price: String,

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
