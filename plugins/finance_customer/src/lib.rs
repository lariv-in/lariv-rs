//! Finance integration for the customers plugin (accounting sidebar, addon app, finance chrome).

pub mod accounting_sidebar;
pub mod apps;
#[cfg(feature = "plugin-llm-assistant")]
pub mod hitl;
pub mod rune_env;
pub mod templates;

pub struct FinanceCustomerTag;

#[cfg(feature = "plugin-llm-assistant")]
lariv_core::define_plugin_install! {
    plugin: FinanceCustomerTag;
    steps: [
        cap_hook(lariv_plugin_finance_accounts::accounting_sidebar::AccountingSidebarTag, lariv_plugin_finance_accounts::accounting_sidebar::AccountingSidebarCap, accounting_sidebar::Hook),
        cap_hook(lariv_plugin_llm_assistant::hitl::HitlTag, lariv_plugin_llm_assistant::hitl::HitlCap, hitl::Hook),
        apps(apps::Hook),
        rune_env(rune_env::Hook),
    ]
}

#[cfg(not(feature = "plugin-llm-assistant"))]
lariv_core::define_plugin_install! {
    plugin: FinanceCustomerTag;
    steps: [
        cap_hook(lariv_plugin_finance_accounts::accounting_sidebar::AccountingSidebarTag, lariv_plugin_finance_accounts::accounting_sidebar::AccountingSidebarCap, accounting_sidebar::Hook),
        apps(apps::Hook),
        rune_env(rune_env::Hook),
    ]
}
