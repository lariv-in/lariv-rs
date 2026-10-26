//! Sidebar link and quotation preferences on the shared Accounting app.

use lariv_plugin_finance_accounts::accounting_sidebar::{self, AccountingSidebarRegistrar};

use crate::routes::QuotationDefaultRouteTag;

pub const QUOTATIONS_APP_KEY: &str = "p_finance_quotations";

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl AccountingSidebarRegistrar for Hook {
    fn register_accounting_sidebar(
        self,
        cap: accounting_sidebar::AccountingSidebarRegistry,
    ) -> accounting_sidebar::AccountingSidebarRegistry {
        cap.push(accounting_sidebar::link::<QuotationDefaultRouteTag>(
            "quotations",
            "Quotations",
            90,
            Some("document-text"),
        ))
    }

    fn register_accounting_preferences(
        self,
        cap: lariv_plugin_finance_accounts::accounting_preferences_patch::AccountingPreferencesRegistry,
    ) -> lariv_plugin_finance_accounts::accounting_preferences_patch::AccountingPreferencesRegistry
    {
        cap.register_addon(&crate::logic::prefs::PREFERENCES_ADDON)
    }
}
