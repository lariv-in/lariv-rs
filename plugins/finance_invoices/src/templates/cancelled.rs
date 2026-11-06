//! Cancelled invoice detail page.

use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    DetailHeader, FieldText, FieldTextarea, ShellChrome, button_modal_route, container_column,
    detail, detail_header, field_text, field_textarea, label,
};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use lariv_plugin_finance_accounts::templates::{
    app_scaffold_with_sidebar, layout_main_with_crumbs, layout_with_entity_sidebar_crumbs,
};

use crate::components::field_invoice_lines;
use crate::logic::PaymentTermLineDisplayRow;
use crate::logic::invoice_line_editor::InvoiceLineDisplayRow;
use crate::routes::{
    CancelledInvoiceDetailRouteTag, CancelledInvoiceNewDraftGetRouteTag,
    CancelledInvoicePdfModalRouteTag,
};

use super::common::{
    cancelled_detail_link, cancelled_invoice_detail_menu, customer_link,
    field_payment_term_schedule, invoice_number_label, invoice_section_crumbs,
};

#[derive(Generic)]
pub struct CancelledInvoiceDetailPage {
    pub id: i64,
    pub number: String,
    pub reference: String,
    pub eway_bill_number: String,
    pub payment_reference: String,
    pub bank_account: String,
    pub remarks: String,
    pub datetime: String,
    pub delivery_date: String,
    pub bill_to_individual: bool,
    pub customer_id: i64,
    pub customer_name: String,
    pub payment_term_rows: Vec<PaymentTermLineDisplayRow>,
    pub tax_labels: String,
    pub line_rows: Vec<InvoiceLineDisplayRow>,
    pub posted_invoice_label: String,
    pub posted_invoice_href: Option<String>,
    pub credit_note_label: String,
    pub credit_note_href: Option<String>,
    pub can_edit: bool,
}

impl CancelledInvoiceDetailPage {
    fn body(&self) -> Markup {
        let actions = html! {
            (button_modal_route(CancelledInvoicePdfModalRouteTag::new(self.id), "PDF", "btn-outline"))
            @if self.can_edit {
                (button_modal_route(
                    CancelledInvoiceNewDraftGetRouteTag::new(self.id),
                    "New quotation from cancelled",
                    "btn-primary",
                ))
            }
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &format!("Cancelled invoice {}", self.number),
                        actions,
                    }))
                    (label("Reference", field_text(FieldText { value: &self.reference, classes: "" })))
                    (label("E-Way bill number", field_text(FieldText { value: &self.eway_bill_number, classes: "" })))
                    (label("Payment reference", field_text(FieldText { value: &self.payment_reference, classes: "" })))
                    (label("Account", field_textarea(FieldTextarea { value: &self.bank_account, classes: "" })))
                    (label("Remarks", field_textarea(FieldTextarea { value: &self.remarks, classes: "" })))
                    (label("Date", field_text(FieldText { value: &self.datetime, classes: "" })))
                    (label("Delivery date", field_text(FieldText { value: &self.delivery_date, classes: "" })))
                    (label("Customer", customer_link(self.bill_to_individual, self.customer_id, &self.customer_name)))
                    (label("Payment schedule", field_payment_term_schedule(&self.payment_term_rows)))
                    (label("Taxes", field_text(FieldText { value: &self.tax_labels, classes: "" })))
                    (label("Posted invoice", cancelled_detail_link(&self.posted_invoice_href, &self.posted_invoice_label)))
                    (label("Credit note", cancelled_detail_link(&self.credit_note_href, &self.credit_note_label)))
                    h3 class="font-semibold mt-4" { "Lines" }
                    (field_invoice_lines(&self.line_rows))
                }))
            }))
        }
    }

    fn menu(&self) -> Markup {
        cancelled_invoice_detail_menu(self.id, &self.number)
    }
}

impl RenderAppPane for CancelledInvoiceDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let label = invoice_number_label(self.id, &self.number);
        let detail_url = CancelledInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        let label = invoice_number_label(self.id, &self.number);
        let detail_url = CancelledInvoiceDetailRouteTag::new(self.id).url();
        layout_main_with_crumbs(
            invoice_section_crumbs(&label, &detail_url, None),
            self.body(),
        )
    }
}

impl RenderTemplate for CancelledInvoiceDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = invoice_number_label(self.id, &self.number);
        let detail_url = CancelledInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        app_scaffold_with_sidebar(
            "Cancelled Invoice",
            chrome,
            self.menu(),
            crumbs,
            self.body(),
        )
    }
}
