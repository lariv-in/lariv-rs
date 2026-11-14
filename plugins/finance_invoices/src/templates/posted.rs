//! Posted invoice detail and paid / partially paid settlement views.

use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_core::components::{
    ButtonModalForm, DetailHeader, FieldText, FieldTextarea, ShellChrome, button_modal_form,
    button_modal_route, container_column, detail, detail_header, field_text, field_textarea, label,
};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use lariv_plugin_finance_accounts::templates::{
    app_scaffold_with_sidebar, layout_main_with_crumbs, layout_with_entity_sidebar_crumbs,
};

use crate::components::field_invoice_lines;
use crate::keys::INVOICE_PAY_MODAL;
use crate::logic::PaymentTermLineDisplayRow;
use crate::logic::invoice_line_editor::InvoiceLineDisplayRow;
use crate::routes::{
    PaidInvoiceDetailRouteTag, PaidInvoicePdfModalRouteTag, PartiallyPaidInvoiceDetailRouteTag,
    PartiallyPaidInvoicePdfModalRouteTag, PostedInvoiceCancelGetRouteTag,
    PostedInvoiceDetailRouteTag, PostedInvoiceNewDraftGetRouteTag, PostedInvoicePayGetRouteTag,
    PostedInvoicePayPostRouteTag, PostedInvoicePdfModalRouteTag,
    PostedInvoiceResetDraftGetRouteTag,
};

use super::common::{
    cancelled_detail_link, customer_link, field_payment_term_schedule, invoice_number_label,
    invoice_section_crumbs, journal_entry_link, paid_invoice_detail_menu,
    partially_paid_invoice_detail_menu, posted_invoice_detail_menu,
};

pub struct PostedInvoiceDetailPage {
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
    pub journal_entry_id: i64,
    pub can_edit: bool,
    pub can_pay: bool,
    pub extra_detail: String,
}

impl PostedInvoiceDetailPage {
    fn body(&self) -> Markup {
        let pay_href = PostedInvoicePayGetRouteTag::new(self.id).url();
        let pay_post = PostedInvoicePayPostRouteTag::new(self.id).path();
        let actions = html! {
            (button_modal_route(PostedInvoicePdfModalRouteTag::new(self.id), "PDF", "btn-outline"))
            @if self.can_edit {
                (button_modal_route(
                    PostedInvoiceNewDraftGetRouteTag::new(self.id),
                    "New quotation",
                    "btn-outline",
                ))
                (button_modal_route(
                    PostedInvoiceResetDraftGetRouteTag::new(self.id),
                    "Reset to draft",
                    "btn-warning",
                ))
            }
            @if self.can_pay {
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_invoices.InvoicePaymentForm",
                    label: "Pay",
                    href: &pay_href,
                    form_post_url: &pay_post,
                    modal_uid: INVOICE_PAY_MODAL.id(),
                    classes: "btn-primary",
                    ..Default::default()
                }))
            }
            @if self.can_edit {
                a class="btn btn-error" href=(PostedInvoiceCancelGetRouteTag::new(self.id).url()) { "Cancel" }
            }
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &format!("Posted invoice {}", self.number),
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
                    (PreEscaped(&self.extra_detail))
                    (label("Journal entry", journal_entry_link(self.journal_entry_id)))
                    h3 class="font-semibold mt-4" { "Lines" }
                    (field_invoice_lines(&self.line_rows))
                }))
            }))
        }
    }

    fn menu(&self) -> Markup {
        posted_invoice_detail_menu(self.id, &self.number)
    }
}

impl RenderAppPane for PostedInvoiceDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let label = invoice_number_label(self.id, &self.number);
        let detail_url = PostedInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        let label = invoice_number_label(self.id, &self.number);
        let detail_url = PostedInvoiceDetailRouteTag::new(self.id).url();
        layout_main_with_crumbs(
            invoice_section_crumbs(&label, &detail_url, None),
            self.body(),
        )
    }
}

impl RenderTemplate for PostedInvoiceDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = invoice_number_label(self.id, &self.number);
        let detail_url = PostedInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        app_scaffold_with_sidebar("Posted Invoice", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Clone)]
pub struct SettlementDetailContext {
    pub settlement_id: i64,
    pub posted_invoice_id: i64,
    pub number: String,
    pub reference: String,
    pub eway_bill_number: String,
    pub payment_reference: String,
    pub bank_account: String,
    pub remarks: String,
    pub datetime: String,
    pub posted_at: Option<String>,
    pub bill_to_individual: bool,
    pub customer_id: i64,
    pub customer_name: String,
    pub payment_term_rows: Vec<PaymentTermLineDisplayRow>,
    pub tax_labels: String,
    pub line_rows: Vec<InvoiceLineDisplayRow>,
    pub journal_entry_id: i64,
    pub payment_id: i64,
    pub payment_label: String,
    pub payment_href: String,
    pub payment_datetime: String,
    pub prior_partial_label: Option<String>,
    pub prior_partial_href: Option<String>,
}

impl SettlementDetailContext {
    fn empty(settlement_id: i64) -> Self {
        Self {
            settlement_id,
            posted_invoice_id: 0,
            number: "Not found".to_string(),
            reference: String::new(),
            eway_bill_number: String::new(),
            payment_reference: String::new(),
            bank_account: String::new(),
            remarks: String::new(),
            datetime: String::new(),
            posted_at: None,
            bill_to_individual: false,
            customer_id: 0,
            customer_name: String::new(),
            payment_term_rows: vec![],
            tax_labels: String::new(),
            line_rows: vec![],
            journal_entry_id: 0,
            payment_id: 0,
            payment_label: String::new(),
            payment_href: String::new(),
            payment_datetime: String::new(),
            prior_partial_label: None,
            prior_partial_href: None,
        }
    }
}

fn settlement_detail_body(
    title: &str,
    ctx: &SettlementDetailContext,
    pdf_route: impl lariv_core::http::RouteUrl,
    can_pay: bool,
    can_edit: bool,
) -> Markup {
    let pay_href = PostedInvoicePayGetRouteTag::new(ctx.posted_invoice_id).url();
    let pay_post = PostedInvoicePayPostRouteTag::new(ctx.posted_invoice_id).path();
    let actions = html! {
        (button_modal_route(pdf_route, "PDF", "btn-outline"))
        @if can_pay {
            (button_modal_form(ButtonModalForm {
                name: "p_finance_invoices.InvoicePaymentForm",
                label: "Pay",
                href: &pay_href,
                form_post_url: &pay_post,
                modal_uid: INVOICE_PAY_MODAL.id(),
                classes: "btn-primary",
                ..Default::default()
            }))
        }
        @if can_edit && ctx.posted_invoice_id > 0 {
            a class="btn btn-error" href=(PostedInvoiceCancelGetRouteTag::new(ctx.posted_invoice_id).url()) { "Cancel" }
        }
    };
    let posted_at_display = ctx.posted_at.as_deref().unwrap_or("—");
    html! {
        (detail(html! {
            (container_column("", html! {
                (detail_header(DetailHeader { title, actions }))
                (label("Number", field_text(FieldText { value: &ctx.number, classes: "" })))
                (label("Reference", field_text(FieldText { value: &ctx.reference, classes: "" })))
                (label("E-Way bill number", field_text(FieldText { value: &ctx.eway_bill_number, classes: "" })))
                (label("Payment reference", field_text(FieldText { value: &ctx.payment_reference, classes: "" })))
                (label("Account", field_textarea(FieldTextarea { value: &ctx.bank_account, classes: "" })))
                (label("Remarks", field_textarea(FieldTextarea { value: &ctx.remarks, classes: "" })))
                (label("Posted at", field_text(FieldText { value: posted_at_display, classes: "" })))
                (label("Invoice date", field_text(FieldText { value: &ctx.datetime, classes: "" })))
                (label("Customer", customer_link(ctx.bill_to_individual, ctx.customer_id, &ctx.customer_name)))
                (label("Payment schedule", field_payment_term_schedule(&ctx.payment_term_rows)))
                (label("Taxes", field_text(FieldText { value: &ctx.tax_labels, classes: "" })))
                (label("Journal entry", journal_entry_link(ctx.journal_entry_id)))
                (label("Payment", cancelled_detail_link(&Some(ctx.payment_href.clone()), &ctx.payment_label)))
                (label("Payment date", field_text(FieldText { value: &ctx.payment_datetime, classes: "" })))
                (label("Prior partial record", cancelled_detail_link(
                    &ctx.prior_partial_href,
                    ctx.prior_partial_label.as_deref().unwrap_or("—"),
                )))
                h3 class="font-semibold mt-4" { "Lines" }
                (field_invoice_lines(&ctx.line_rows))
            }))
        }))
    }
}

#[derive(Generic)]
pub struct PaidInvoiceDetailPage {
    pub ctx: SettlementDetailContext,
    pub can_edit: bool,
    pub can_pay: bool,
}

impl PaidInvoiceDetailPage {
    pub fn not_found(id: i64) -> Self {
        Self {
            ctx: SettlementDetailContext::empty(id),
            can_edit: false,
            can_pay: false,
        }
    }

    fn body(&self) -> Markup {
        let title = if self.ctx.number.is_empty() || self.ctx.number == "Not found" {
            format!("Paid invoice #{}", self.ctx.settlement_id)
        } else {
            format!("Paid invoice {}", self.ctx.number)
        };
        settlement_detail_body(
            &title,
            &self.ctx,
            PaidInvoicePdfModalRouteTag::new(self.ctx.settlement_id),
            self.can_pay,
            self.can_edit,
        )
    }

    fn menu(&self) -> Markup {
        paid_invoice_detail_menu(self.ctx.settlement_id, &self.ctx.number)
    }
}

impl RenderAppPane for PaidInvoiceDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let label = invoice_number_label(self.ctx.settlement_id, &self.ctx.number);
        let detail_url = PaidInvoiceDetailRouteTag::new(self.ctx.settlement_id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        let label = invoice_number_label(self.ctx.settlement_id, &self.ctx.number);
        let detail_url = PaidInvoiceDetailRouteTag::new(self.ctx.settlement_id).url();
        layout_main_with_crumbs(
            invoice_section_crumbs(&label, &detail_url, None),
            self.body(),
        )
    }
}

impl RenderTemplate for PaidInvoiceDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = invoice_number_label(self.ctx.settlement_id, &self.ctx.number);
        let detail_url = PaidInvoiceDetailRouteTag::new(self.ctx.settlement_id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        app_scaffold_with_sidebar("Paid Invoice", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Generic)]
pub struct PartiallyPaidInvoiceDetailPage {
    pub ctx: SettlementDetailContext,
    pub can_edit: bool,
    pub can_pay: bool,
}

impl PartiallyPaidInvoiceDetailPage {
    pub fn not_found(id: i64) -> Self {
        Self {
            ctx: SettlementDetailContext::empty(id),
            can_edit: false,
            can_pay: false,
        }
    }

    fn body(&self) -> Markup {
        let title = if self.ctx.number.is_empty() || self.ctx.number == "Not found" {
            format!("Partially paid invoice #{}", self.ctx.settlement_id)
        } else {
            format!("Partially paid invoice {}", self.ctx.number)
        };
        settlement_detail_body(
            &title,
            &self.ctx,
            PartiallyPaidInvoicePdfModalRouteTag::new(self.ctx.settlement_id),
            self.can_pay,
            self.can_edit,
        )
    }

    fn menu(&self) -> Markup {
        partially_paid_invoice_detail_menu(self.ctx.settlement_id, &self.ctx.number)
    }
}

impl RenderAppPane for PartiallyPaidInvoiceDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let label = invoice_number_label(self.ctx.settlement_id, &self.ctx.number);
        let detail_url = PartiallyPaidInvoiceDetailRouteTag::new(self.ctx.settlement_id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        let label = invoice_number_label(self.ctx.settlement_id, &self.ctx.number);
        let detail_url = PartiallyPaidInvoiceDetailRouteTag::new(self.ctx.settlement_id).url();
        layout_main_with_crumbs(
            invoice_section_crumbs(&label, &detail_url, None),
            self.body(),
        )
    }
}

impl RenderTemplate for PartiallyPaidInvoiceDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = invoice_number_label(self.ctx.settlement_id, &self.ctx.number);
        let detail_url = PartiallyPaidInvoiceDetailRouteTag::new(self.ctx.settlement_id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        app_scaffold_with_sidebar(
            "Partially Paid Invoice",
            chrome,
            self.menu(),
            crumbs,
            self.body(),
        )
    }
}
