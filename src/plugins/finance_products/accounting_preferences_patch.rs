//! Patches product GL preferences onto `/finance/preferences`.

use crate::plugins::finance_accounts::{
    accounting_preferences_patch::{AccountingPreferencesAddon, str_to_opt_i64},
    scope::load_account_parent_label,
};
use crate::plugins::finance_products::preferences::{load_product_preferences, optional_i64};
use chrono::Utc;
use maud::Markup;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection};

use crate::plugins::finance_products::{
    entities::product_preferences::{self},
    forms::ProductPreferencesForm,
};

fn fk_value(id: Option<i64>) -> String {
    optional_i64(id).to_string()
}

pub(crate) struct ProductsAccountingPreferencesAddon;

#[async_trait::async_trait]
impl AccountingPreferencesAddon for ProductsAccountingPreferencesAddon {
    fn id(&self) -> &'static str {
        "finance-products"
    }

    async fn render_inputs(&self, db: &DatabaseConnection) -> Markup {
        use crate::html_form::{FormCtx, HtmlForm};
        use crate::plugins::finance_products::forms::ProductPreferencesFormField;
        use maud::html;

        let prefs = load_product_preferences(db).await;
        let inventory_display = load_account_parent_label(db, prefs.inventory_account_id).await;
        let cos_display = load_account_parent_label(db, prefs.cost_of_sales_account_id).await;

        html! {
            (ProductPreferencesForm::render_inputs(
                &FormCtx::form::<ProductPreferencesForm>()
                    .value(
                        ProductPreferencesFormField::InventoryAccountId,
                        fk_value(prefs.inventory_account_id),
                    )
                    .display(
                        ProductPreferencesFormField::InventoryAccountId,
                        &inventory_display,
                    )
                    .value(
                        ProductPreferencesFormField::CostOfSalesAccountId,
                        fk_value(prefs.cost_of_sales_account_id),
                    )
                    .display(
                        ProductPreferencesFormField::CostOfSalesAccountId,
                        &cos_display,
                    ),
            ))
        }
    }

    async fn save_from_form(
        &self,
        db: &DatabaseConnection,
        post: &crate::plugins::finance_accounts::accounting_preferences_patch::AccountingPreferencesPost,
    ) -> Result<(), String> {
        let form = post
            .deserialize::<ProductPreferencesForm>()
            .map_err(|e| e.to_string())?;

        let prefs = load_product_preferences(db).await;
        let now = Utc::now();
        let mut am: product_preferences::ActiveModel = prefs.into();
        am.inventory_account_id = Set(str_to_opt_i64(&form.inventory_account_id));
        am.cost_of_sales_account_id = Set(str_to_opt_i64(&form.cost_of_sales_account_id));
        am.updated_at = Set(Some(now));
        am.update(db).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub(crate) static PRODUCTS_ADDON: ProductsAccountingPreferencesAddon =
    ProductsAccountingPreferencesAddon;
