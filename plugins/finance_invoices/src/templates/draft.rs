//! Draft invoice modals, detail, and draft multi-select picker.

use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_core::components::picker::RenderPickerSelect;
use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DetailHeader, FieldText, FieldTextarea, FilterClauseView,
    FormOpts, ManyToManyItem, ObjectList, ShellChrome, TableButtonFilter, TableColumnHeader,
    TableRow, button_modal_form, button_modal_route, button_submit, column_sort_url,
    container_column, container_row, detail, detail_header, field_text, field_textarea, form,
    form_hx_post_url, label, modal_keyed, render_picker_pagination, row_attr_select_multi,
    sort_indicator, table_button_filter,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};

use lariv_core::template::{RenderAppPane, RenderTemplate};
use lariv_core::web::{modal_create_post_url, modal_edit_post_url};

use lariv_plugin_finance_accounts::templates::{
    app_scaffold_with_sidebar, data_table_with_filter_errors, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs,
};

use crate::components::field_invoice_lines;
use crate::forms::{
    DraftInvoiceBulkEditForm, DraftInvoiceBulkEditFormField, DraftInvoiceForm,
    DraftInvoiceFormField,
};
use crate::keys::{
    DRAFT_INVOICE_BULK_EDIT_MODAL, DRAFT_INVOICE_CREATE_MODAL, DRAFT_INVOICE_DELETE_MODAL,
    DRAFT_INVOICE_EDIT_MODAL, DRAFT_INVOICE_SELECT_MODAL, DRAFT_INVOICE_SELECT_TABLE,
};
use crate::logic::PaymentTermLineDisplayRow;
use crate::logic::invoice_line_editor::InvoiceLineDisplayRow;
use crate::routes::{
    DraftInvoiceBulkEditPostRouteTag, DraftInvoiceCreatePostRouteTag,
    DraftInvoiceDeleteGetRouteTag, DraftInvoiceDetailRouteTag, DraftInvoiceEditGetRouteTag,
    DraftInvoiceEditPostRouteTag, DraftInvoiceMultiSelectRouteTag, DraftInvoicePdfModalRouteTag,
    DraftInvoicePostGetRouteTag,
};
use lariv_core::list_filters::{FilterField, FilterRowError};

use super::common::{
    customer_link, draft_invoice_detail_menu, draft_invoice_label, field_payment_term_schedule,
    invoice_section_crumbs,
};
use super::hub::invoice_select_filter_form;

/// Edit draft invoice form (modal). Create uses [`DraftInvoiceCreateModalPage`].
#[derive(Generic)]
pub struct DraftInvoiceEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub form: DraftInvoiceForm,
    pub error: String,
    pub customer_display: String,
    pub tax_items: Vec<ManyToManyItem>,
    pub invoice_lines_preview: String,
    pub extra_inputs: String,
}

impl RenderTemplate for DraftInvoiceEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = DraftInvoiceDeleteGetRouteTag::new(self.id).url();
        modal_keyed(
            DRAFT_INVOICE_EDIT_MODAL,
            "!max-w-6xl w-full",
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit draft invoice" }
                (form(&CsrfToken::current(), FormOpts {
                    classes: "@container",
                    attrs: form_hx_post_url(DRAFT_INVOICE_EDIT_MODAL, &modal_edit_post_url(
                        DraftInvoiceEditPostRouteTag::new(self.id),
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
                            name: "p_finance_invoices.DraftInvoiceDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: DRAFT_INVOICE_DELETE_MODAL.id(),
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
pub struct DraftInvoiceCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub form: DraftInvoiceForm,
    pub customer_display: String,
    pub tax_items: Vec<ManyToManyItem>,
    pub invoice_lines_preview: String,
    pub extra_inputs: String,
    pub error: String,
}

impl RenderTemplate for DraftInvoiceCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_invoices.DraftInvoiceCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed(
            DRAFT_INVOICE_CREATE_MODAL,
            "!max-w-6xl w-full",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create draft invoice",
                    subtitle: "Create a new draft invoice",
                    classes: "@container",
                    attrs: form_hx_post_url(
                        DRAFT_INVOICE_CREATE_MODAL,
                        &modal_create_post_url(
                            DraftInvoiceCreatePostRouteTag,
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
                                .m2m(DraftInvoiceFormField::Taxes, &self.tax_items)
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

/// Bulk-edit selected draft invoices (blank form; non-empty fields apply to all).
#[derive(Generic)]
pub struct DraftInvoiceBulkEditModalPage {
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

impl RenderTemplate for DraftInvoiceBulkEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_invoices.DraftInvoiceBulkEditForm"
        } else {
            self.form_name.as_str()
        };
        let subtitle = if self.selected_count == 1 {
            "Update the selected draft invoice. Only non-empty fields are applied.".to_string()
        } else {
            format!(
                "Update {} selected draft invoices. Only non-empty fields are applied to every selected draft invoice.",
                self.selected_count
            )
        };
        modal_keyed(
            DRAFT_INVOICE_BULK_EDIT_MODAL,
            "!max-w-6xl w-full",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Bulk edit draft invoices",
                    subtitle: &subtitle,
                    classes: "@container",
                    attrs: form_hx_post_url(
                        DRAFT_INVOICE_BULK_EDIT_MODAL,
                        &modal_create_post_url(
                            DraftInvoiceBulkEditPostRouteTag,
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
                                .m2m(DraftInvoiceBulkEditFormField::Taxes, &self.tax_items)
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
pub struct DraftInvoiceDetailPage {
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

impl DraftInvoiceDetailPage {
    fn body(&self) -> Markup {
        let actions = html! {
            (button_modal_route(DraftInvoicePdfModalRouteTag::new(self.id), "PDF", "btn-outline"))
            @if self.can_edit {
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_invoices.DraftInvoiceEditForm",
                    href: &DraftInvoiceEditGetRouteTag::new(self.id).url(),
                    form_post_url: &DraftInvoiceEditPostRouteTag::new(self.id).path(),
                    modal_uid: DRAFT_INVOICE_EDIT_MODAL.id(),
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
                (button_modal_route(
                    DraftInvoicePostGetRouteTag::new(self.id),
                    "Post invoice",
                    "btn-primary",
                ))
            }
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &format!("Draft invoice #{}", self.id),
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
        draft_invoice_detail_menu(self.id, &self.number)
    }
}

impl RenderAppPane for DraftInvoiceDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let label = draft_invoice_label(self.id, &self.number);
        let detail_url = DraftInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        let label = draft_invoice_label(self.id, &self.number);
        let detail_url = DraftInvoiceDetailRouteTag::new(self.id).url();
        layout_main_with_crumbs(
            invoice_section_crumbs(&label, &detail_url, None),
            self.body(),
        )
    }
}

impl RenderTemplate for DraftInvoiceDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = draft_invoice_label(self.id, &self.number);
        let detail_url = DraftInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        app_scaffold_with_sidebar("Draft invoice", chrome, self.menu(), crumbs, self.body())
    }
}
#[derive(Clone)]
pub struct DraftInvoiceSelectRow {
    pub id: i64,
    pub number: String,
    pub datetime: String,
    pub customer_name: String,
}

#[derive(Generic)]
pub struct DraftInvoiceSelectPage {
    pub invoices: ObjectList<DraftInvoiceSelectRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    /// Pre-rendered addon filter fields, matching the invoice hub.
    pub extra_filters: String,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl RenderPickerSelect for DraftInvoiceSelectPage {
    fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "Invoices"
        } else {
            self.target_input.as_str()
        };
        let id_sort = column_sort_url(&self.path_and_query, "ID", &self.sort);
        let number_sort = column_sort_url(&self.path_and_query, "Number", &self.sort);
        let date_sort = column_sort_url(&self.path_and_query, "Date", &self.sort);
        let id_label = format!("ID{}", sort_indicator(&self.sort, "ID"));
        let number_label = format!("Number{}", sort_indicator(&self.sort, "Number"));
        let date_label = format!("Date{}", sort_indicator(&self.sort, "Date"));
        let headers = [
            TableColumnHeader {
                key: "ID",
                label: &id_label,
                sort_url: Some(&id_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Number",
                label: &number_label,
                sort_url: Some(&number_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Date",
                label: &date_label,
                sort_url: Some(&date_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Customer",
                label: "Customer",
                sort_url: None,
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .invoices
            .items
            .iter()
            .map(|inv| TableRow {
                attrs: row_attr_select_multi(target, &inv.id.to_string(), &inv.number),
                cells: vec![
                    field_text(FieldText {
                        value: &inv.id.to_string(),
                        classes: "tabular-nums",
                    }),
                    field_text(FieldText {
                        value: &inv.number,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &inv.datetime,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &inv.customer_name,
                        classes: "",
                    }),
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: invoice_select_filter_form(
                    &self.fields,
                    &self.filters,
                    self.page_size,
                    &self.sort,
                    DraftInvoiceMultiSelectRouteTag,
                    &self.target_input,
                    &self.extra_filters,
                ),
                ..Default::default()
            }))
        };
        let pagination = render_picker_pagination(
            DRAFT_INVOICE_SELECT_MODAL,
            &self.path_and_query,
            self.invoices.number,
            self.invoices.num_pages,
        );
        data_table_with_filter_errors(
            DRAFT_INVOICE_SELECT_TABLE,
            "Select draft invoices",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl RenderTemplate for DraftInvoiceSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal(DRAFT_INVOICE_SELECT_MODAL).into_inner()
    }
}
