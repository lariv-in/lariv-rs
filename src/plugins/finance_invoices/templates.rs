use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use crate::plugins::customer::routes::CustomerDetailRouteTag;
use crate::plugins::finance_accounts::routes::JournalEntryDetailRouteTag;

use crate::components::{
    ButtonDeletePost, ButtonModalForm, ButtonSubmit, Crumb, DeleteConfirmation, DetailHeader,
    FieldLink, FieldText, FieldTitle, FormOpts, ManyToManyItem, ObjectList, PaginationPage,
    ShellChrome, SlotCapability, SlotRegistrar, SwapKey, TableColumnHeader, TablePagination,
    TableRow, breadcrumbs, button_delete_post_route, button_modal_form, button_modal_route,
    button_submit, column_sort_url, container_column, container_row, data_table_list_refresh,
    delete_confirmation, detail, detail_header, field_link, field_text, field_title, form,
    form_hx_post_main_url, form_hx_post_selector, form_hx_post_url, label, modal, modal_keyed,
    pagination_pages, row_attr_navigate, row_attr_select, row_attr_select_multi, sort_indicator,
    table_pagination,
};
use crate::{
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    picker::RenderPickerSelect,
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{modal_create_post_url, modal_edit_post_url},
};

use crate::plugins::finance_accounts::accounting_detail_menu::{
    DetailMenuNavItem, detail_sidebar_menu,
};
use crate::plugins::finance_accounts::templates::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs,
};

use crate::plugins::finance_invoices::components::{
    self, field_invoice_lines, fiscal_year_environment_selector,
};
use crate::plugins::finance_invoices::logic::PaymentTermLineDisplayRow;
use crate::plugins::finance_invoices::logic::invoice_line_editor::InvoiceLineDisplayRow;

use super::forms::{
    CancelInvoiceForm, CancelInvoiceFormField, DraftInvoiceBulkEditForm,
    DraftInvoiceBulkEditFormField, DraftInvoiceForm, DraftInvoiceFormField, InvoicePreferencesForm,
    InvoicePreferencesFormField, PaymentBatchForm, PaymentBatchFormField, PaymentForm,
    PaymentFormField, PaymentPreferencesForm, PaymentPreferencesFormField,
};
use super::keys::{
    DraftInvoiceBulkDeleteModalKey, DraftInvoiceBulkEditModalKey, DraftInvoiceCreateModalKey,
    DraftInvoiceDeleteModalKey, DraftInvoiceEditModalKey, DraftInvoiceSelectModalKey,
    DraftInvoiceSelectTableKey, InvoiceHubTableKey, PaymentBatchCreateModalKey,
    PaymentCreateModalKey, PaymentTableKey, PostedInvoiceSelectModalKey,
    PostedInvoiceSelectTableKey,
};
use super::routes::{
    CancelledInvoiceDetailRouteTag, CancelledInvoiceNewDraftRouteTag,
    CancelledInvoicePdfModalRouteTag, DraftInvoiceBulkDeletePostRouteTag,
    DraftInvoiceBulkEditPostRouteTag, DraftInvoiceCreateGetRouteTag,
    DraftInvoiceCreatePostRouteTag, DraftInvoiceDeleteGetRouteTag, DraftInvoiceDeletePostRouteTag,
    DraftInvoiceDetailRouteTag, DraftInvoiceEditGetRouteTag, DraftInvoiceEditPostRouteTag,
    DraftInvoicePdfModalRouteTag, DraftInvoicePostRouteTag, InvoiceDefaultRouteTag,
    InvoicePreferencesRouteTag, PaidInvoiceDetailRouteTag, PaidInvoicePdfModalRouteTag,
    PartiallyPaidInvoiceDetailRouteTag, PartiallyPaidInvoicePdfModalRouteTag,
    PaymentBatchCreatePostRouteTag, PaymentBatchDetailRouteTag, PaymentCreateGetRouteTag,
    PaymentCreatePostRouteTag, PaymentDetailRouteTag, PaymentListRouteTag,
    PaymentPreferencesRouteTag, PostedInvoiceBulkCancelPostRouteTag,
    PostedInvoiceCancelGetRouteTag, PostedInvoiceDetailRouteTag, PostedInvoicePdfModalRouteTag,
};

crate::define_register_items! {
    plugin: FinanceInvoicesTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        InvoiceHubIdx: InvoiceHubPageTag => InvoiceHubPage,
        DraftInvoiceEditModalIdx: DraftInvoiceEditModalPageTag => DraftInvoiceEditModalPage,
        DraftInvoiceCreateModalIdx: DraftInvoiceCreateModalPageTag => DraftInvoiceCreateModalPage,
        DraftInvoiceBulkEditModalIdx: DraftInvoiceBulkEditModalPageTag => DraftInvoiceBulkEditModalPage,
        DraftInvoiceDetailIdx: DraftInvoiceDetailPageTag => DraftInvoiceDetailPage,
        DraftInvoiceSelectIdx: DraftInvoiceSelectPageTag => DraftInvoiceSelectPage,
        PostedInvoiceDetailIdx: PostedInvoiceDetailPageTag => PostedInvoiceDetailPage,
        PaidInvoiceDetailIdx: PaidInvoiceDetailPageTag => PaidInvoiceDetailPage,
        PartiallyPaidInvoiceDetailIdx: PartiallyPaidInvoiceDetailPageTag => PartiallyPaidInvoiceDetailPage,
        CancelledInvoiceDetailIdx: CancelledInvoiceDetailPageTag => CancelledInvoiceDetailPage,
        PaymentListIdx: PaymentListPageTag => PaymentListPage,
        PaymentCreateModalIdx: PaymentCreateModalPageTag => PaymentCreateModalPage,
        PaymentDetailIdx: PaymentDetailPageTag => PaymentDetailPage,
        PaymentBatchCreateModalIdx: PaymentBatchCreateModalPageTag => PaymentBatchCreateModalPage,
        PaymentBatchDetailIdx: PaymentBatchDetailPageTag => PaymentBatchDetailPage,
        CancelInvoiceIdx: CancelInvoicePageTag => CancelInvoicePage,
        CancelBulkInvoiceIdx: CancelBulkInvoicePageTag => CancelBulkInvoicePage,
        InvoicePreferencesIdx: InvoicePreferencesPageTag => InvoicePreferencesPage,
        PaymentPreferencesIdx: PaymentPreferencesPageTag => PaymentPreferencesPage,
        ConfirmDeleteIdx: DraftInvoiceConfirmDeletePageTag => ConfirmDeletePage,
        ConfirmBulkDeleteIdx: DraftInvoiceConfirmBulkDeletePageTag => ConfirmBulkDeletePage,
    ]
}

crate::define_register_items! {
    plugin: FinanceInvoicesTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
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

fn invoices_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Invoices",
        href: None,
    }])
}

fn payments_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Payments",
        href: None,
    }])
}

fn field_payment_term_schedule(rows: &[PaymentTermLineDisplayRow]) -> Markup {
    if rows.is_empty() {
        return html! { p { "—" } };
    }
    html! {
        table class="table table-sm w-full max-w-lg [&_th]:pl-0 [&_td]:pl-0" {
            thead {
                tr {
                    th class="text-xs" { "Due" }
                    th class="text-xs" { "Amount" }
                }
            }
            tbody {
                @for row in rows {
                    tr {
                        td { (row.due_display) }
                        td { (row.amount_display) }
                    }
                }
            }
        }
    }
}

fn invoice_preferences_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Invoice preferences",
        href: None,
    }])
}

fn payment_preferences_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Payment preferences",
        href: None,
    }])
}

fn invoice_number_label(id: i64, number: &str) -> String {
    if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    }
}

fn draft_invoice_label(id: i64, number: &str) -> String {
    if number.is_empty() {
        format!("Draft #{id}")
    } else {
        format!("Draft {number}")
    }
}

fn invoice_section_crumbs(label: &str, detail_url: &str, action: Option<&str>) -> Markup {
    let list_url = InvoiceDefaultRouteTag.url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Invoices",
                href: Some(&list_url),
            },
            Crumb {
                label: label,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Invoices",
                href: Some(&list_url),
            },
            Crumb {
                label: label,
                href: Some(detail_url),
            },
            Crumb {
                label: act,
                href: None,
            },
        ]),
    }
}

fn payment_crumbs(label: &str) -> Markup {
    let list_url = PaymentListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Payments",
            href: Some(&list_url),
        },
        Crumb {
            label: label,
            href: None,
        },
    ])
}

fn payment_batch_crumbs(label: &str) -> Markup {
    let list_url = payment_tab_href("batches");
    breadcrumbs(&[
        Crumb {
            label: "Payments",
            href: Some(&list_url),
        },
        Crumb {
            label: label,
            href: None,
        },
    ])
}

fn tab_href(tab: &str) -> String {
    crate::http::RouteQueryBuilder::new(InvoiceDefaultRouteTag)
        .query("tab", tab)
        .build()
}

fn payment_tab_href(tab: &str) -> String {
    crate::http::RouteQueryBuilder::new(PaymentListRouteTag)
        .query("tab", tab)
        .build()
}

fn draft_invoice_detail_menu(id: i64, number: &str) -> Markup {
    let label = if number.is_empty() {
        format!("Draft #{id}")
    } else {
        format!("Draft {number}")
    };
    let detail_url = DraftInvoiceDetailRouteTag::new(id).url();
    let nav = vec![DetailMenuNavItem {
        title: "Draft Invoice Detail",
        url: detail_url,
        active: true,
    }];
    detail_sidebar_menu(format!("Invoice: {label}"), &nav, None, html! {})
}

fn posted_invoice_detail_menu(id: i64, number: &str) -> Markup {
    let label = if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    };
    detail_sidebar_menu(
        format!("Posted invoice: {label}"),
        &[DetailMenuNavItem {
            title: "Posted Invoice Detail",
            url: PostedInvoiceDetailRouteTag::new(id).url(),
            active: true,
        }],
        None,
        html! {},
    )
}

fn cancelled_invoice_detail_menu(id: i64, number: &str) -> Markup {
    let label = if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    };
    detail_sidebar_menu(
        format!("Cancelled invoice: {label}"),
        &[DetailMenuNavItem {
            title: "Cancelled Invoice Detail",
            url: CancelledInvoiceDetailRouteTag::new(id).url(),
            active: true,
        }],
        None,
        html! {},
    )
}

fn paid_invoice_detail_menu(id: i64, number: &str) -> Markup {
    let label = if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    };
    detail_sidebar_menu(
        format!("Paid invoice: {label}"),
        &[DetailMenuNavItem {
            title: "Paid Invoice Detail",
            url: PaidInvoiceDetailRouteTag::new(id).url(),
            active: true,
        }],
        None,
        html! {},
    )
}

fn partially_paid_invoice_detail_menu(id: i64, number: &str) -> Markup {
    let label = if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    };
    detail_sidebar_menu(
        format!("Partially paid invoice: {label}"),
        &[DetailMenuNavItem {
            title: "Partially Paid Invoice Detail",
            url: PartiallyPaidInvoiceDetailRouteTag::new(id).url(),
            active: true,
        }],
        None,
        html! {},
    )
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

fn payment_batch_detail_menu(id: i64) -> Markup {
    detail_sidebar_menu(
        format!("Batch #{id}"),
        &[DetailMenuNavItem {
            title: "Batch Detail",
            url: PaymentBatchDetailRouteTag::new(id).url(),
            active: true,
        }],
        None,
        html! {},
    )
}

#[derive(Clone)]
pub struct InvoiceRow {
    pub id: i64,
    /// Draft invoice id when the row can be linked to site (and other) addons.
    pub draft_invoice_id: Option<i64>,
    pub number: String,
    pub datetime: String,
    pub delivery_date: String,
    pub detail_href: String,
    pub customer_name: String,
    pub open_balance: String,
    pub selectable: bool,
    pub untaxed_amount: String,
    pub total_amount: String,
    pub tax_levied: String,
    pub product_count: String,
    pub final_due_date: String,
    /// Cell values for registered [`super::hub_table_addon`] columns, in order.
    pub extra_cells: Vec<String>,
}

#[derive(Generic)]
pub struct InvoiceHubPage {
    pub invoices: ObjectList<InvoiceRow>,
    pub tab: String,
    pub sort: String,
    pub path_and_query: String,
    pub fiscal_years: Vec<components::FiscalYearOption>,
    pub selected_fiscal_year_start: Option<i32>,
    pub can_edit: bool,
    pub extra_columns: Vec<super::hub_table_addon::InvoiceHubExtraColumn>,
}

impl InvoiceHubPage {
    fn tab_link(&self, tab: &str, label: &str) -> Markup {
        use crate::components::attrs::escape_attr;
        use maud::PreEscaped;

        let active = self.tab == tab;
        let cls = if active { "tab tab-active" } else { "tab" };
        let href = tab_href(tab);
        let nav = crate::components::nav_content_attrs(&href);
        html! {
            (PreEscaped(format!(
                r#"<a class="{cls}" href="{href}"{attrs}>"#,
                cls = escape_attr(cls),
                href = escape_attr(&href),
                attrs = nav.as_string(),
            )))
            (label)
            (PreEscaped("</a>"))
        }
    }

    fn drafts_hub(&self) -> bool {
        self.tab == "drafts"
    }

    fn posted_hub(&self) -> bool {
        self.tab == "posted"
    }

    fn cancelled_hub(&self) -> bool {
        self.tab == "cancelled"
    }

    /// Selection is needed for hub bulk actions (including PDF zip download on every tab).
    fn show_select(&self) -> bool {
        self.can_edit
    }

    /// Alpine helpers on the selection root (outside the swapped table).
    fn selection_root_js() -> &'static str {
        "Alpine.$data($el.closest('[data-invoice-hub-selection]'))"
    }

    fn selection_x_data() -> &'static str {
        r#"{
            selected: {},
            toggle(id) {
                const k = String(id);
                if (this.selected[k]) delete this.selected[k];
                else this.selected[k] = true;
            },
            selectedIds() {
                return Object.keys(this.selected).filter(k => this.selected[k]);
            },
            paySelectedHref() {
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '/finance-invoices/payments/batch/create/?PostedInvoiceIDs=' + ids.join(',');
            },
            bulkDeleteHref() {
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '/finance-invoices/bulk-delete/?ids=' + ids.join(',');
            },
            bulkEditHref() {
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '/finance-invoices/bulk-edit/?ids=' + ids.join(',') + '&refresh=invoice-hub-table';
            },
            bulkPostHref() {
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '/finance-invoices/bulk-post/?ids=' + ids.join(',');
            },
            bulkCancelHref() {
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '/finance-invoices/bulk-cancel/?ids=' + ids.join(',');
            },
            bulkNewDraftHref() {
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '/finance-invoices/cancelled/bulk-new-draft/?ids=' + ids.join(',');
            },
            bulkDownloadPdfsHref() {
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                const params = new URLSearchParams(window.location.search);
                const tab = params.get('tab') || 'drafts';
                return '/finance-invoices/bulk-pdfs/?tab=' + encodeURIComponent(tab) + '&ids=' + ids.join(',');
            },
            requestPaySelected(el) {
                const href = this.paySelectedHref();
                if (href === '#' || typeof htmx === 'undefined') return;
                htmx.ajax('GET', href, { target: 'body', swap: 'beforeend', source: el });
            },
            requestBulkDelete(el) {
                const href = this.bulkDeleteHref();
                if (href === '#' || typeof htmx === 'undefined') return;
                htmx.ajax('GET', href, { target: 'body', swap: 'beforeend', source: el });
            },
            requestBulkEdit(el) {
                const href = this.bulkEditHref();
                if (href === '#' || typeof htmx === 'undefined') return;
                htmx.ajax('GET', href, { target: 'body', swap: 'beforeend', source: el });
            },
            requestBulkPost(el) {
                const href = this.bulkPostHref();
                if (href === '#' || typeof htmx === 'undefined') return;
                if (!confirm('Post selected draft invoices? This will create posted invoices.')) return;
                htmx.ajax('POST', href, {
                    target: '#app-layout',
                    select: '#app-layout',
                    swap: 'outerHTML',
                    push: true,
                    source: el,
                });
            },
            requestBulkCancel(el) {
                const href = this.bulkCancelHref();
                if (href === '#' || typeof htmx === 'undefined') return;
                htmx.ajax('GET', href, {
                    target: '#app-layout',
                    select: '#app-layout',
                    swap: 'outerHTML',
                    push: true,
                    source: el,
                });
            },
            requestBulkNewDraft(el) {
                const href = this.bulkNewDraftHref();
                if (href === '#' || typeof htmx === 'undefined') return;
                if (!confirm('Create new draft invoices from the selected cancelled invoices? The cancelled records will be unchanged.')) return;
                htmx.ajax('POST', href, {
                    target: '#app-layout',
                    select: '#app-layout',
                    swap: 'outerHTML',
                    push: true,
                    source: el,
                });
            },
            requestBulkDownloadPdfs() {
                const href = this.bulkDownloadPdfsHref();
                if (href === '#') return;
                window.location.assign(href);
            }
        }"#
    }

    fn wrap_with_selection(&self, table: Markup) -> Markup {
        html! {
            (PreEscaped(format!(
                r#"<div data-invoice-hub-selection x-data="{}">"#,
                crate::components::attrs::escape_attr(Self::selection_x_data()),
            )))
            (table)
            (PreEscaped("</div>"))
        }
    }

    pub fn render_table(&self) -> Markup {
        let posted_hub = self.posted_hub();
        let drafts_hub = self.drafts_hub();
        let cancelled_hub = self.cancelled_hub();
        let show_select = self.show_select();
        let sel = Self::selection_root_js();

        let id_sort = column_sort_url(&self.path_and_query, "ID", &self.sort);
        let number_sort = column_sort_url(&self.path_and_query, "Number", &self.sort);
        let date_sort = column_sort_url(&self.path_and_query, "Date", &self.sort);
        let delivery_date_sort = column_sort_url(&self.path_and_query, "DeliveryDate", &self.sort);
        let customer_sort = column_sort_url(&self.path_and_query, "Customer", &self.sort);
        let open_balance_sort = column_sort_url(&self.path_and_query, "OpenBalance", &self.sort);
        let untaxed_sort = column_sort_url(&self.path_and_query, "UntaxedAmount", &self.sort);
        let total_sort = column_sort_url(&self.path_and_query, "TotalAmount", &self.sort);
        let tax_sort = column_sort_url(&self.path_and_query, "TaxLevied", &self.sort);
        let product_count_sort = column_sort_url(&self.path_and_query, "ProductCount", &self.sort);
        let final_due_sort = column_sort_url(&self.path_and_query, "FinalDueDate", &self.sort);
        let id_label = format!("ID{}", sort_indicator(&self.sort, "ID"));
        let number_label = format!("Number{}", sort_indicator(&self.sort, "Number"));
        let date_label = format!("Date{}", sort_indicator(&self.sort, "Date"));
        let delivery_date_label = format!(
            "Delivery date{}",
            sort_indicator(&self.sort, "DeliveryDate")
        );
        let customer_label = format!("Customer{}", sort_indicator(&self.sort, "Customer"));
        let open_balance_label =
            format!("Open balance{}", sort_indicator(&self.sort, "OpenBalance"));
        let untaxed_label = format!(
            "Untaxed amount{}",
            sort_indicator(&self.sort, "UntaxedAmount")
        );
        let total_label = format!("Total amount{}", sort_indicator(&self.sort, "TotalAmount"));
        let tax_label = format!("Tax levied{}", sort_indicator(&self.sort, "TaxLevied"));
        let product_count_label = format!(
            "Number of products{}",
            sort_indicator(&self.sort, "ProductCount")
        );
        let final_due_label = format!(
            "Final due date{}",
            sort_indicator(&self.sort, "FinalDueDate")
        );

        let mut headers = Vec::new();
        if show_select {
            headers.push(TableColumnHeader {
                key: "Select",
                label: "",
                sort_url: None,
                push_url: true,
            });
        }
        headers.push(TableColumnHeader {
            key: "ID",
            label: &id_label,
            sort_url: Some(&id_sort),
            push_url: true,
        });
        headers.push(TableColumnHeader {
            key: "Number",
            label: &number_label,
            sort_url: Some(&number_sort),
            push_url: true,
        });
        if posted_hub {
            headers.push(TableColumnHeader {
                key: "Customer",
                label: &customer_label,
                sort_url: Some(&customer_sort),
                push_url: true,
            });
            headers.push(TableColumnHeader {
                key: "OpenBalance",
                label: &open_balance_label,
                sort_url: Some(&open_balance_sort),
                push_url: true,
            });
        }
        headers.push(TableColumnHeader {
            key: "Date",
            label: &date_label,
            sort_url: Some(&date_sort),
            push_url: true,
        });
        headers.push(TableColumnHeader {
            key: "DeliveryDate",
            label: &delivery_date_label,
            sort_url: Some(&delivery_date_sort),
            push_url: true,
        });
        headers.push(TableColumnHeader {
            key: "UntaxedAmount",
            label: &untaxed_label,
            sort_url: Some(&untaxed_sort),
            push_url: true,
        });
        headers.push(TableColumnHeader {
            key: "TotalAmount",
            label: &total_label,
            sort_url: Some(&total_sort),
            push_url: true,
        });
        headers.push(TableColumnHeader {
            key: "TaxLevied",
            label: &tax_label,
            sort_url: Some(&tax_sort),
            push_url: true,
        });
        headers.push(TableColumnHeader {
            key: "ProductCount",
            label: &product_count_label,
            sort_url: Some(&product_count_sort),
            push_url: true,
        });
        headers.push(TableColumnHeader {
            key: "FinalDueDate",
            label: &final_due_label,
            sort_url: Some(&final_due_sort),
            push_url: true,
        });
        for col in &self.extra_columns {
            headers.push(TableColumnHeader {
                key: col.key,
                label: col.label,
                sort_url: None,
                push_url: true,
            });
        }

        let rows: Vec<TableRow> = self
            .invoices
            .items
            .iter()
            .map(|inv| {
                let mut cells = Vec::new();
                if show_select && inv.selectable {
                    cells.push(maud::PreEscaped(format!(
                        r#"<label class="flex justify-center" @click.stop=""><input type="checkbox" class="checkbox checkbox-sm" @change="{sel}.toggle({id})" :checked="!!{sel}.selected['{id}']" /></label>"#,
                        sel = sel,
                        id = inv.id,
                    ))
                    .into());
                } else if show_select {
                    cells.push(html! {}.into());
                }
                cells.push(field_text(FieldText {
                    value: &inv.id.to_string(),
                    classes: "tabular-nums",
                }));
                cells.push(field_text(FieldText {
                    value: &inv.number,
                    classes: "",
                }));
                if posted_hub {
                    cells.push(field_text(FieldText {
                        value: &inv.customer_name,
                        classes: "",
                    }));
                    cells.push(field_text(FieldText {
                        value: &inv.open_balance,
                        classes: "text-end tabular-nums",
                    }));
                }
                cells.push(field_text(FieldText {
                    value: &inv.datetime,
                    classes: "",
                }));
                cells.push(field_text(FieldText {
                    value: &inv.delivery_date,
                    classes: "",
                }));
                cells.push(field_text(FieldText {
                    value: &inv.untaxed_amount,
                    classes: "text-end tabular-nums",
                }));
                cells.push(field_text(FieldText {
                    value: &inv.total_amount,
                    classes: "text-end tabular-nums",
                }));
                cells.push(field_text(FieldText {
                    value: &inv.tax_levied,
                    classes: "text-end tabular-nums",
                }));
                cells.push(field_text(FieldText {
                    value: &inv.product_count,
                    classes: "text-end tabular-nums",
                }));
                cells.push(field_text(FieldText {
                    value: &inv.final_due_date,
                    classes: "",
                }));
                for cell in &inv.extra_cells {
                    cells.push(field_text(FieldText {
                        value: cell,
                        classes: "",
                    }));
                }
                TableRow {
                    attrs: row_attr_navigate(&inv.detail_href),
                    cells,
                }
            })
            .collect();

        let pagination = render_pagination::<InvoiceHubTableKey>(
            &self.path_and_query,
            self.invoices.number,
            self.invoices.num_pages,
        );

        let draft_create = if self.can_edit && drafts_hub {
            button_modal_form(ButtonModalForm {
                name: "p_finance_invoices.DraftInvoiceCreateForm",
                href: &DraftInvoiceCreateGetRouteTag.url(),
                form_post_url: &DraftInvoiceCreatePostRouteTag.path(),
                modal_uid: DraftInvoiceCreateModalKey::ID,
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            })
        } else {
            html! {}
        };

        let bulk_actions = if show_select {
            let item = |label: &str, classes: &str, on_click: &str| {
                format!(
                    r#"<button type="button" class="btn {classes} btn-sm justify-start w-full" x-bind:class="{sel}.selectedIds().length >= 1 ? '' : 'btn-disabled pointer-events-none opacity-50'" @click="{sel}.{on_click}($el); $el.closest('details')?.removeAttribute('open')">{label}</button>"#,
                    classes = classes,
                    sel = sel,
                    on_click = on_click,
                    label = label,
                )
            };
            let mut items = String::new();
            items.push_str(&item(
                "Download PDFs",
                "btn-ghost",
                "requestBulkDownloadPdfs",
            ));
            if drafts_hub {
                items.push_str(&item("Edit selected", "btn-ghost", "requestBulkEdit"));
                items.push_str(&item("Post selected", "btn-ghost", "requestBulkPost"));
                items.push_str(&item(
                    "Delete selected",
                    "btn-ghost text-error",
                    "requestBulkDelete",
                ));
            }
            if posted_hub {
                items.push_str(&item("Pay selected", "btn-ghost", "requestPaySelected"));
                items.push_str(&item(
                    "Cancel selected",
                    "btn-ghost text-error",
                    "requestBulkCancel",
                ));
            }
            if cancelled_hub {
                items.push_str(&item(
                    "New draft from cancelled",
                    "btn-ghost",
                    "requestBulkNewDraft",
                ));
            }
            html! {
                (PreEscaped(
                    r#"<details class="dropdown dropdown-end" @click.outside="$el.removeAttribute('open')">"#,
                ))
                summary class="btn btn-outline btn-sm dropdown-toggle w-32" {
                    "Bulk actions"
                }
                div class="card w-56 my-1.5 card-body shadow dropdown-content border border-base-300 rounded-box z-50 bg-base-100 p-2" {
                    div class="flex flex-col gap-1" {
                        (PreEscaped(items))
                    }
                }
                (PreEscaped("</details>"))
            }
        } else {
            html! {}
        };

        // Keep create inside the table so refresh id resolves and hx-swap is not lost.
        let actions = html! {
            (draft_create)
            (bulk_actions)
        };

        // Bare table only — selection Alpine state lives outside so pagination swaps
        // do not reset checkboxes.
        data_table_list_refresh::<InvoiceHubTableKey>(
            "Invoices",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        let table = self.render_table();
        let table = if self.show_select() {
            self.wrap_with_selection(table)
        } else {
            table
        };
        html! {
            (container_column("", html! {
                (fiscal_year_environment_selector(&self.fiscal_years, self.selected_fiscal_year_start))
                div class="tabs tabs-boxed mb-4" {
                    (self.tab_link("drafts", "Drafts"))
                    (self.tab_link("posted", "Posted"))
                    (self.tab_link("cancelled", "Cancelled"))
                    (self.tab_link("paid", "Paid"))
                    (self.tab_link("partial", "Partially paid"))
                }
                (table)
            }))
        }
    }
}

impl RenderAppPane for InvoiceHubPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(&self.path_and_query, invoices_list_crumbs(), self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(invoices_list_crumbs(), self.body())
    }
}

impl RenderTemplate for InvoiceHubPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Finance Invoices",
            chrome,
            invoices_list_crumbs(),
            self.body(),
            &self.path_and_query,
        )
    }
}

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
        modal_keyed::<DraftInvoiceEditModalKey>(
            "!max-w-6xl w-full",
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit draft invoice" }
                (form(FormOpts {
                    classes: "@container",
                    attrs: form_hx_post_url::<DraftInvoiceEditModalKey>(&modal_edit_post_url(
                        DraftInvoiceEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (DraftInvoiceForm::render_inputs(&FormCtx::form::<DraftInvoiceForm>()
                            .value(DraftInvoiceFormField::Number, &self.form.number)
                            .value(DraftInvoiceFormField::Reference, &self.form.reference)
                            .value(DraftInvoiceFormField::PaymentReference, &self.form.payment_reference)
                            .value(DraftInvoiceFormField::BankAccount, &self.form.bank_account)
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
                            modal_uid: DraftInvoiceDeleteModalKey::ID,
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
        modal_keyed::<DraftInvoiceCreateModalKey>(
            "!max-w-6xl w-full",
            form(FormOpts {
                title: "Create draft invoice",
                subtitle: "Create a new draft invoice",
                classes: "@container",
                attrs: form_hx_post_url::<DraftInvoiceCreateModalKey>(&modal_create_post_url(
                    DraftInvoiceCreatePostRouteTag,
                    form_name,
                    &self.refresh_table,
                )),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: html! {
                    (DraftInvoiceForm::render_inputs(
                        &FormCtx::form::<DraftInvoiceForm>()
                            .value(DraftInvoiceFormField::Number, &self.form.number)
                            .value(DraftInvoiceFormField::Reference, &self.form.reference)
                            .value(
                                DraftInvoiceFormField::PaymentReference,
                                &self.form.payment_reference,
                            )
                            .value(DraftInvoiceFormField::BankAccount, &self.form.bank_account)
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
                            .display(DraftInvoiceFormField::CustomerId, &self.customer_display)
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
            }),
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
                "Update {} selected draft invoices. Only non-empty fields are applied to every selected draft.",
                self.selected_count
            )
        };
        modal_keyed::<DraftInvoiceBulkEditModalKey>(
            "!max-w-6xl w-full",
            form(FormOpts {
                title: "Bulk edit draft invoices",
                subtitle: &subtitle,
                classes: "@container",
                attrs: form_hx_post_url::<DraftInvoiceBulkEditModalKey>(&modal_create_post_url(
                    DraftInvoiceBulkEditPostRouteTag,
                    form_name,
                    &self.refresh_table,
                )),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: html! {
                    input type="hidden" name="ids" value=(self.ids);
                    (DraftInvoiceBulkEditForm::render_inputs(
                        &FormCtx::form::<DraftInvoiceBulkEditForm>()
                            .value(DraftInvoiceBulkEditFormField::Number, &self.form.number)
                            .value(DraftInvoiceBulkEditFormField::Reference, &self.form.reference)
                            .value(
                                DraftInvoiceBulkEditFormField::PaymentReference,
                                &self.form.payment_reference,
                            )
                            .value(
                                DraftInvoiceBulkEditFormField::BankAccount,
                                &self.form.bank_account,
                            )
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
            }),
        )
    }
}

#[derive(Generic)]
pub struct DraftInvoiceDetailPage {
    pub id: i64,
    pub number: String,
    pub reference: String,
    pub payment_reference: String,
    pub bank_account: String,
    pub datetime: String,
    pub delivery_date: String,
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
                    modal_uid: DraftInvoiceEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
                (button_delete_post_route(
                    DraftInvoicePostRouteTag::new(self.id),
                    ButtonDeletePost {
                        label: "Post invoice",
                        confirm: "Post this draft invoice? This will create a posted invoice.",
                        classes: "btn-primary",
                    },
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
                    (label("Payment reference", field_text(FieldText { value: &self.payment_reference, classes: "" })))
                    (label("Bank account", field_text(FieldText { value: &self.bank_account, classes: "" })))
                    (label("Date", field_text(FieldText { value: &self.datetime, classes: "" })))
                    (label("Delivery date", field_text(FieldText { value: &self.delivery_date, classes: "" })))
                    (label("Customer", customer_link(self.customer_id, &self.customer_name)))
                    (label("Payment schedule", field_payment_term_schedule(&self.payment_term_rows)))
                    (label("Taxes", field_text(FieldText { value: &self.tax_labels, classes: "" })))
                    (PreEscaped(&self.extra_detail))
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let label = draft_invoice_label(self.id, &self.number);
        let detail_url = DraftInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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
        app_scaffold_with_sidebar("Draft Invoice", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Generic)]
pub struct PostedInvoiceDetailPage {
    pub id: i64,
    pub number: String,
    pub reference: String,
    pub payment_reference: String,
    pub bank_account: String,
    pub datetime: String,
    pub delivery_date: String,
    pub customer_id: i64,
    pub customer_name: String,
    pub payment_term_rows: Vec<PaymentTermLineDisplayRow>,
    pub tax_labels: String,
    pub line_rows: Vec<InvoiceLineDisplayRow>,
    pub journal_entry_id: i64,
    pub can_edit: bool,
    pub can_pay: bool,
}

impl PostedInvoiceDetailPage {
    fn body(&self) -> Markup {
        let pay_href = PaymentCreateGetRouteTag
            .with_query()
            .query("PostedInvoiceID", self.id)
            .build();
        let actions = html! {
            (button_modal_route(PostedInvoicePdfModalRouteTag::new(self.id), "PDF", "btn-outline"))
            @if self.can_pay {
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_invoices.PaymentCreateForm",
                    label: "Pay",
                    href: &pay_href,
                    form_post_url: &PaymentCreateGetRouteTag.path(),
                    modal_uid: PaymentCreateModalKey::ID,
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
                    (label("Payment reference", field_text(FieldText { value: &self.payment_reference, classes: "" })))
                    (label("Bank account", field_text(FieldText { value: &self.bank_account, classes: "" })))
                    (label("Date", field_text(FieldText { value: &self.datetime, classes: "" })))
                    (label("Delivery date", field_text(FieldText { value: &self.delivery_date, classes: "" })))
                    (label("Customer", customer_link(self.customer_id, &self.customer_name)))
                    (label("Payment schedule", field_payment_term_schedule(&self.payment_term_rows)))
                    (label("Taxes", field_text(FieldText { value: &self.tax_labels, classes: "" })))
                    (label("Journal entry", journal_entry_link(self.journal_entry_id)))
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let label = invoice_number_label(self.id, &self.number);
        let detail_url = PostedInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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
    pub payment_reference: String,
    pub bank_account: String,
    pub datetime: String,
    pub posted_at: Option<String>,
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
            payment_reference: String::new(),
            bank_account: String::new(),
            datetime: String::new(),
            posted_at: None,
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
    pdf_route: impl crate::http::RouteUrl,
    can_pay: bool,
    can_edit: bool,
) -> Markup {
    let pay_href = PaymentCreateGetRouteTag
        .with_query()
        .query("PostedInvoiceID", ctx.posted_invoice_id)
        .build();
    let actions = html! {
        (button_modal_route(pdf_route, "PDF", "btn-outline"))
        @if can_pay {
            (button_modal_form(ButtonModalForm {
                name: "p_finance_invoices.PaymentCreateForm",
                label: "Pay",
                href: &pay_href,
                form_post_url: &PaymentCreateGetRouteTag.path(),
                modal_uid: PaymentCreateModalKey::ID,
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
                (label("Payment reference", field_text(FieldText { value: &ctx.payment_reference, classes: "" })))
                (label("Bank account", field_text(FieldText { value: &ctx.bank_account, classes: "" })))
                (label("Posted at", field_text(FieldText { value: posted_at_display, classes: "" })))
                (label("Invoice date", field_text(FieldText { value: &ctx.datetime, classes: "" })))
                (label("Customer", customer_link(ctx.customer_id, &ctx.customer_name)))
                (label("Payment schedule", field_payment_term_schedule(&ctx.payment_term_rows)))
                (label("Taxes", field_text(FieldText { value: &ctx.tax_labels, classes: "" })))
                (label("Journal entry", journal_entry_link(ctx.journal_entry_id)))
                (label("Payment", cancelled_detail_link(&Some(ctx.payment_href.clone()), &ctx.payment_label)))
                (label("Payment date", field_text(FieldText { value: &ctx.payment_datetime, classes: "" })))
                (label("Prior partial record", cancelled_detail_link(
                    &ctx.prior_partial_href,
                    ctx.prior_partial_label.as_deref().unwrap_or("—"),
                )))
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let label = invoice_number_label(self.ctx.settlement_id, &self.ctx.number);
        let detail_url = PaidInvoiceDetailRouteTag::new(self.ctx.settlement_id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let label = invoice_number_label(self.ctx.settlement_id, &self.ctx.number);
        let detail_url = PartiallyPaidInvoiceDetailRouteTag::new(self.ctx.settlement_id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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

#[derive(Generic)]
pub struct CancelledInvoiceDetailPage {
    pub id: i64,
    pub number: String,
    pub reference: String,
    pub payment_reference: String,
    pub bank_account: String,
    pub datetime: String,
    pub delivery_date: String,
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
                (button_delete_post_route(
                    CancelledInvoiceNewDraftRouteTag::new(self.id),
                    ButtonDeletePost {
                        label: "New draft from cancelled",
                        confirm: "Create a new draft invoice from this cancelled invoice? The cancelled record will be unchanged.",
                        classes: "btn-primary",
                    },
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
                    (label("Payment reference", field_text(FieldText { value: &self.payment_reference, classes: "" })))
                    (label("Bank account", field_text(FieldText { value: &self.bank_account, classes: "" })))
                    (label("Date", field_text(FieldText { value: &self.datetime, classes: "" })))
                    (label("Delivery date", field_text(FieldText { value: &self.delivery_date, classes: "" })))
                    (label("Customer", customer_link(self.customer_id, &self.customer_name)))
                    (label("Payment schedule", field_payment_term_schedule(&self.payment_term_rows)))
                    (label("Taxes", field_text(FieldText { value: &self.tax_labels, classes: "" })))
                    (label("Posted invoice", cancelled_detail_link(&self.posted_invoice_href, &self.posted_invoice_label)))
                    (label("Credit note", cancelled_detail_link(&self.credit_note_href, &self.credit_note_label)))
                    (field_invoice_lines(&self.line_rows))
                }))
            }))
        }
    }

    fn menu(&self) -> Markup {
        cancelled_invoice_detail_menu(self.id, &self.number)
    }
}

fn cancelled_detail_link(href: &Option<String>, label: &str) -> Markup {
    if let Some(url) = href {
        field_link(FieldLink {
            href: url,
            label,
            classes: "link link-hover",
        })
    } else if label.is_empty() {
        field_text(FieldText {
            value: "—",
            classes: "",
        })
    } else {
        field_text(FieldText {
            value: label,
            classes: "",
        })
    }
}

fn customer_link(customer_id: i64, customer_name: &str) -> Markup {
    if customer_id > 0 {
        field_link(FieldLink {
            href: &CustomerDetailRouteTag::new(customer_id).url(),
            label: customer_name,
            classes: "link link-hover",
        })
    } else if customer_name.is_empty() {
        field_text(FieldText {
            value: "—",
            classes: "",
        })
    } else {
        field_text(FieldText {
            value: customer_name,
            classes: "",
        })
    }
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

impl RenderAppPane for CancelledInvoiceDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let label = invoice_number_label(self.id, &self.number);
        let detail_url = CancelledInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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

#[derive(Clone)]
pub struct PaymentRow {
    pub id: i64,
    pub invoice_label: String,
    pub amount: String,
    pub datetime: String,
}

#[derive(Clone)]
pub struct PaymentBatchRow {
    pub id: i64,
    pub datetime: String,
    pub total_amount: String,
    pub payment_count: u64,
}

#[derive(Generic)]
pub struct PaymentListPage {
    pub tab: String,
    pub payments: ObjectList<PaymentRow>,
    pub batches: ObjectList<PaymentBatchRow>,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl PaymentListPage {
    fn tab_link(&self, tab: &str, label: &str) -> Markup {
        use crate::components::attrs::escape_attr;
        use maud::PreEscaped;

        let active = self.tab == tab;
        let cls = if active { "tab tab-active" } else { "tab" };
        let href = payment_tab_href(tab);
        let nav = crate::components::nav_content_attrs(&href);
        html! {
            (PreEscaped(format!(
                r#"<a class="{cls}" href="{href}"{attrs}>"#,
                cls = escape_attr(cls),
                href = escape_attr(&href),
                attrs = nav.as_string(),
            )))
            (label)
            (PreEscaped("</a>"))
        }
    }

    pub fn render_table(&self) -> Markup {
        if self.tab == "batches" {
            let date_sort = column_sort_url(&self.path_and_query, "Date", &self.sort);
            let total_sort = column_sort_url(&self.path_and_query, "Total", &self.sort);
            let date_label = format!("Date{}", sort_indicator(&self.sort, "Date"));
            let total_label = format!("Total{}", sort_indicator(&self.sort, "Total"));
            let headers = [
                TableColumnHeader {
                    key: "Date",
                    label: &date_label,
                    sort_url: Some(&date_sort),
                    push_url: true,
                },
                TableColumnHeader {
                    key: "Total",
                    label: &total_label,
                    sort_url: Some(&total_sort),
                    push_url: true,
                },
                TableColumnHeader {
                    key: "Payments",
                    label: "Payments",
                    sort_url: None,
                    push_url: true,
                },
            ];
            let rows: Vec<TableRow> = self
                .batches
                .items
                .iter()
                .map(|b| TableRow {
                    attrs: row_attr_navigate(&PaymentBatchDetailRouteTag::new(b.id).url()),
                    cells: vec![
                        field_text(FieldText {
                            value: &b.datetime,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &b.total_amount,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &b.payment_count.to_string(),
                            classes: "",
                        }),
                    ],
                })
                .collect();
            let pagination = render_pagination::<PaymentTableKey>(
                &self.path_and_query,
                self.batches.number,
                self.batches.num_pages,
            );
            return data_table_list_refresh::<PaymentTableKey>(
                "Batch payments",
                html! {},
                &headers,
                &rows,
                pagination,
                &self.path_and_query,
            );
        }

        let amount_sort = column_sort_url(&self.path_and_query, "Amount", &self.sort);
        let date_sort = column_sort_url(&self.path_and_query, "Date", &self.sort);
        let amount_label = format!("Amount{}", sort_indicator(&self.sort, "Amount"));
        let date_label = format!("Date{}", sort_indicator(&self.sort, "Date"));
        let headers = [
            TableColumnHeader {
                key: "Invoice",
                label: "Invoice",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Amount",
                label: &amount_label,
                sort_url: Some(&amount_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Date",
                label: &date_label,
                sort_url: Some(&date_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .payments
            .items
            .iter()
            .map(|p| TableRow {
                attrs: row_attr_navigate(&format!("/finance-invoices/payments/{}/", p.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &p.invoice_label,
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
            })
            .collect();
        let pagination = render_pagination::<PaymentTableKey>(
            &self.path_and_query,
            self.payments.number,
            self.payments.num_pages,
        );
        let actions = if self.can_edit {
            button_modal_form(ButtonModalForm {
                name: "p_finance_invoices.PaymentCreateForm",
                href: &PaymentCreateGetRouteTag.url(),
                form_post_url: &PaymentCreateGetRouteTag.path(),
                modal_uid: PaymentCreateModalKey::ID,
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            })
        } else {
            html! {}
        };
        data_table_list_refresh::<PaymentTableKey>(
            "Single payments",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        html! {
            (container_column("", html! {
                div class="tabs tabs-boxed mb-4" {
                    (self.tab_link("single", "Single payments"))
                    (self.tab_link("batches", "Batch payments"))
                }
                (self.render_table())
            }))
        }
    }
}

impl RenderAppPane for PaymentListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(&self.path_and_query, payments_list_crumbs(), self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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
    pub posted_invoice_display: String,
    pub account_display: String,
    pub tax_items: Vec<ManyToManyItem>,
    pub error: String,
}

impl RenderTemplate for PaymentCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_invoices.PaymentCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed::<PaymentCreateModalKey>(
            "",
            form(FormOpts {
                title: "Record payment",
                subtitle: "Create a payment against a posted invoice",
                classes: "@container",
                attrs: form_hx_post_url::<PaymentCreateModalKey>(&modal_create_post_url(
                    PaymentCreatePostRouteTag,
                    form_name,
                    &self.refresh_table,
                )),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: PaymentForm::render_inputs(
                    &FormCtx::form::<PaymentForm>()
                        .value(
                            PaymentFormField::PostedInvoiceId,
                            &self.form.posted_invoice_id.to_string(),
                        )
                        .value(PaymentFormField::Amount, &self.form.amount)
                        .value(PaymentFormField::AccountId, &self.form.account_id)
                        .value(PaymentFormField::Datetime, &self.form.datetime)
                        .display(
                            PaymentFormField::PostedInvoiceId,
                            &self.posted_invoice_display,
                        )
                        .display(PaymentFormField::AccountId, &self.account_display)
                        .m2m(PaymentFormField::Taxes, &self.tax_items),
                ),
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
            }),
        )
    }
}

#[derive(Generic)]
pub struct PaymentDetailPage {
    pub id: i64,
    pub posted_invoice_label: String,
    pub posted_invoice_href: Option<String>,
    pub amount: String,
    pub tax_labels: String,
    pub datetime: String,
    pub journal_entry_id: i64,
    pub payment_batch_id: Option<i64>,
    pub payment_batch_href: Option<String>,
    pub can_edit: bool,
}

impl PaymentDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &format!("Payment #{}", self.id), classes: "" }))
                    (label("Posted invoice", cancelled_detail_link(&self.posted_invoice_href, &self.posted_invoice_label)))
                    @if let Some(href) = &self.payment_batch_href {
                        @if let Some(batch_id) = self.payment_batch_id {
                            (label("Batch", field_link(FieldLink { href: href.as_str(), label: &format!("Batch #{batch_id}"), classes: "" })))
                        }
                    }
                    (label("Settlement amount", field_text(FieldText { value: &self.amount, classes: "" })))
                    (label("Withholding taxes", field_text(FieldText { value: &self.tax_labels, classes: "" })))
                    (label("Datetime", field_text(FieldText { value: &self.datetime, classes: "" })))
                    (label("Journal entry", journal_entry_link(self.journal_entry_id)))
                }))
            }))
        }
    }

    fn menu(&self) -> Markup {
        payment_detail_menu(self.id)
    }
}

impl RenderAppPane for PaymentDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let label = format!("#{}", self.id);
        let crumbs = payment_crumbs(&label);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        let label = format!("#{}", self.id);
        layout_main_with_crumbs(payment_crumbs(&label), self.body())
    }
}

impl RenderTemplate for PaymentDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = format!("#{}", self.id);
        let crumbs = payment_crumbs(&label);
        app_scaffold_with_sidebar("Payment", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PaymentBatchAllocationRow {
    pub posted_invoice_id: i64,
    pub amount: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tax_ids: Vec<i64>,
    pub invoice_number: String,
    pub customer_name: String,
    pub open_balance: String,
}

#[derive(Generic)]
pub struct PaymentBatchCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub form: PaymentBatchForm,
    pub account_display: String,
    pub batch_allocations_preview: String,
    pub error: String,
}

impl RenderTemplate for PaymentBatchCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_invoices.PaymentBatchCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed::<PaymentBatchCreateModalKey>(
            "!max-w-6xl w-full",
            form(FormOpts {
                title: "Batch payment",
                subtitle: "Record payments against multiple posted invoices",
                classes: "@container",
                attrs: form_hx_post_url::<PaymentBatchCreateModalKey>(&modal_create_post_url(
                    PaymentBatchCreatePostRouteTag,
                    form_name,
                    &self.refresh_table,
                )),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: PaymentBatchForm::render_inputs(
                    &FormCtx::form::<PaymentBatchForm>()
                        .value(PaymentBatchFormField::Datetime, &self.form.datetime)
                        .value(PaymentBatchFormField::AccountId, &self.form.account_id)
                        .value(
                            PaymentBatchFormField::AllocationsJson,
                            &self.form.allocations_json,
                        )
                        .display(PaymentBatchFormField::AccountId, &self.account_display)
                        .display(
                            PaymentBatchFormField::AllocationsJson,
                            &self.batch_allocations_preview,
                        ),
                ),
                actions: html! {
                    (container_row("flex justify-end gap-2 mt-2", html! {
                        (button_submit(ButtonSubmit {
                            label: "Record batch payment",
                            classes: "btn-primary",
                            ..Default::default()
                        }))
                    }))
                },
                ..Default::default()
            }),
        )
    }
}

#[derive(Clone)]
pub struct PaymentBatchPaymentRow {
    pub id: i64,
    pub href: String,
    pub invoice_label: String,
    pub invoice_href: String,
    pub amount: String,
    pub tax_labels: String,
}

#[derive(Generic)]
pub struct PaymentBatchDetailPage {
    pub id: i64,
    pub datetime: String,
    pub account_label: String,
    pub total_amount: String,
    pub journal_entry_id: i64,
    pub payments: Vec<PaymentBatchPaymentRow>,
    pub can_edit: bool,
}

impl PaymentBatchDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &format!("Payment batch #{}", self.id), classes: "" }))
                    (label("Datetime", field_text(FieldText { value: &self.datetime, classes: "" })))
                    (label("Bank account", field_text(FieldText { value: &self.account_label, classes: "" })))
                    (label("Total settlement", field_text(FieldText { value: &self.total_amount, classes: "" })))
                    (label("Journal entry", journal_entry_link(self.journal_entry_id)))
                    h3 class="text-lg font-semibold mt-4" { "Payments in batch" }
                    div class="overflow-x-auto" {
                        table class="table table-zebra w-full" {
                            thead {
                                tr {
                                    th { "Payment" }
                                    th { "Invoice" }
                                    th class="text-end" { "Amount" }
                                    th { "Withholding" }
                                }
                            }
                            tbody {
                                @for p in &self.payments {
                                    tr {
                                        td {
                                            (field_link(FieldLink { href: &p.href, label: &format!("#{}", p.id), classes: "" }))
                                        }
                                        td {
                                            (field_link(FieldLink { href: &p.invoice_href, label: &p.invoice_label, classes: "" }))
                                        }
                                        td class="text-end tabular-nums" {
                                            (field_text(FieldText { value: &p.amount, classes: "" }))
                                        }
                                        td {
                                            (field_text(FieldText { value: &p.tax_labels, classes: "" }))
                                        }
                                    }
                                }
                            }
                        }
                    }
                }))
            }))
        }
    }

    fn menu(&self) -> Markup {
        payment_batch_detail_menu(self.id)
    }
}

impl RenderAppPane for PaymentBatchDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let label = format!("Batch #{}", self.id);
        let crumbs = payment_batch_crumbs(&label);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        let label = format!("Batch #{}", self.id);
        layout_main_with_crumbs(payment_batch_crumbs(&label), self.body())
    }
}

impl RenderTemplate for PaymentBatchDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = format!("Batch #{}", self.id);
        let crumbs = payment_batch_crumbs(&label);
        app_scaffold_with_sidebar("Payment Batch", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Clone)]
pub struct PostedInvoiceSelectRow {
    pub id: i64,
    pub number: String,
    pub datetime: String,
}

#[derive(Generic)]
pub struct PostedInvoiceSelectPage {
    pub invoices: ObjectList<PostedInvoiceSelectRow>,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
}

impl RenderPickerSelect<PostedInvoiceSelectTableKey, PostedInvoiceSelectModalKey>
    for PostedInvoiceSelectPage
{
    fn render_table(&self) -> Markup {
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
        ];
        let rows: Vec<TableRow> = self
            .invoices
            .items
            .iter()
            .map(|inv| {
                let label = if inv.number.is_empty() {
                    format!("#{}", inv.id)
                } else {
                    inv.number.clone()
                };
                TableRow {
                    attrs: row_attr_select(&self.target_input, &inv.id.to_string(), &label),
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
                    ],
                }
            })
            .collect();
        let pagination = render_pagination::<PostedInvoiceSelectTableKey>(
            &self.path_and_query,
            self.invoices.number,
            self.invoices.num_pages,
        );
        data_table_list_refresh::<PostedInvoiceSelectTableKey>(
            "Select Posted Invoice",
            html! {},
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for PostedInvoiceSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
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
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
}

impl RenderPickerSelect<DraftInvoiceSelectTableKey, DraftInvoiceSelectModalKey>
    for DraftInvoiceSelectPage
{
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
            })
            .collect();
        let pagination = render_pagination::<DraftInvoiceSelectTableKey>(
            &self.path_and_query,
            self.invoices.number,
            self.invoices.num_pages,
        );
        data_table_list_refresh::<DraftInvoiceSelectTableKey>(
            "Select invoices",
            html! {},
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for DraftInvoiceSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

#[derive(Generic)]
pub struct CancelInvoicePage {
    pub id: i64,
    pub form: CancelInvoiceForm,
    pub can_edit: bool,
}

impl CancelInvoicePage {
    fn body(&self) -> Markup {
        html! {
            (field_title(FieldTitle { value: &format!("Cancel posted invoice #{}", self.id), classes: "" }))
            form method="post" action=(format!("/finance-invoices/posted/{}/cancel/", self.id)) {
                (CancelInvoiceForm::render_inputs(
                    &FormCtx::form::<CancelInvoiceForm>()
                        .value(CancelInvoiceFormField::Reason, &self.form.reason),
                ))
                (button_submit(ButtonSubmit { label: "Cancel invoice", ..Default::default() }))
            }
        }
    }

    fn menu(&self) -> Markup {
        posted_invoice_detail_menu(self.id, "")
    }
}

impl RenderAppPane for CancelInvoicePage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let label = invoice_number_label(self.id, "");
        let detail_url = PostedInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, Some("Cancel"));
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        let label = invoice_number_label(self.id, "");
        let detail_url = PostedInvoiceDetailRouteTag::new(self.id).url();
        layout_main_with_crumbs(
            invoice_section_crumbs(&label, &detail_url, Some("Cancel")),
            self.body(),
        )
    }
}

impl RenderTemplate for CancelInvoicePage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = invoice_number_label(self.id, "");
        let detail_url = PostedInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, Some("Cancel"));
        app_scaffold_with_sidebar("Cancel Invoice", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Generic)]
pub struct CancelBulkInvoicePage {
    pub ids: String,
    pub count: usize,
    pub form: CancelInvoiceForm,
    pub can_edit: bool,
    pub error: String,
}

impl CancelBulkInvoicePage {
    fn title(&self) -> String {
        if self.count == 1 {
            "Cancel 1 posted invoice".into()
        } else {
            format!("Cancel {} posted invoices", self.count)
        }
    }

    fn body(&self) -> Markup {
        let title = self.title();
        let post_url = PostedInvoiceBulkCancelPostRouteTag.url();
        let form_attrs = form_hx_post_main_url(&post_url);
        html! {
            (field_title(FieldTitle { value: &title, classes: "" }))
            @if !self.error.is_empty() {
                p class="text-error mb-2" { (self.error) }
            }
            @if self.count > 0 && self.can_edit {
                (PreEscaped(format!(
                    r#"<form method="POST"{}>"#,
                    form_attrs.as_string(),
                )))
                input type="hidden" name="ids" value=(self.ids);
                (CancelInvoiceForm::render_inputs(
                    &FormCtx::form::<CancelInvoiceForm>()
                        .value(CancelInvoiceFormField::Reason, &self.form.reason),
                ))
                (button_submit(ButtonSubmit {
                    label: "Cancel invoices",
                    classes: "btn-error",
                    ..Default::default()
                }))
                (PreEscaped("</form>"))
            }
        }
    }
}

impl RenderAppPane for CancelBulkInvoicePage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &InvoiceDefaultRouteTag.url(),
            invoices_list_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(invoices_list_crumbs(), self.body())
    }
}

impl RenderTemplate for CancelBulkInvoicePage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Cancel Invoices",
            chrome,
            invoices_list_crumbs(),
            self.body(),
            &InvoiceDefaultRouteTag.url(),
        )
    }
}

#[derive(Generic)]
pub struct InvoicePreferencesPage {
    pub form: InvoicePreferencesForm,
    pub can_edit: bool,
}

impl InvoicePreferencesPage {
    fn body(&self) -> Markup {
        html! {
            (field_title(FieldTitle { value: "Invoice preferences", classes: "" }))
            form method="post" action="/finance-invoices/preferences/" {
                (InvoicePreferencesForm::render_inputs(&FormCtx::form::<InvoicePreferencesForm>()
                    .value(
                        InvoicePreferencesFormField::AccountReceivableId,
                        &self.form.account_receivable_id,
                    )
                    .value(
                        InvoicePreferencesFormField::AccountRevenueId,
                        &self.form.account_revenue_id,
                    )
                    .value(
                        InvoicePreferencesFormField::AccountTaxPayableId,
                        &self.form.account_tax_payable_id,
                    )
                    .value(InvoicePreferencesFormField::JournalId, &self.form.journal_id)))
                (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
            }
        }
    }
}

impl RenderAppPane for InvoicePreferencesPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &InvoicePreferencesRouteTag.url(),
            invoice_preferences_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(invoice_preferences_crumbs(), self.body())
    }
}

impl RenderTemplate for InvoicePreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Invoice Preferences",
            chrome,
            invoice_preferences_crumbs(),
            self.body(),
            &InvoiceDefaultRouteTag.url(),
        )
    }
}

#[derive(Generic)]
pub struct PaymentPreferencesPage {
    pub form: PaymentPreferencesForm,
    pub can_edit: bool,
}

impl PaymentPreferencesPage {
    fn body(&self) -> Markup {
        html! {
            (field_title(FieldTitle { value: "Payment preferences", classes: "" }))
            form method="post" action="/finance-invoices/payment-preferences/" {
                (PaymentPreferencesForm::render_inputs(&FormCtx::form::<PaymentPreferencesForm>()
                    .value(
                        PaymentPreferencesFormField::PaymentAccountId,
                        &self.form.payment_account_id,
                    )))
                (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
            }
        }
    }
}

impl RenderAppPane for PaymentPreferencesPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &PaymentPreferencesRouteTag.url(),
            payment_preferences_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(payment_preferences_crumbs(), self.body())
    }
}

impl RenderTemplate for PaymentPreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Payment Preferences",
            chrome,
            payment_preferences_crumbs(),
            self.body(),
            &PaymentListRouteTag.url(),
        )
    }
}

#[derive(Generic)]
pub struct ConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub id: i64,
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = if self.modal_uid.is_empty() {
            format!("#{}", DraftInvoiceDeleteModalKey::ID)
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            DraftInvoiceDeleteModalKey::ID
        } else {
            self.modal_uid.as_str()
        };
        let post_url = DraftInvoiceDeletePostRouteTag::new(self.id).url();
        modal(crate::components::Modal {
            uid,
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: &self.message,
                attrs: form_hx_post_selector(&post_url, &target),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

#[derive(Generic)]
pub struct ConfirmBulkDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub ids: String,
    pub error: String,
    pub can_submit: bool,
}

impl RenderTemplate for ConfirmBulkDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = if self.modal_uid.is_empty() {
            format!("#{}", DraftInvoiceBulkDeleteModalKey::ID)
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            DraftInvoiceBulkDeleteModalKey::ID
        } else {
            self.modal_uid.as_str()
        };
        let post_url = DraftInvoiceBulkDeletePostRouteTag.url();
        let form_attrs = form_hx_post_selector(&post_url, &target);
        modal(crate::components::Modal {
            uid,
            children: html! {
                div class="container mx-auto" {
                    h2 class="text-xl font-bold text-error" { "Confirm Deletion" }
                    p class="my-2" { (self.message) }
                    @if !self.error.is_empty() {
                        div class="alert alert-error my-2 text-sm" { (self.error) }
                    }
                    @if self.can_submit {
                        (PreEscaped(format!(
                            r#"<form class="flex flex-col gap-2 my-4"{}>"#,
                            form_attrs.as_string(),
                        )))
                        input type="hidden" name="ids" value=(self.ids);
                        div class="my-2" {
                            (button_submit(ButtonSubmit {
                                label: "Confirm Delete",
                                classes: "btn-error my-2",
                                ..Default::default()
                            }))
                        }
                        (PreEscaped("</form>"))
                    }
                }
            },
            ..Default::default()
        })
    }
}
