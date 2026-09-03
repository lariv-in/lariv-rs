//! Finance integration for the customers plugin (accounting sidebar, addon app, finance chrome).

pub mod accounting_sidebar;
pub mod apps;
pub mod rune_env;
#[cfg(feature = "plugin-finance-customer")]
pub mod templates;

pub struct FinanceCustomerTag;

crate::define_plugin_install! {
    plugin: FinanceCustomerTag;
    steps: [
        cap_hook(crate::plugins::finance_accounts::accounting_sidebar::AccountingSidebarTag, crate::plugins::finance_accounts::accounting_sidebar::AccountingSidebarCap, accounting_sidebar::Hook),
        apps(apps::Hook),
        rune_env(rune_env::Hook),
    ]
}
