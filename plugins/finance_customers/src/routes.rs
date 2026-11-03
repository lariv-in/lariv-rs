use lariv_plugin_customer::keys::CUSTOMER_TABLE;

use super::handlers;

pub struct FinanceCustomersView;

lariv_core::define_plugin_routes! {
    plugin: FinanceCustomersTag;
    prefix: "/dashboard";
    routes: [
        get CustomerListRouteTag, "/finance/customers", handlers::list, fragment(CUSTOMER_TABLE), authorize(FinanceCustomersView, []);
        get CustomerDetailRouteTag, "/finance/customers/{id}", handlers::detail, authorize(FinanceCustomersView, []);
        get CustomerJournalRouteTag, "/finance/customers/{id}/journal", handlers::journal, authorize(FinanceCustomersView, []);
    ]
}
