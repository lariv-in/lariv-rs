//! Quotation screens. Field shape matches drafts; posting stays on drafts.

use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DetailHeader, FieldText, FieldTextarea, FormOpts,
    ManyToManyItem, ShellChrome, button_modal_form, button_modal_route, button_submit,
    container_column, container_row, detail, detail_header, field_text, field_textarea, form,
    form_hx_post_url, label, modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::{RenderAppPane, RenderTemplate};
use lariv_core::web::{modal_create_post_url, modal_edit_post_url};
use lariv_plugin_finance_accounts::templates::{
    app_scaffold_with_sidebar, layout_main_with_crumbs, layout_with_entity_sidebar_crumbs,
};

use crate::forms::{
    DraftInvoiceBulkEditForm, DraftInvoiceBulkEditFormField, DraftInvoiceForm,
    DraftInvoiceFormField,
};
use crate::keys::{
    QUOTATION_INVOICE_BULK_EDIT_MODAL, QUOTATION_INVOICE_CREATE_MODAL,
    QUOTATION_INVOICE_DELETE_MODAL, QUOTATION_INVOICE_EDIT_MODAL,
};
use crate::logic::PaymentTermLineDisplayRow;
use crate::logic::invoice_line_editor::InvoiceLineDisplayRow;
use crate::routes::{
    QuotationInvoiceBulkEditPostRouteTag, QuotationInvoiceCreateDraftGetRouteTag,
    QuotationInvoiceCreatePostRouteTag, QuotationInvoiceDeleteGetRouteTag,
    QuotationInvoiceDetailRouteTag, QuotationInvoiceEditGetRouteTag,
    QuotationInvoiceEditPostRouteTag, QuotationInvoicePdfModalRouteTag,
};

use super::{
    customer_link, field_invoice_lines, field_payment_term_schedule, invoice_section_crumbs,
    quotation_invoice_detail_menu, quotation_invoice_label,
};

/// Edit quotation form (modal). Create uses [`QuotationInvoiceCreateModalPage`].
#[derive(Generic)]
pub struct QuotationInvoiceEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub form: DraftInvoiceForm,
    pub error: String,
    pub customer_display: String,
    pub tax_items: Vec<ManyToManyItem>,
    pub invoice_lines_preview: String,
    pub extra_inputs: String,
}

impl RenderTemplate for QuotationInvoiceEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = QuotationInvoiceDeleteGetRouteTag::new(self.id).url();
        modal_keyed(
            QUOTATION_INVOICE_EDIT_MODAL,
            "!max-w-6xl w-full",
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit quotation" }
                (form(&CsrfToken::current(), FormOpts {
                    classes: "@container",
                    attrs: form_hx_post_url(QUOTATION_INVOICE_EDIT_MODAL, &modal_edit_post_url(
                        QuotationInvoiceEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (DraftInvoiceForm::render_inputs(&FormCtx::form::<DraftInvoiceForm>(CsrfToken::current())
                            .value(DraftInvoiceFormField::Number, &self.form.number)
                            .value(DraftInvoiceFormField::Reference, &self.form.reference)
                            .value(DraftInvoiceFormField::EwayBillNumber, &self.form.eway_bill_number)
                            .value(DraftInvoiceFormField::PaymentReference, &self.form.payment_reference)
                            .value(DraftInvoiceFormField::BankAccount, &self.form.bank_account)
                            .value(DraftInvoiceFormField::Remarks, &self.form.remarks)
                            .value(DraftInvoiceFormField::Datetime, &self.form.datetime)
                            .value(DraftInvoiceFormField::DeliveryDate, &self.form.delivery_date)
                            .value(DraftInvoiceFormField::CustomerId, &self.form.customer_id.to_string())
                            .value(DraftInvoiceFormField::PaymentTermLinesJson, &self.form.payment_term_lines_json)
                            .value(DraftInvoiceFormField::InvoiceLinesJson, &self.form.invoice_lines_json)
                            .display(DraftInvoiceFormField::CustomerId, &self.customer_display)
                            .display(DraftInvoiceFormField::InvoiceLinesJson, &self.invoice_lines_preview)
                            .m2m(DraftInvoiceFormField::Taxes, &self.tax_items)))
                        (PreEscaped(&self.extra_inputs))
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_finance_invoices.QuotationInvoiceDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: QUOTATION_INVOICE_DELETE_MODAL.id(),
                            classes: "btn-error",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}
#[derive(Generic)]
pub struct QuotationInvoiceCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub form: DraftInvoiceForm,
    pub customer_display: String,
    pub tax_items: Vec<ManyToManyItem>,
    pub invoice_lines_preview: String,
    pub extra_inputs: String,
    pub error: String,
}

impl RenderTemplate for QuotationInvoiceCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_invoices.QuotationInvoiceCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed(
            QUOTATION_INVOICE_CREATE_MODAL,
            "!max-w-6xl w-full",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create quotation",
                    subtitle: "Create a new quotation",
                    classes: "@container",
                    attrs: form_hx_post_url(
                        QUOTATION_INVOICE_CREATE_MODAL,
                        &modal_create_post_url(
                            QuotationInvoiceCreatePostRouteTag,
                            form_name,
                            &self.refresh_table,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (DraftInvoiceForm::render_inputs(
                            &FormCtx::form::<DraftInvoiceForm>(CsrfToken::current())
                                .value(DraftInvoiceFormField::Number, &self.form.number)
                                .value(DraftInvoiceFormField::Reference, &self.form.reference)
                                .value(
                                    DraftInvoiceFormField::EwayBillNumber,
                                    &self.form.eway_bill_number,
                                )
                                .value(
                                    DraftInvoiceFormField::PaymentReference,
                                    &self.form.payment_reference,
                                )
                                .value(DraftInvoiceFormField::BankAccount, &self.form.bank_account)
                                .value(DraftInvoiceFormField::Remarks, &self.form.remarks)
                                .value(DraftInvoiceFormField::Datetime, &self.form.datetime)
                                .value(
                                    DraftInvoiceFormField::DeliveryDate,
                                    &self.form.delivery_date,
                                )
                                .value(
                                    DraftInvoiceFormField::CustomerId,
                                    &self.form.customer_id.to_string(),
                                )
                                .value(
                                    DraftInvoiceFormField::PaymentTermLinesJson,
                                    &self.form.payment_term_lines_json,
                                )
                                .value(
                                    DraftInvoiceFormField::InvoiceLinesJson,
                                    &self.form.invoice_lines_json,
                                )
                                .display(
                                    DraftInvoiceFormField::CustomerId,
                                    &self.customer_display,
                                )
                                .display(
                                    DraftInvoiceFormField::InvoiceLinesJson,
                                    &self.invoice_lines_preview,
                                )
                                .m2m(DraftInvoiceFormField::Taxes, &self.tax_items),
                        ))
                        (PreEscaped(&self.extra_inputs))
                    },
                    actions: html! {
                        (container_row("flex justify-end gap-2 mt-2", html! {
                            (button_submit(ButtonSubmit {
                                label: "Save",
                                classes: "btn-primary",
                                ..Default::default()
                            }))
                        }))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}

/// Bulk-edit selected quotations (blank form; non-empty fields apply to all).
#[derive(Generic)]
pub struct QuotationInvoiceBulkEditModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub ids: String,
    pub selected_count: usize,
    pub form: DraftInvoiceBulkEditForm,
    pub customer_display: String,
    pub tax_items: Vec<ManyToManyItem>,
    pub invoice_lines_preview: String,
    pub extra_inputs: String,
    pub error: String,
    pub can_submit: bool,
}

impl RenderTemplate for QuotationInvoiceBulkEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_invoices.QuotationInvoiceBulkEditForm"
        } else {
            self.form_name.as_str()
        };
        let subtitle = if self.selected_count == 1 {
            "Update the selected quotation. Only non-empty fields are applied.".to_string()
        } else {
            format!(
                "Update {} selected quotations. Only non-empty fields are applied to every selected quotation.",
                self.selected_count
            )
        };
        modal_keyed(
            QUOTATION_INVOICE_BULK_EDIT_MODAL,
            "!max-w-6xl w-full",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Bulk edit quotations",
                    subtitle: &subtitle,
                    classes: "@container",
                    attrs: form_hx_post_url(
                        QUOTATION_INVOICE_BULK_EDIT_MODAL,
                        &modal_create_post_url(
                            QuotationInvoiceBulkEditPostRouteTag,
                            form_name,
                            &self.refresh_table,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        input type="hidden" name="ids" value=(self.ids);
                        (DraftInvoiceBulkEditForm::render_inputs(
                            &FormCtx::form::<DraftInvoiceBulkEditForm>(CsrfToken::current())
                                .value(DraftInvoiceBulkEditFormField::Number, &self.form.number)
                                .value(DraftInvoiceBulkEditFormField::Reference, &self.form.reference)
                                .value(
                                    DraftInvoiceBulkEditFormField::EwayBillNumber,
                                    &self.form.eway_bill_number,
                                )
                                .value(
                                    DraftInvoiceBulkEditFormField::PaymentReference,
                                    &self.form.payment_reference,
                                )
                                .value(
                                    DraftInvoiceBulkEditFormField::BankAccount,
                                    &self.form.bank_account,
                                )
                                .value(DraftInvoiceBulkEditFormField::Remarks, &self.form.remarks)
                                .value(DraftInvoiceBulkEditFormField::Datetime, &self.form.datetime)
                                .value(
                                    DraftInvoiceBulkEditFormField::DeliveryDate,
                                    &self.form.delivery_date,
                                )
                                .value(
                                    DraftInvoiceBulkEditFormField::CustomerId,
                                    &self.form.customer_id.to_string(),
                                )
                                .value(
                                    DraftInvoiceBulkEditFormField::PaymentTermLinesJson,
                                    &self.form.payment_term_lines_json,
                                )
                                .value(
                                    DraftInvoiceBulkEditFormField::InvoiceLinesJson,
                                    &self.form.invoice_lines_json,
                                )
                                .display(
                                    DraftInvoiceBulkEditFormField::CustomerId,
                                    &self.customer_display,
                                )
                                .display(
                                    DraftInvoiceBulkEditFormField::InvoiceLinesJson,
                                    &self.invoice_lines_preview,
                                )
                                .m2m(DraftInvoiceBulkEditFormField::Taxes, &self.tax_items),
                        ))
                        (PreEscaped(&self.extra_inputs))
                    },
                    actions: html! {
                        @if self.can_submit {
                            (container_row("flex justify-end gap-2 mt-2", html! {
                                (button_submit(ButtonSubmit {
                                    label: "Apply to selected",
                                    classes: "btn-primary",
                                    ..Default::default()
                                }))
                            }))
                        }
                    },
                    ..Default::default()
                },
            ),
        )
    }
}

#[derive(Generic)]
pub struct QuotationInvoiceDetailPage {
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
    pub extra_detail: String,
    pub line_rows: Vec<InvoiceLineDisplayRow>,
    pub can_edit: bool,
    pub error: Option<String>,
}

impl QuotationInvoiceDetailPage {
    fn body(&self) -> Markup {
        let actions = html! {
            (button_modal_route(QuotationInvoicePdfModalRouteTag::new(self.id), "PDF", "btn-outline"))
            @if self.can_edit {
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_invoices.QuotationInvoiceEditForm",
                    href: &QuotationInvoiceEditGetRouteTag::new(self.id).url(),
                    form_post_url: &QuotationInvoiceEditPostRouteTag::new(self.id).path(),
                    modal_uid: QUOTATION_INVOICE_EDIT_MODAL.id(),
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
                (button_modal_route(
                    QuotationInvoiceCreateDraftGetRouteTag::new(self.id),
                    "Create draft",
                    "btn-primary",
                ))
            }
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &format!("Quotation #{}", self.id),
                        actions,
                    }))
                    @if let Some(e) = &self.error {
                        p class="text-error mb-2" { (e) }
                    }
                    (label("Number", field_text(FieldText { value: &self.number, classes: "" })))
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
                    h3 class="font-semibold mt-4" { "Lines" }
                    (field_invoice_lines(&self.line_rows))
                }))
            }))
        }
    }

    fn menu(&self) -> Markup {
        quotation_invoice_detail_menu(self.id, &self.number)
    }
}

impl RenderAppPane for QuotationInvoiceDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let label = quotation_invoice_label(self.id, &self.number);
        let detail_url = QuotationInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        let label = quotation_invoice_label(self.id, &self.number);
        let detail_url = QuotationInvoiceDetailRouteTag::new(self.id).url();
        layout_main_with_crumbs(
            invoice_section_crumbs(&label, &detail_url, None),
            self.body(),
        )
    }
}

impl RenderTemplate for QuotationInvoiceDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = quotation_invoice_label(self.id, &self.number);
        let detail_url = QuotationInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        app_scaffold_with_sidebar("Quotation", chrome, self.menu(), crumbs, self.body())
    }
}
