use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_core::components::table::table_grid_content;
use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, Crumb, DataTable, DataTableDisplay, DeleteConfirmation,
    DetailHeader, FieldLink, FieldText, FieldTextarea, FilterClauseView, FormOpts,
    ListFilterBuilder, ObjectList, PaginationPage, ShellChrome, SlotCapability, SlotRegistrar,
    SwapKey, TableButtonFilter, TableColumnHeader, TableListContent, TablePagination, TableRow,
    breadcrumbs, button_download_route, button_modal_form, button_modal_route, button_post_route,
    button_submit, container_column, container_row, data_table, delete_confirmation, detail,
    detail_header, field_link, field_text, field_textarea, filter_error_banner, form,
    form_hx_get_route, form_hx_post_selector, form_hx_post_url, label, list_filter_builder,
    modal_keyed, pagination_pages, row_attr_navigate_route, table_button_columns,
    table_button_filter, table_create_button, table_list_content, table_pagination,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::{
    RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar,
};
use lariv_core::web::{modal_create_post_query, modal_edit_post_url};
use lariv_plugin_finance_invoices::components::field_invoice_lines;
use lariv_plugin_finance_invoices::logic::invoice_line_editor::InvoiceLineDisplayRow;

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

fn choice_pairs(choices: &[(&str, &str)]) -> Vec<(String, String)> {
    choices
        .iter()
        .map(|(key, label)| ((*key).to_string(), (*label).to_string()))
        .collect()
}

pub struct QuotationFormFields {
    pub number: String,
    pub date: String,
    pub status: String,
    pub customer_id: String,
    pub customer_display: String,
    pub bank_account: String,
    pub remarks: String,
    pub lines_json: String,
    pub lines_preview: String,
}

fn quotation_inputs(fields: &QuotationFormFields) -> Markup {
    let status_choices = choice_pairs(QuotationForm::quotation_status_choices());
    QuotationForm::render_inputs(
        &FormCtx::form::<QuotationForm>(CsrfToken::current())
            .value(QuotationFormField::Number, &fields.number)
            .value(QuotationFormField::Date, &fields.date)
            .value(QuotationFormField::Status, &fields.status)
            .choices(QuotationFormField::Status, &status_choices)
            .value(QuotationFormField::CustomerId, &fields.customer_id)
            .display(QuotationFormField::CustomerId, &fields.customer_display)
            .value(QuotationFormField::BankAccount, &fields.bank_account)
            .value(QuotationFormField::Remarks, &fields.remarks)
            .value(QuotationFormField::LinesJson, &fields.lines_json)
            .display(QuotationFormField::LinesJson, &fields.lines_preview),
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

fn data_table_with_filter_errors<K: SwapKey>(
    title: &str,
    actions: Markup,
    headers: &[TableColumnHeader<'_>],
    rows: &[TableRow],
    pagination: Markup,
    refresh_url: &str,
    errors: &[FilterRowError],
) -> Markup {
    let list = html! {
        (filter_error_banner(errors))
        (table_list_content(TableListContent {
            headers,
            rows,
            hx_target: K::SELECTOR,
        }))
    };
    let grid = html! {
        (filter_error_banner(errors))
        (table_grid_content(headers, rows))
    };
    let column_keys: Vec<&str> = headers
        .iter()
        .map(|header| header.key)
        .filter(|key| !key.is_empty())
        .collect();
    let toolbar = if headers.len() > 1 {
        html! {
            (table_button_columns(headers))
            (actions)
        }
    } else {
        actions
    };
    data_table(DataTable {
        uid: K::ID,
        title,
        classes: "w-full",
        actions: toolbar,
        displays: vec![
            DataTableDisplay {
                name: "Grid".into(),
                html: grid,
            },
            DataTableDisplay {
                name: "List".into(),
                html: list,
            },
        ],
        pagination,
        refresh_url,
        column_keys: &column_keys,
        ..Default::default()
    })
}

fn quotation_list_filter(page: &QuotationListPage) -> Markup {
    list_filter_builder(ListFilterBuilder {
        fields: &page.fields,
        rows: &page.filters,
        page_size: page.page_size,
        sort: "",
        attrs: form_hx_get_route::<QuotationTableKey, QuotationDefaultRouteTag>(
            QuotationDefaultRouteTag,
        ),
        apply_label: "Apply Filters",
        extra: html! {},
    })
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
    pub status: String,
    pub customer: String,
    pub total: String,
}

#[derive(Generic)]
pub struct QuotationListPage {
    pub quotations: ObjectList<QuotationRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
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
                key: "Status",
                label: "Status",
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
                        value: &row.status,
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
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: quotation_list_filter(self),
                ..Default::default()
            }))
        };
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<FinanceQuotationsMutate>(),
        ) {
            actions = html! {
                (actions)
                (table_create_button::<QuotationTableKey, QuotationCreateModalKey>(
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        let pagination = render_pagination::<QuotationTableKey>(
            &self.path_and_query,
            self.quotations.number,
            self.quotations.num_pages,
        );
        data_table_with_filter_errors::<QuotationTableKey>(
            "Quotations",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
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
    pub status: String,
    pub customer: String,
    pub customer_href: String,
    pub bank_account: String,
    pub remarks: String,
    pub lines: Vec<InvoiceLineDisplayRow>,
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
                    (label("Status", field_text(FieldText { value: &self.status, classes: "" })))
                    (label("Customer", customer_field(&self.customer_href, &self.customer)))
                    (label("Account", field_textarea(FieldTextarea { value: &self.bank_account, classes: "" })))
                    (label("Remarks", field_textarea(FieldTextarea { value: &self.remarks, classes: "" })))
                    (label("Grand total", field_text(FieldText { value: &self.grand_total, classes: "" })))
                    h3 class="font-semibold mt-4" { "Lines" }
                    (field_invoice_lines(&self.lines))
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
