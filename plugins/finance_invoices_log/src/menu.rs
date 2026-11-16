//! "Logs" entry on quotation, draft, posted, and cancelled invoice sidebars.

use lariv_plugin_finance_invoices::invoice_detail_menu::{
    InvoiceDetailKind, InvoiceDetailLink, InvoiceDetailMenuRegistrar, InvoiceDetailMenuRegistry,
};

use crate::routes::{
    CancelledInvoiceLogListRouteTag, DraftInvoiceLogListRouteTag, PostedInvoiceLogListRouteTag,
    QuotationInvoiceLogListRouteTag,
};

fn logs_url(kind: InvoiceDetailKind, id: i64) -> String {
    match kind {
        InvoiceDetailKind::Quotation => QuotationInvoiceLogListRouteTag::new(id).url(),
        InvoiceDetailKind::Draft => DraftInvoiceLogListRouteTag::new(id).url(),
        InvoiceDetailKind::Posted => PostedInvoiceLogListRouteTag::new(id).url(),
        InvoiceDetailKind::Cancelled => CancelledInvoiceLogListRouteTag::new(id).url(),
    }
}

/// Install hook that adds the Logs sidebar item.
#[derive(Clone, Copy, Default)]
pub struct Hook;

impl InvoiceDetailMenuRegistrar for Hook {
    fn register_invoice_detail_links(
        self,
        cap: InvoiceDetailMenuRegistry,
    ) -> InvoiceDetailMenuRegistry {
        cap.push(InvoiceDetailLink {
            key: "logs",
            title: "Logs",
            url_fn: logs_url,
            order: 20,
            superuser_only: true,
        })
    }
}
