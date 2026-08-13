//! Sidebar links and accounting preferences patched onto the shared Accounting app.

use crate::plugins::finance_accounts::accounting_sidebar::{self, AccountingSidebarRegistrar};

use crate::plugins::finance_invoices::routes::{InvoiceDefaultRouteTag, PaymentListRouteTag};

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl AccountingSidebarRegistrar for Hook {
    fn register_accounting_sidebar(
        self,
        cap: accounting_sidebar::AccountingSidebarRegistry,
    ) -> accounting_sidebar::AccountingSidebarRegistry {
        cap.push(accounting_sidebar::link::<InvoiceDefaultRouteTag>(
            "invoices",
            "Invoices",
            100,
            Some("document-text"),
        ))
        .push(accounting_sidebar::link::<PaymentListRouteTag>(
            "payments",
            "Payments",
            120,
            Some("banknotes"),
        ))
    }

    fn register_accounting_preferences(
        self,
        cap: crate::plugins::finance_accounts::accounting_preferences_patch::AccountingPreferencesRegistry,
    ) -> crate::plugins::finance_accounts::accounting_preferences_patch::AccountingPreferencesRegistry
    {
        cap.register_addon(
            &crate::plugins::finance_invoices::accounting_preferences_patch::INVOICES_ADDON,
        )
    }
}
