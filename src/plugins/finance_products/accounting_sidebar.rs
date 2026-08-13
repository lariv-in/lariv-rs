//! Sidebar links and accounting preferences patched onto the shared Accounting app.

use crate::plugins::finance_accounts::accounting_sidebar::{self, AccountingSidebarRegistrar};

use crate::plugins::finance_products::routes::ProductDefaultRouteTag;

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl AccountingSidebarRegistrar for Hook {
    fn register_accounting_sidebar(
        self,
        cap: accounting_sidebar::AccountingSidebarRegistry,
    ) -> accounting_sidebar::AccountingSidebarRegistry {
        cap.push(accounting_sidebar::link::<ProductDefaultRouteTag>(
            "products",
            "Products",
            60,
            Some("cube"),
        ))
    }

    fn register_accounting_preferences(
        self,
        cap: crate::plugins::finance_accounts::accounting_preferences_patch::AccountingPreferencesRegistry,
    ) -> crate::plugins::finance_accounts::accounting_preferences_patch::AccountingPreferencesRegistry
    {
        cap.register_addon(
            &crate::plugins::finance_products::accounting_preferences_patch::PRODUCTS_ADDON,
        )
    }
}
