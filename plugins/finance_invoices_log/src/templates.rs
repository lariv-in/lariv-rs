//! Per-invoice audit log page, inside the invoice detail chrome.

use maud::Markup;

use lariv_core::capability::define_register_items;
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar};
use lariv_plugin_finance_accounts::templates::{
    app_scaffold_with_sidebar, layout_main_with_crumbs, layout_with_entity_sidebar_crumbs,
};
use lariv_plugin_finance_invoices::invoice_detail_menu::InvoiceDetailKind;
use lariv_plugin_finance_invoices::templates::common::{
    cancelled_invoice_detail_menu, draft_invoice_detail_menu, draft_invoice_label,
    invoice_number_label, invoice_section_crumbs, posted_invoice_detail_menu,
    quotation_invoice_detail_menu, quotation_invoice_label,
};
use lariv_plugin_logging::templates::LogListPage;

define_register_items! {
    plugin: FinanceInvoicesLogTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        InvoiceLogIdx: InvoiceLogPageTag => InvoiceLogPage,
    ]
}

/// Audit log for one invoice, shown with that invoice's detail sidebar.
pub struct InvoiceLogPage {
    pub kind: InvoiceDetailKind,
    pub id: i64,
    pub number: String,
    pub logs: LogListPage,
}

impl InvoiceLogPage {
    fn sidebar(&self) -> Markup {
        match self.kind {
            InvoiceDetailKind::Quotation => {
                quotation_invoice_detail_menu(self.id, &self.number, "logs")
            }
            InvoiceDetailKind::Draft => draft_invoice_detail_menu(self.id, &self.number, "logs"),
            InvoiceDetailKind::Posted => posted_invoice_detail_menu(self.id, &self.number, "logs"),
            InvoiceDetailKind::Cancelled => {
                cancelled_invoice_detail_menu(self.id, &self.number, "logs")
            }
        }
    }

    fn label(&self) -> String {
        match self.kind {
            InvoiceDetailKind::Quotation => quotation_invoice_label(self.id, &self.number),
            InvoiceDetailKind::Draft => draft_invoice_label(self.id, &self.number),
            InvoiceDetailKind::Posted | InvoiceDetailKind::Cancelled => {
                invoice_number_label(self.id, &self.number)
            }
        }
    }

    fn detail_url(&self) -> String {
        match self.kind {
            InvoiceDetailKind::Quotation => {
                lariv_plugin_finance_invoices::routes::QuotationInvoiceDetailRouteTag::new(self.id)
                    .url()
            }
            InvoiceDetailKind::Draft => {
                lariv_plugin_finance_invoices::routes::DraftInvoiceDetailRouteTag::new(self.id)
                    .url()
            }
            InvoiceDetailKind::Posted => {
                lariv_plugin_finance_invoices::routes::PostedInvoiceDetailRouteTag::new(self.id)
                    .url()
            }
            InvoiceDetailKind::Cancelled => {
                lariv_plugin_finance_invoices::routes::CancelledInvoiceDetailRouteTag::new(self.id)
                    .url()
            }
        }
    }

    fn crumbs(&self) -> Markup {
        let label = self.label();
        let detail_url = self.detail_url();
        invoice_section_crumbs(&label, &detail_url, Some("Logs"))
    }
}

impl lariv_core::template::RenderAppPane for InvoiceLogPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_entity_sidebar_crumbs(self.sidebar(), self.crumbs(), self.logs.render_table())
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(self.crumbs(), self.logs.render_table())
    }
}

impl RenderTemplate for InvoiceLogPage {
    fn render(&self, chrome: &lariv_core::components::ShellChrome) -> Markup {
        app_scaffold_with_sidebar(
            "Invoice logs",
            chrome,
            self.sidebar(),
            self.crumbs(),
            self.logs.render_table(),
        )
    }
}
