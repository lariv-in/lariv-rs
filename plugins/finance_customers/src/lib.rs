//! Finance customers plugin: accounting list, detail, and journal.

pub mod accounting_sidebar;
pub mod handlers;
pub mod journal;
pub mod routes;
pub mod templates;
mod xlsx;

pub struct FinanceCustomersTag;

lariv_core::define_plugin_install! {
    plugin: FinanceCustomersTag;
    steps: [
        cap_attach(journal::CustomerJournalTag, journal::CustomerJournalCap, journal::CustomerJournalCap::<frunk::HNil>::new()),
        cap_hook(lariv_plugin_finance_accounts::accounting_sidebar::AccountingSidebarTag, lariv_plugin_finance_accounts::accounting_sidebar::AccountingSidebarCap, accounting_sidebar::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        http(routes::Hook),
    ]
}
