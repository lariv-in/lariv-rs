//! Sidebar links and accounting preferences patched onto the shared Accounting app.

use lariv_plugin_finance_accounts::accounting_sidebar::{self, AccountingSidebarRegistrar};

use crate::routes::ProductDefaultRouteTag;

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
        cap: lariv_plugin_finance_accounts::accounting_preferences_patch::AccountingPreferencesRegistry,
    ) -> lariv_plugin_finance_accounts::accounting_preferences_patch::AccountingPreferencesRegistry
    {
        cap.register_addon(&crate::accounting_preferences_patch::PRODUCTS_ADDON)
    }
}
