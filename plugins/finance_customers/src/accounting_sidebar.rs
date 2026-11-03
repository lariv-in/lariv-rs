//! Customers link on the shared Accounting sidebar.

use lariv_plugin_finance_accounts::accounting_sidebar::{self, AccountingSidebarRegistrar};

use crate::routes::CustomerListRouteTag;

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl AccountingSidebarRegistrar for Hook {
    fn register_accounting_sidebar(
        self,
        cap: accounting_sidebar::AccountingSidebarRegistry,
    ) -> accounting_sidebar::AccountingSidebarRegistry {
        cap.push(accounting_sidebar::link::<CustomerListRouteTag>(
            "customers",
            "Customers",
            40,
            Some("user-circle"),
        ))
    }
}
