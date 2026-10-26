use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, Crumb, DeleteConfirmation, DetailHeader, FieldLink, FieldText,
    FieldTextarea, FormOpts, ObjectList, PaginationPage, ShellChrome, SlotCapability,
    SlotRegistrar, SwapKey, TableColumnHeader, TablePagination, TableRow, breadcrumbs,
    button_download_route, button_modal_form, button_modal_route, button_post_route, button_submit,
    container_column, container_row, data_table_list_refresh, delete_confirmation, detail,
    detail_header, field_link, field_text, field_textarea, form, form_hx_post_selector,
    form_hx_post_url, label, modal_keyed, pagination_pages, row_attr_navigate_route,
    table_create_button, table_pagination,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{
    RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar,
};
use lariv_core::web::{modal_create_post_query, modal_edit_post_url};
use lariv_plugin_finance_invoices::components::{
    field_invoice_goods_lines, field_invoice_service_lines,
};
use lariv_plugin_finance_invoices::logic::invoice_goods_line_editor::InvoiceGoodsLineDisplayRow;
use lariv_plugin_finance_invoices::logic::invoice_service_line_editor::InvoiceServiceLineDisplayRow;

use lariv_plugin_finance_accounts::accounting_detail_menu::{
    DetailMenuNavItem, detail_sidebar_menu,
};
use lariv_plugin_finance_accounts::templates::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs,
};

use crate::forms::{QuotationForm, QuotationFormField};
use crate::keys::{
    QuotationCreateModalKey, QuotationDeleteModalKey, QuotationEditModalKey,
    QuotationInvoiceModalKey, QuotationTableKey,
};
use crate::routes::{
    FinanceQuotationsMutate, QuotationCopyPostRouteTag, QuotationCreateInvoiceGetRouteTag,
    QuotationCreateInvoicePostRouteTag, QuotationCreatePostRouteTag, QuotationDefaultRouteTag,
    QuotationDeleteGetRouteTag, QuotationDeletePostRouteTag, QuotationDetailRouteTag,
    QuotationEditGetRouteTag, QuotationEditPostRouteTag, QuotationMailRouteTag,
    QuotationPdfModalRouteTag,
};

pub struct QuotationFormFields {
    pub number: String,
    pub date: String,
    pub customer_id: String,
    pub customer_display: String,
    pub bank_account: String,
    pub remarks: String,
    pub goods_json: String,
    pub service_json: String,
    pub goods_preview: String,
    pub service_preview: String,
}

fn quotation_inputs(fields: &QuotationFormFields) -> Markup {
    QuotationForm::render_inputs(
        &FormCtx::form::<QuotationForm>(CsrfToken::current())
            .value(QuotationFormField::Number, &fields.number)
            .value(QuotationFormField::Date, &fields.date)
            .value(QuotationFormField::CustomerId, &fields.customer_id)
            .display(QuotationFormField::CustomerId, &fields.customer_display)
            .value(QuotationFormField::BankAccount, &fields.bank_account)
            .value(QuotationFormField::Remarks, &fields.remarks)
            .value(QuotationFormField::GoodsLinesJson, &fields.goods_json)
            .display(QuotationFormField::GoodsLinesJson, &fields.goods_preview)
            .value(QuotationFormField::ServiceLinesJson, &fields.service_json)
            .display(
                QuotationFormField::ServiceLinesJson,
                &fields.service_preview,
            ),
    )
}

fn list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Quotations",
        href: None,
    }])
}

fn detail_crumbs(id: i64, number: &str) -> Markup {
    let list_url = QuotationDefaultRouteTag.url();
    let current = number_label(id, number);
    breadcrumbs(&[
        Crumb {
            label: "Quotations",
            href: Some(&list_url),
        },
        Crumb {
            label: &current,
            href: None,
        },
    ])
}

fn number_label(id: i64, number: &str) -> String {
    if number.trim().is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    }
}

fn render_pagination<K: SwapKey>(path_and_query: &str, number: u32, num_pages: u32) -> Markup {
    let owned = pagination_pages(path_and_query, number, num_pages, true);
    let pages: Vec<PaginationPage<'_>> = owned
        .iter()
        .map(|(ellipsis, url, push_url, active, label)| PaginationPage {
            ellipsis: *ellipsis,
            url: url.as_str(),
            push_url: *push_url,
            active: *active,
            label: label.as_str(),
        })
        .collect();
    table_pagination(TablePagination {
        pages: &pages,
        hx_target: K::SELECTOR,
    })
}

#[derive(Clone)]
pub struct QuotationRow {
    pub id: i64,
    pub number: String,
    pub date: String,
    pub customer: String,
    pub total: String,
}

#[derive(Generic)]
pub struct QuotationListPage {
    pub quotations: ObjectList<QuotationRow>,
    pub path_and_query: String,
    pub page_size: u32,
}

impl QuotationListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader {
                key: "Number",
                label: "Number",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Date",
                label: "Date",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Customer",
                label: "Customer",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Total",
                label: "Total",
                sort_url: None,
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .quotations
            .items
            .iter()
            .map(|row| TableRow {
                attrs: row_attr_navigate_route(QuotationDetailRouteTag::new(row.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &row.number,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.date,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.customer,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.total,
                        classes: "",
                    }),
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let create = if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<FinanceQuotationsMutate>(),
        ) {
            table_create_button::<QuotationTableKey, QuotationCreateModalKey>(
                Some("plus"),
                "btn-square btn-outline btn-sm",
            )
        } else {
            html! {}
        };
        let pagination = render_pagination::<QuotationTableKey>(
            &self.path_and_query,
            self.quotations.number,
            self.quotations.num_pages,
        );
        data_table_list_refresh::<QuotationTableKey>(
            "Quotations",
            create,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        container_column("", self.render_table())
    }
}

impl RenderAppPane for QuotationListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(&self.path_and_query, list_crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(list_crumbs(), self.body())
    }
}

impl RenderTemplate for QuotationListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Quotations",
            chrome,
            list_crumbs(),
            self.body(),
            &self.path_and_query,
        )
    }
}

#[derive(Generic)]
pub struct QuotationDetailPage {
    pub id: i64,
    pub number: String,
    pub date: String,
    pub customer: String,
    pub customer_href: String,
    pub bank_account: String,
    pub remarks: String,
    pub goods: Vec<InvoiceGoodsLineDisplayRow>,
    pub services: Vec<InvoiceServiceLineDisplayRow>,
    pub grand_total: String,
    pub can_edit: bool,
    pub actions_html: String,
}

impl QuotationDetailPage {
    fn menu(&self) -> Markup {
        let title = format!("Quotation {}", number_label(self.id, &self.number));
        let detail_url = QuotationDetailRouteTag::new(self.id).url();
        detail_sidebar_menu(
            title,
            &[DetailMenuNavItem {
                title: "Quotation",
                url: detail_url,
                active: true,
            }],
            None,
            html! {},
        )
    }

    fn body(&self) -> Markup {
        let title = format!("Quotation {}", number_label(self.id, &self.number));
        let actions = html! {
            (button_modal_route(
                QuotationPdfModalRouteTag::new(self.id),
                "PDF",
                "btn-outline",
            ))
            (button_download_route(
                QuotationMailRouteTag::new(self.id),
                "Email",
                "btn-outline",
            ))
            @if self.can_edit {
                (button_post_route(
                    QuotationCopyPostRouteTag::new(self.id),
                    "Copy",
                    "btn-outline",
                ))
                (button_modal_route(
                    QuotationCreateInvoiceGetRouteTag::new(self.id),
                    "Create draft invoice",
                    "btn-primary",
                ))
            }
            (PreEscaped(self.actions_html.clone()))
            @if self.can_edit {
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_quotations.QuotationEditForm",
                    href: &QuotationEditGetRouteTag::new(self.id).url(),
                    form_post_url: &QuotationEditPostRouteTag::new(self.id).path(),
                    modal_uid: QuotationEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader { title: &title, actions }))
                    (label("Number", field_text(FieldText { value: &number_label(self.id, &self.number), classes: "" })))
                    (label("Date", field_text(FieldText { value: &self.date, classes: "" })))
                    (label("Customer", customer_field(&self.customer_href, &self.customer)))
                    (label("Account", field_textarea(FieldTextarea { value: &self.bank_account, classes: "" })))
                    (label("Remarks", field_textarea(FieldTextarea { value: &self.remarks, classes: "" })))
                    (label("Grand total", field_text(FieldText { value: &self.grand_total, classes: "" })))
                    h3 class="font-semibold mt-4" { "Goods" }
                    (field_invoice_goods_lines(&self.goods))
                    h3 class="font-semibold mt-4" { "Services" }
                    (field_invoice_service_lines(&self.services))
                }))
            }))
        }
    }
}

fn customer_field(href: &str, name: &str) -> Markup {
    if !href.is_empty() {
        field_link(FieldLink {
            href,
            label: name,
            classes: "link link-hover",
        })
    } else if name.is_empty() {
        field_text(FieldText {
            value: "—",
            classes: "",
        })
    } else {
        field_text(FieldText {
            value: name,
            classes: "",
        })
    }
}

impl RenderAppPane for QuotationDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_entity_sidebar_crumbs(
            self.menu(),
            detail_crumbs(self.id, &self.number),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(detail_crumbs(self.id, &self.number), self.body())
    }
}

impl RenderTemplate for QuotationDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold_with_sidebar(
            "Quotation",
            chrome,
            self.menu(),
            detail_crumbs(self.id, &self.number),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct QuotationCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub fields: QuotationFormFields,
    pub error: String,
}

impl RenderTemplate for QuotationCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let post_url = modal_create_post_query(
            QuotationCreatePostRouteTag,
            &self.form_name,
            &self.refresh_table,
            &self.target_input,
        );
        modal_keyed::<QuotationCreateModalKey>(
            "!max-w-6xl w-full",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "New quotation",
                    subtitle: "Create a quotation",
                    classes: "@container",
                    attrs: form_hx_post_url::<QuotationCreateModalKey>(&post_url),
                    form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                    inputs: quotation_inputs(&self.fields),
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

#[derive(Generic)]
pub struct QuotationEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub number_label: String,
    pub fields: QuotationFormFields,
    pub error: String,
}

impl RenderTemplate for QuotationEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let post_url =
            modal_edit_post_url(QuotationEditPostRouteTag::new(self.id), &self.form_name);
        let delete_url = QuotationDeleteGetRouteTag::new(self.id).url();
        modal_keyed::<QuotationEditModalKey>(
            "!max-w-6xl w-full",
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit " (number_label(self.id, &self.number_label)) }
                (form(&CsrfToken::current(), FormOpts {
                    classes: "@container",
                    attrs: form_hx_post_url::<QuotationEditModalKey>(&post_url),
                    form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                    inputs: quotation_inputs(&self.fields),
                    actions: html! {
                        (button_submit(ButtonSubmit {
                            label: "Save",
                            ..Default::default()
                        }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            href: &delete_url,
                            name: "p_finance_quotations.QuotationDeleteForm",
                            form_post_url: &QuotationDeletePostRouteTag::new(self.id).path(),
                            modal_uid: QuotationDeleteModalKey::ID,
                            icon_name: Some("trash"),
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
pub struct ConfirmDeletePage {
    pub id: i64,
    pub form_name: String,
    pub message: String,
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let post_url = QuotationDeletePostRouteTag::new(self.id).url();
        let target = format!("#{}", QuotationDeleteModalKey::ID);
        modal_keyed::<QuotationDeleteModalKey>(
            "!max-w-lg",
            delete_confirmation(DeleteConfirmation {
                title: "Confirm deletion",
                message: &self.message,
                attrs: form_hx_post_selector(&post_url, &target),
                form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                ..Default::default()
            }),
        )
    }
}

#[derive(Generic)]
pub struct ConfirmInvoicePage {
    pub id: i64,
    pub number: String,
    pub error: String,
}

impl RenderTemplate for ConfirmInvoicePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let post_url = QuotationCreateInvoicePostRouteTag::new(self.id).url();
        let target = format!("#{}", QuotationInvoiceModalKey::ID);
        let label = number_label(self.id, &self.number);
        let message = format!(
            "Create a draft invoice from quotation {label}. Lines, rates, and taxes are copied. The invoice uses a single payment of the full amount due in 15 days."
        );
        modal_keyed::<QuotationInvoiceModalKey>(
            "!max-w-lg",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create draft invoice",
                    subtitle: &message,
                    attrs: form_hx_post_selector(&post_url, &target),
                    form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                    inputs: html! {},
                    actions: html! {
                        (container_row("flex justify-end gap-2 mt-2", html! {
                            (button_submit(ButtonSubmit {
                                label: "Create draft",
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

lariv_core::define_register_items! {
    plugin: crate::FinanceQuotationsTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        QuotationListIdx: QuotationListPageTag => QuotationListPage,
        QuotationDetailIdx: QuotationDetailPageTag => QuotationDetailPage,
        QuotationEditModalIdx: QuotationEditModalPageTag => QuotationEditModalPage,
        QuotationCreateModalIdx: QuotationCreateModalPageTag => QuotationCreateModalPage,
        QuotationDeleteIdx: QuotationConfirmDeletePageTag => ConfirmDeletePage,
        QuotationInvoiceIdx: QuotationConfirmInvoicePageTag => ConfirmInvoicePage,
    ]
}

lariv_core::define_register_items! {
    plugin: crate::FinanceQuotationsTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}
