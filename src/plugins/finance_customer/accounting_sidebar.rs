//! Sidebar links patched onto the shared Accounting app menu.

use crate::plugins::finance_accounts::accounting_sidebar::{self, AccountingSidebarRegistrar};

use crate::plugins::customer::routes::CustomerDefaultRouteTag;

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl AccountingSidebarRegistrar for Hook {
    fn register_accounting_sidebar(
        self,
        cap: accounting_sidebar::AccountingSidebarRegistry,
    ) -> accounting_sidebar::AccountingSidebarRegistry {
        cap.push(accounting_sidebar::link::<CustomerDefaultRouteTag>(
            "customers",
            "Customers",
            50,
            Some("building-storefront"),
        ))
    }
}
