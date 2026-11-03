use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonClear, ButtonModalForm, ButtonSubmit, Crumb, FieldLink, FieldText, FieldTitle, FormOpts,
    ManyToManyItem, ObjectList, PaginationPage, ShellChrome, SlotCapability, SlotRegistrar,
    SwapKey, TableButtonFilter, TableColumnHeader, TableRow, breadcrumbs, button_clear,
    button_modal_form, button_submit, column_sort_url, container_column, container_row,
    data_table_list_refresh, detail, field_link, field_text, field_title, form, form_hx_get_url,
    form_hx_post_url, label, list_filter_common_inputs, page_size_only_filter_form,
    pagination_pages, row_attr_navigate, sort_indicator, table_button_filter,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::http::{ProvideRequestCaps, RouteQueryBuilder};
use lariv_core::template::{
    RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar,
};
use lariv_core::web::modal_create_post_url;

use lariv_plugin_finance_accounts::accounting_detail_menu::{
    DetailMenuNavItem, detail_sidebar_menu,
};
use lariv_plugin_finance_accounts::routes::JournalEntryDetailRouteTag;
use lariv_plugin_finance_accounts::templates::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs,
};

use super::forms::{PaymentForm, PaymentFormField};
use super::keys::PAYMENT_CREATE_MODAL;
use super::routes::{
    PaymentCreateGetRouteTag, PaymentCreatePostRouteTag, PaymentDetailRouteTag, PaymentListRouteTag,
};

fn payments_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Payments",
        href: None,
    }])
}

fn payment_crumbs(label: &str) -> Markup {
    let list_url = PaymentListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Payments",
            href: Some(&list_url),
        },
        Crumb { label, href: None },
    ])
}

fn payment_detail_menu(id: i64) -> Markup {
    detail_sidebar_menu(
        format!("Payment #{id}"),
        &[DetailMenuNavItem {
            title: "Payment Detail",
            url: PaymentDetailRouteTag::new(id).url(),
            active: true,
        }],
        None,
        html! {},
    )
}

fn render_pagination(
    key: SwapKey,
    path_and_query: &str,
    number: u32,
    num_pages: u32,
    push: bool,
) -> Markup {
    let hx_target = key.selector();
    let owned = pagination_pages(path_and_query, number, num_pages, push);
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
    lariv_core::components::table_pagination(lariv_core::components::TablePagination {
        pages: &pages,
        hx_target: &hx_target,
    })
}

fn journal_entry_link(id: i64) -> Markup {
    if id > 0 {
        field_link(FieldLink {
            href: &JournalEntryDetailRouteTag::new(id).url(),
            label: &format!("Entry #{id}"),
            classes: "link link-hover",
        })
    } else {
        field_text(FieldText {
            value: "—",
            classes: "",
        })
    }
}

fn optional_link(href: &Option<String>, label: &str) -> Markup {
    if let Some(href) = href
        && !label.is_empty()
    {
        return field_link(FieldLink {
            href,
            label,
            classes: "link link-hover",
        });
    }
    field_text(FieldText {
        value: if label.is_empty() { "—" } else { label },
        classes: "",
    })
}

lariv_core::define_register_items! {
    plugin: FinancePaymentsTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        PaymentListIdx: PaymentListPageTag => PaymentListPage,
        PaymentCreateModalIdx: PaymentCreateModalPageTag => PaymentCreateModalPage,
        PaymentDetailIdx: PaymentDetailPageTag => PaymentDetailPage,
    ]
}

lariv_core::define_register_items! {
    plugin: FinancePaymentsTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

#[derive(Clone)]
pub struct PaymentRow {
    pub id: i64,
    pub customer_label: String,
    pub amount: String,
    pub datetime: String,
}

#[derive(Generic)]
pub struct PaymentListPage {
    pub payments: ObjectList<PaymentRow>,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
    pub page_size: u32,
    pub table_key: SwapKey,
    /// Filter target. Empty uses the payments list route.
    pub filter_url: String,
    /// When set, the create button opens with this customer filled in.
    pub create_customer_id: Option<i64>,
}

fn payment_create_href(customer_id: Option<i64>) -> String {
    match customer_id.filter(|id| *id > 0) {
        Some(id) => RouteQueryBuilder::new(PaymentCreateGetRouteTag)
            .query("CustomerID", id)
            .build(),
        None => PaymentCreateGetRouteTag.url(),
    }
}

fn payment_filter_panel(page: &PaymentListPage) -> Markup {
    if page.filter_url.is_empty() {
        return page_size_only_filter_form::<PaymentListRouteTag>(page.page_size);
    }
    form(
        &CsrfToken::current(),
        FormOpts {
            attrs: form_hx_get_url(page.table_key, &page.filter_url).set("hx-push-url", "false"),
            inputs: list_filter_common_inputs(page.page_size),
            actions: container_row(
                "flex gap-2",
                html! {
                    (button_submit(ButtonSubmit { label: "Apply Filters", ..Default::default() }))
                    (button_clear(ButtonClear { label: "Clear", ..Default::default() }))
                },
            ),
            ..Default::default()
        },
    )
}

impl PaymentListPage {
    pub fn render_table(&self) -> Markup {
        let amount_sort = column_sort_url(&self.path_and_query, "Amount", &self.sort);
        let date_sort = column_sort_url(&self.path_and_query, "Date", &self.sort);
        let amount_label = format!("Amount{}", sort_indicator(&self.sort, "Amount"));
        let date_label = format!("Date{}", sort_indicator(&self.sort, "Date"));
        let push_history = self.filter_url.is_empty();
        let headers = [
            TableColumnHeader {
                key: "Customer",
                label: "Customer",
                sort_url: None,
                push_url: push_history,
            },
            TableColumnHeader {
                key: "Amount",
                label: &amount_label,
                sort_url: Some(&amount_sort),
                push_url: push_history,
            },
            TableColumnHeader {
                key: "Date",
                label: &date_label,
                sort_url: Some(&date_sort),
                push_url: push_history,
            },
        ];
        let rows: Vec<TableRow> = self
            .payments
            .items
            .iter()
            .map(|p| TableRow {
                attrs: row_attr_navigate(&PaymentDetailRouteTag::new(p.id).url()),
                cells: vec![
                    field_text(FieldText {
                        value: &p.customer_label,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &p.amount,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &p.datetime,
                        classes: "",
                    }),
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let pagination = render_pagination(
            self.table_key,
            &self.path_and_query,
            self.payments.number,
            self.payments.num_pages,
            self.filter_url.is_empty(),
        );
        let page_size_filter = table_button_filter(TableButtonFilter {
            panel: payment_filter_panel(self),
            ..Default::default()
        });
        let create_href = payment_create_href(self.create_customer_id);
        let actions = if self.can_edit {
            html! {
                (page_size_filter)
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_payments.PaymentCreateForm",
                    href: &create_href,
                    form_post_url: &PaymentCreateGetRouteTag.path(),
                    modal_uid: PAYMENT_CREATE_MODAL.id(),
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            }
        } else {
            page_size_filter
        };
        data_table_list_refresh(
            self.table_key,
            "Payments",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        html! {
            (container_column("", self.render_table()))
        }
    }
}

impl RenderAppPane for PaymentListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(&self.path_and_query, payments_list_crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(payments_list_crumbs(), self.body())
    }
}

impl RenderTemplate for PaymentListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Payments",
            chrome,
            payments_list_crumbs(),
            self.body(),
            &self.path_and_query,
        )
    }
}

#[derive(Generic)]
pub struct PaymentCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub form: PaymentForm,
    pub customer_display: String,
    pub tax_items: Vec<ManyToManyItem>,
    pub error: String,
}

impl RenderTemplate for PaymentCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_payments.PaymentCreateForm"
        } else {
            self.form_name.as_str()
        };
        lariv_core::components::modal_keyed(
            PAYMENT_CREATE_MODAL,
            "!max-w-6xl w-full",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Record payment",
                    subtitle: "Record a payment from a customer",
                    classes: "@container",
                    attrs: form_hx_post_url(
                        PAYMENT_CREATE_MODAL,
                        &modal_create_post_url(
                            PaymentCreatePostRouteTag,
                            form_name,
                            &self.refresh_table,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: PaymentForm::render_inputs(
                        &FormCtx::form::<PaymentForm>(CsrfToken::current())
                            .value(
                                PaymentFormField::CustomerId,
                                self.form.customer_id.to_string(),
                            )
                            .display(PaymentFormField::CustomerId, &self.customer_display)
                            .value(PaymentFormField::Amount, &self.form.amount)
                            .value(PaymentFormField::Datetime, &self.form.datetime)
                            .value(PaymentFormField::JournalLines, &self.form.journal_lines)
                            .m2m(PaymentFormField::Taxes, &self.tax_items),
                    ),
                    actions: html! {
                        (lariv_core::components::container_row("flex justify-end gap-2 mt-2", html! {
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
pub struct PaymentDetailPage {
    pub id: i64,
    pub customer_label: String,
    pub customer_href: Option<String>,
    pub amount: String,
    pub tax_labels: String,
    pub datetime: String,
    pub journal_entry_id: i64,
}

impl PaymentDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &format!("Payment #{}", self.id), classes: "" }))
                    (label("Customer", optional_link(&self.customer_href, &self.customer_label)))
                    (label("Amount", field_text(FieldText { value: &self.amount, classes: "" })))
                    (label("Withholding taxes", field_text(FieldText { value: &self.tax_labels, classes: "" })))
                    (label("Datetime", field_text(FieldText { value: &self.datetime, classes: "" })))
                    (label("Journal entry", journal_entry_link(self.journal_entry_id)))
                }))
            }))
        }
    }
}

impl RenderAppPane for PaymentDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let label = format!("#{}", self.id);
        layout_with_entity_sidebar_crumbs(
            payment_detail_menu(self.id),
            payment_crumbs(&label),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        let label = format!("#{}", self.id);
        layout_main_with_crumbs(payment_crumbs(&label), self.body())
    }
}

impl RenderTemplate for PaymentDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = format!("#{}", self.id);
        app_scaffold_with_sidebar(
            "Payment",
            chrome,
            payment_detail_menu(self.id),
            payment_crumbs(&label),
            self.body(),
        )
    }
}
