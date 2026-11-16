//! Per-invoice audit log routes. Empty allowlist: superuser only.

use lariv_core::define_plugin_routes;
use lariv_plugin_logging::keys::LOG_TABLE;

use super::handlers;

/// Reading one invoice's audit log. Empty allowlist: superuser only.
pub struct InvoiceLogsView;

define_plugin_routes! {
    plugin: FinanceInvoicesLogTag;
    prefix: "/dashboard";
    routes: [
        get QuotationInvoiceLogListRouteTag, "/finance-invoices/quotations/{id}/logs", handlers::quotation, fragment(LOG_TABLE), authorize(InvoiceLogsView, []);
        get DraftInvoiceLogListRouteTag, "/finance-invoices/i/{id}/logs", handlers::draft, fragment(LOG_TABLE), authorize(InvoiceLogsView, []);
        get PostedInvoiceLogListRouteTag, "/finance-invoices/posted/{id}/logs", handlers::posted, fragment(LOG_TABLE), authorize(InvoiceLogsView, []);
        get CancelledInvoiceLogListRouteTag, "/finance-invoices/cancelled/{id}/logs", handlers::cancelled, fragment(LOG_TABLE), authorize(InvoiceLogsView, []);
    ]
}
