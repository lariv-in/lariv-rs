//! Finance integration for the customers plugin (accounting sidebar, addon app, finance chrome).

pub mod accounting_sidebar;
pub mod apps;
#[cfg(feature = "plugin-llm-assistant")]
pub mod hitl;
pub mod rune_env;
#[cfg(feature = "plugin-finance-customer")]
pub mod templates;

pub struct FinanceCustomerTag;

#[cfg(feature = "plugin-llm-assistant")]
crate::define_plugin_install! {
    plugin: FinanceCustomerTag;
    steps: [
        cap_hook(crate::plugins::finance_accounts::accounting_sidebar::AccountingSidebarTag, crate::plugins::finance_accounts::accounting_sidebar::AccountingSidebarCap, accounting_sidebar::Hook),
        cap_hook(crate::plugins::llm_assistant::hitl::HitlTag, crate::plugins::llm_assistant::hitl::HitlCap, hitl::Hook),
        apps(apps::Hook),
        rune_env(rune_env::Hook),
    ]
}

#[cfg(not(feature = "plugin-llm-assistant"))]
crate::define_plugin_install! {
    plugin: FinanceCustomerTag;
    steps: [
        cap_hook(crate::plugins::finance_accounts::accounting_sidebar::AccountingSidebarTag, crate::plugins::finance_accounts::accounting_sidebar::AccountingSidebarCap, accounting_sidebar::Hook),
        apps(apps::Hook),
        rune_env(rune_env::Hook),
    ]
}
