use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_plugin_contacts::routes::{CompanyDetailRouteTag, ContactDetailRouteTag};
use lariv_plugin_finance_accounts::routes::JournalEntryDetailRouteTag;

use lariv_core::components::{
    ButtonDownload, ButtonModalForm, ButtonSubmit, Crumb, DataTable, DataTableDisplay,
    DeleteConfirmation, DetailHeader, FieldLink, FieldText, FieldTextarea, FieldTitle,
    FilterClauseView, FormOpts, HtmlAttrs, ListFilterBuilder, ManyToManyItem, ObjectList,
    PaginationPage, ShellChrome, SlotCapability, SlotRegistrar, SwapKey, TableButtonFilter,
    TableColumnHeader, TableListContent, TablePagination, TableRow, breadcrumbs, button_download,
    button_modal_form, button_modal_route, button_submit, column_sort_url, container_column,
    container_row, data_table, delete_confirmation, detail, detail_header, field_link, field_text,
    field_textarea, field_title, filter_error_banner, form, form_hx_get_picker_route,
    form_hx_get_route, form_hx_get_url, form_hx_post_main_url, form_hx_post_selector,
    form_hx_post_url, label, list_filter_builder, modal, modal_keyed, pagination_pages,
    row_attr_navigate, row_attr_select_multi, sort_indicator, table_button_bulk_actions,
    table_button_columns, table_button_filter, table_list_content, table_pagination,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm, csrf_hidden_field};
use lariv_core::http::{FkSelectGet, ProvideRequestCaps, RouteQueryBuilder, RouteUrl};
use lariv_core::picker::RenderPickerSelect;
use lariv_core::template::{
    RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar,
};
use lariv_core::web::{modal_create_post_url, modal_edit_post_url};

use lariv_plugin_finance_accounts::accounting_detail_menu::{
    DetailMenuNavItem, detail_sidebar_menu,
};
use lariv_plugin_finance_accounts::templates::{
    app_scaffold, app_scaffold_with_sidebar, data_table_with_filter_errors,
    layout_main_with_crumbs, layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs,
    render_picker_pagination,
};

use crate::components::{self, field_invoice_lines, fiscal_year_environment_selector};
use crate::logic::PaymentTermLineDisplayRow;
use crate::logic::invoice_line_editor::InvoiceLineDisplayRow;

use super::forms::{
    CancelInvoiceForm, CancelInvoiceFormField, DraftInvoiceBulkEditForm,
    DraftInvoiceBulkEditFormField, DraftInvoiceForm, DraftInvoiceFormField, InvoicePaymentForm,
    InvoicePaymentFormField,
};
use super::keys::{
    DRAFT_INVOICE_BULK_DELETE_MODAL, DRAFT_INVOICE_BULK_EDIT_MODAL, DRAFT_INVOICE_CREATE_MODAL,
    DRAFT_INVOICE_DELETE_MODAL, DRAFT_INVOICE_EDIT_MODAL, DRAFT_INVOICE_SELECT_MODAL,
    DRAFT_INVOICE_SELECT_TABLE, INVOICE_PAY_MODAL,
};
use super::routes::{
    CancelledInvoiceBulkNewDraftRouteTag, CancelledInvoiceDetailRouteTag,
    CancelledInvoiceNewDraftGetRouteTag, CancelledInvoicePdfModalRouteTag,
    DraftInvoiceBulkDeleteGetRouteTag, DraftInvoiceBulkDeletePostRouteTag,
    DraftInvoiceBulkEditGetRouteTag, DraftInvoiceBulkEditPostRouteTag,
    DraftInvoiceBulkPostRouteTag, DraftInvoiceCreateGetRouteTag, DraftInvoiceCreatePostRouteTag,
    DraftInvoiceDeleteGetRouteTag, DraftInvoiceDeletePostRouteTag, DraftInvoiceDetailRouteTag,
    DraftInvoiceEditGetRouteTag, DraftInvoiceEditPostRouteTag, DraftInvoiceMultiSelectRouteTag,
    DraftInvoicePdfModalRouteTag, DraftInvoicePostGetRouteTag, InvoiceBulkPdfsRouteTag,
    InvoiceDefaultRouteTag, InvoiceExcelExportRouteTag, InvoicePreferencesRouteTag,
    PaidInvoiceDetailRouteTag, PaidInvoicePdfModalRouteTag, PartiallyPaidInvoiceDetailRouteTag,
    PartiallyPaidInvoicePdfModalRouteTag, PostedInvoiceBulkCancelGetRouteTag,
    PostedInvoiceBulkCancelPostRouteTag, PostedInvoiceBulkNewDraftRouteTag,
    PostedInvoiceCancelGetRouteTag, PostedInvoiceCancelRouteTag, PostedInvoiceDetailRouteTag,
    PostedInvoiceNewDraftGetRouteTag, PostedInvoicePayGetRouteTag, PostedInvoicePayPostRouteTag,
    PostedInvoicePdfModalRouteTag,
};
use lariv_core::list_filters::{FilterField, FilterRowError};

lariv_core::define_register_items! {
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
        CancelInvoiceIdx: CancelInvoicePageTag => CancelInvoicePage,
        CancelBulkInvoiceIdx: CancelBulkInvoicePageTag => CancelBulkInvoicePage,
        InvoicePayModalIdx: InvoicePayModalPageTag => InvoicePayModalPage,
        InvoicePreferencesIdx: InvoicePreferencesPageTag => InvoicePreferencesPage,
        ConfirmDeleteIdx: DraftInvoiceConfirmDeletePageTag => ConfirmDeletePage,
        ConfirmBulkDeleteIdx: DraftInvoiceConfirmBulkDeletePageTag => ConfirmBulkDeletePage,
    ]
}

lariv_core::define_register_items! {
    plugin: FinanceInvoicesTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
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
    table_pagination(TablePagination {
        pages: &pages,
        hx_target: &hx_target,
    })
}

fn invoices_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Invoices",
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

fn invoice_number_label(id: i64, number: &str) -> String {
    if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    }
}

fn draft_invoice_label(id: i64, number: &str) -> String {
    if number.is_empty() {
        format!("Quotation #{id}")
    } else {
        format!("Quotation {number}")
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

fn tab_href(tab: &str) -> String {
    lariv_core::http::RouteQueryBuilder::new(InvoiceDefaultRouteTag)
        .query("tab", tab)
        .build()
}

fn invoice_select_filter_form<R>(
    fields: &[FilterField],
    rows: &[FilterClauseView],
    page_size: u32,
    sort: &str,
    route: R,
    target_input: &str,
    extra_filters: &str,
) -> Markup
where
    R: RouteUrl + FkSelectGet,
{
    list_filter_builder(ListFilterBuilder {
        fields,
        rows,
        page_size,
        sort,
        attrs: form_hx_get_picker_route(route),
        apply_label: "Apply Filters",
        extra: html! {
            input type="hidden" name="target_input" value=(target_input);
            (PreEscaped(extra_filters))
        },
    })
}

fn invoice_hub_filter_form(page: &InvoiceHubPage) -> Markup {
    let mut attrs = if page.filter_url.is_empty() {
        form_hx_get_route(InvoiceDefaultRouteTag)
    } else {
        form_hx_get_url(page.table_key, &page.filter_url)
    };
    if !page.push_history {
        attrs = attrs.set("hx-push-url", "false");
    }
    let tab = page.tab.clone();
    let keep_tab = page.filter_url.is_empty();
    list_filter_builder(ListFilterBuilder {
        fields: &page.fields,
        rows: &page.filters,
        page_size: page.page_size,
        sort: &page.sort,
        attrs,
        apply_label: "Apply Filters",
        extra: html! {
            @if keep_tab {
                input type="hidden" name="tab" value=(tab);
            }
            (PreEscaped(page.extra_filters.as_str()))
        },
    })
}

fn hub_data_table(
    key: SwapKey,
    title: &str,
    actions: Markup,
    headers: &[TableColumnHeader<'_>],
    rows: &[TableRow],
    pagination: Markup,
    refresh_url: &str,
    errors: &[FilterRowError],
) -> Markup {
    let hx_target = key.selector();
    let list = html! {
        (filter_error_banner(errors))
        (table_list_content(TableListContent {
            headers,
            rows,
            hx_target: &hx_target,
        }))
    };
    let grid = html! {
        (filter_error_banner(errors))
        (lariv_core::components::table::table_grid_content(headers, rows))
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
        uid: key.id(),
        title,
        subtitle: "",
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
        default_view: "List",
        pagination,
        oob: false,
        refresh_url,
        column_keys: &column_keys,
        instance_id: "",
        body_only: false,
    })
}

fn draft_invoice_detail_menu(id: i64, number: &str) -> Markup {
    let label = if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    };
    let detail_url = DraftInvoiceDetailRouteTag::new(id).url();
    let nav = vec![DetailMenuNavItem {
        title: "Quotation Detail",
        url: detail_url,
        active: true,
    }];
    detail_sidebar_menu(format!("Quotation: {label}"), &nav, None, html! {})
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

#[derive(Clone)]
pub struct InvoiceRow {
    pub id: i64,
    /// Addon lookup key (draft id on drafts tab, posted id elsewhere).
    pub addon_key: Option<crate::invoice_addon_key::InvoiceAddonKey>,
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

pub struct InvoiceHubPage {
    pub invoices: ObjectList<InvoiceRow>,
    pub tab: String,
    pub sort: String,
    pub path_and_query: String,
    pub fiscal_years: Vec<components::FiscalYearOption>,
    pub selected_fiscal_year_start: Option<i32>,
    pub can_edit: bool,
    pub extra_columns: Vec<super::hub_table_addon::InvoiceHubExtraColumn>,
    pub page_size: u32,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    /// Pre-rendered addon filter fields (foreign-key pickers need a database lookup).
    pub extra_filters: String,
    pub table_key: SwapKey,
    /// Filter and sort target. Empty uses the invoice hub route.
    pub filter_url: String,
    pub push_history: bool,
    /// Table title. Empty follows the hub tab (quotations vs invoices).
    pub heading: String,
    /// When set, the quotation create button opens with this customer filled in.
    pub create_customer_id: Option<i64>,
}

impl InvoiceHubPage {
    fn tab_link(&self, tab: &str, label: &str) -> Markup {
        use lariv_core::components::attrs::escape_attr;
        use maud::PreEscaped;

        let active = self.tab == tab;
        let cls = if active { "tab tab-active" } else { "tab" };
        let href = tab_href(tab);
        let nav = lariv_core::components::nav_content_attrs(&href);
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

    fn selection_x_data() -> String {
        let bulk_delete =
            lariv_core::http::trailing_slash(&DraftInvoiceBulkDeleteGetRouteTag.path());
        let bulk_edit = lariv_core::http::trailing_slash(&DraftInvoiceBulkEditGetRouteTag.path());
        let bulk_post = lariv_core::http::trailing_slash(&DraftInvoiceBulkPostRouteTag.path());
        let bulk_cancel =
            lariv_core::http::trailing_slash(&PostedInvoiceBulkCancelGetRouteTag.path());
        let bulk_new_draft =
            lariv_core::http::trailing_slash(&CancelledInvoiceBulkNewDraftRouteTag.path());
        let bulk_new_draft_posted =
            lariv_core::http::trailing_slash(&PostedInvoiceBulkNewDraftRouteTag.path());
        let bulk_pdfs = lariv_core::http::trailing_slash(&InvoiceBulkPdfsRouteTag.path());
        format!(
            r#"{{
            selected: {{}},
            toggle(id) {{
                const k = String(id);
                if (this.selected[k]) delete this.selected[k];
                else this.selected[k] = true;
            }},
            setVisible(ids, on) {{
                for (const id of ids) {{
                    const k = String(id);
                    if (on) this.selected[k] = true;
                    else delete this.selected[k];
                }}
            }},
            allVisibleSelected(ids) {{
                return ids.length > 0 && ids.every(id => !!this.selected[String(id)]);
            }},
            someVisibleSelected(ids) {{
                return ids.some(id => !!this.selected[String(id)]);
            }},
            selectedIds() {{
                return Object.keys(this.selected).filter(k => this.selected[k]);
            }},
            bulkDeleteHref() {{
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '{bulk_delete}?ids=' + ids.join(',');
            }},
            bulkEditHref() {{
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '{bulk_edit}?ids=' + ids.join(',') + '&refresh=invoice-hub-table';
            }},
            bulkPostHref() {{
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '{bulk_post}?ids=' + ids.join(',');
            }},
            bulkCancelHref() {{
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '{bulk_cancel}?ids=' + ids.join(',');
            }},
            bulkNewDraftHref() {{
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '{bulk_new_draft}?ids=' + ids.join(',');
            }},
            bulkNewDraftFromPostedHref() {{
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '{bulk_new_draft_posted}?ids=' + ids.join(',');
            }},
            bulkDownloadPdfsHref() {{
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                const params = new URLSearchParams(window.location.search);
                const tab = params.get('tab') || 'drafts';
                return '{bulk_pdfs}?tab=' + encodeURIComponent(tab) + '&ids=' + ids.join(',');
            }},
            requestBulkDelete(el) {{
                const href = this.bulkDeleteHref();
                if (href === '#' || typeof htmx === 'undefined') return;
                htmx.ajax('GET', href, {{ target: 'body', swap: 'beforeend', source: el }});
            }},
            requestBulkEdit(el) {{
                const href = this.bulkEditHref();
                if (href === '#' || typeof htmx === 'undefined') return;
                htmx.ajax('GET', href, {{ target: 'body', swap: 'beforeend', source: el }});
            }},
            requestBulkPost(el) {{
                const href = this.bulkPostHref();
                if (href === '#' || typeof htmx === 'undefined') return;
                htmx.ajax('GET', href, {{ target: 'body', swap: 'beforeend', source: el }});
            }},
            requestBulkCancel(el) {{
                const href = this.bulkCancelHref();
                if (href === '#' || typeof htmx === 'undefined') return;
                htmx.ajax('GET', href, {{
                    target: '#app-layout',
                    select: '#app-layout',
                    swap: 'outerHTML',
                    push: true,
                    source: el,
                }});
            }},
            requestBulkNewDraft(el) {{
                const href = this.bulkNewDraftHref();
                if (href === '#' || typeof htmx === 'undefined') return;
                htmx.ajax('GET', href, {{ target: 'body', swap: 'beforeend', source: el }});
            }},
            requestBulkNewDraftFromPosted(el) {{
                const href = this.bulkNewDraftFromPostedHref();
                if (href === '#' || typeof htmx === 'undefined') return;
                htmx.ajax('GET', href, {{ target: 'body', swap: 'beforeend', source: el }});
            }},
            requestBulkDownloadPdfs() {{
                const href = this.bulkDownloadPdfsHref();
                if (href === '#') return;
                window.location.assign(href);
            }}
        }}"#
        )
    }

    fn wrap_with_selection(&self, table: Markup) -> Markup {
        html! {
            (PreEscaped(format!(
                r#"<div data-invoice-hub-selection x-data="{}">"#,
                lariv_core::components::attrs::escape_attr(&Self::selection_x_data()),
            )))
            (table)
            (PreEscaped("</div>"))
        }
    }

    /// Hub table, optionally wrapped in the bulk-selection Alpine root.
    pub fn render_hub_table(&self, with_selection_root: bool) -> Markup {
        let table = self.render_table();
        if with_selection_root && self.show_select() {
            self.wrap_with_selection(table)
        } else {
            table
        }
    }

    pub fn render_table(&self) -> Markup {
        let posted_hub = self.posted_hub();
        let drafts_hub = self.drafts_hub();
        let cancelled_hub = self.cancelled_hub();
        let show_customer = drafts_hub || posted_hub;
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

        let visible_ids: Vec<i64> = self
            .invoices
            .items
            .iter()
            .filter(|inv| inv.selectable)
            .map(|inv| inv.id)
            .collect();
        let visible_ids_js = format!(
            "[{}]",
            visible_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        let select_all_label = format!(
            r#"<label class="flex justify-center" @click.stop=""><input type="checkbox" class="checkbox checkbox-sm" @change="{sel}.setVisible({ids}, $event.target.checked)" :checked="{sel}.allVisibleSelected({ids})" x-effect="$el.indeterminate = {sel}.someVisibleSelected({ids}) && !{sel}.allVisibleSelected({ids})" /></label>"#,
            sel = sel,
            ids = visible_ids_js,
        );

        let mut headers = Vec::new();
        if show_select {
            headers.push(TableColumnHeader {
                key: "Select",
                label: &select_all_label,
                sort_url: None,
                push_url: self.push_history,
            });
        }
        headers.push(TableColumnHeader {
            key: "ID",
            label: &id_label,
            sort_url: Some(&id_sort),
            push_url: self.push_history,
        });
        headers.push(TableColumnHeader {
            key: "Number",
            label: &number_label,
            sort_url: Some(&number_sort),
            push_url: self.push_history,
        });
        if show_customer {
            headers.push(TableColumnHeader {
                key: "Customer",
                label: &customer_label,
                sort_url: Some(&customer_sort),
                push_url: self.push_history,
            });
        }
        if posted_hub {
            headers.push(TableColumnHeader {
                key: "OpenBalance",
                label: &open_balance_label,
                sort_url: Some(&open_balance_sort),
                push_url: self.push_history,
            });
        }
        headers.push(TableColumnHeader {
            key: "Date",
            label: &date_label,
            sort_url: Some(&date_sort),
            push_url: self.push_history,
        });
        headers.push(TableColumnHeader {
            key: "DeliveryDate",
            label: &delivery_date_label,
            sort_url: Some(&delivery_date_sort),
            push_url: self.push_history,
        });
        headers.push(TableColumnHeader {
            key: "UntaxedAmount",
            label: &untaxed_label,
            sort_url: Some(&untaxed_sort),
            push_url: self.push_history,
        });
        headers.push(TableColumnHeader {
            key: "TotalAmount",
            label: &total_label,
            sort_url: Some(&total_sort),
            push_url: self.push_history,
        });
        headers.push(TableColumnHeader {
            key: "TaxLevied",
            label: &tax_label,
            sort_url: Some(&tax_sort),
            push_url: self.push_history,
        });
        headers.push(TableColumnHeader {
            key: "ProductCount",
            label: &product_count_label,
            sort_url: Some(&product_count_sort),
            push_url: self.push_history,
        });
        headers.push(TableColumnHeader {
            key: "FinalDueDate",
            label: &final_due_label,
            sort_url: Some(&final_due_sort),
            push_url: self.push_history,
        });
        for col in &self.extra_columns {
            headers.push(TableColumnHeader {
                key: col.key,
                label: col.label,
                sort_url: None,
                push_url: self.push_history,
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
                if show_customer {
                    cells.push(field_text(FieldText {
                        value: &inv.customer_name,
                        classes: "",
                    }));
                }
                if posted_hub {
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
                    key: String::new(),
                    children: Vec::new(),
                }
            })
            .collect();

        let pagination = render_pagination(
            self.table_key,
            &self.path_and_query,
            self.invoices.number,
            self.invoices.num_pages,
            self.push_history,
        );

        let draft_create_href = match self.create_customer_id.filter(|id| *id > 0) {
            Some(id) => RouteQueryBuilder::new(DraftInvoiceCreateGetRouteTag)
                .query("CustomerID", id)
                .build(),
            None => DraftInvoiceCreateGetRouteTag.url(),
        };
        let draft_create = if self.can_edit && drafts_hub {
            button_modal_form(ButtonModalForm {
                name: "p_finance_invoices.DraftInvoiceCreateForm",
                href: &draft_create_href,
                form_post_url: &DraftInvoiceCreatePostRouteTag.path(),
                modal_uid: DRAFT_INVOICE_CREATE_MODAL.id(),
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
                items.push_str(&item(
                    "New quotation from posted",
                    "btn-ghost",
                    "requestBulkNewDraftFromPosted",
                ));
                items.push_str(&item(
                    "Cancel selected",
                    "btn-ghost text-error",
                    "requestBulkCancel",
                ));
            }
            if cancelled_hub {
                items.push_str(&item(
                    "New quotation from cancelled",
                    "btn-ghost",
                    "requestBulkNewDraft",
                ));
            }
            table_button_bulk_actions(html! {
                (PreEscaped(items))
            })
        } else {
            html! {}
        };

        // Keep create inside the table so refresh id resolves and hx-swap is not lost.
        let page_size_filter = table_button_filter(TableButtonFilter {
            panel: invoice_hub_filter_form(self),
            ..Default::default()
        });
        let excel_export = InvoiceExcelExportRouteTag.path();
        let excel = if self.filter_url.is_empty() {
            button_download(ButtonDownload {
                label: "",
                href: &excel_export,
                icon_name: Some("arrow-down-tray"),
                classes: "btn-square btn-outline btn-sm",
                attrs: HtmlAttrs::new()
                    .set("aria-label", "Export Excel")
                    .set("title", "Export Excel"),
            })
        } else {
            html! {}
        };
        let actions = html! {
            (page_size_filter)
            (excel)
            (draft_create)
            (bulk_actions)
        };

        // Bare table only — selection Alpine state lives outside so pagination swaps
        // do not reset checkboxes.
        let table_title = if !self.heading.is_empty() {
            self.heading.as_str()
        } else if drafts_hub {
            "Quotations"
        } else {
            "Invoices"
        };
        hub_data_table(
            self.table_key,
            table_title,
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
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
                    (self.tab_link("drafts", "Quotations"))
                    (self.tab_link("posted", "Posted"))
                    (self.tab_link("cancelled", "Cancelled"))
                }
                (table)
            }))
        }
    }
}

impl RenderAppPane for InvoiceHubPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(&self.path_and_query, invoices_list_crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
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
        modal_keyed(
            DRAFT_INVOICE_EDIT_MODAL,
            "!max-w-6xl w-full",
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit quotation" }
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
                    title: "Create quotation",
                    subtitle: "Create a new quotation",
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
            "Update the selected quotation. Only non-empty fields are applied.".to_string()
        } else {
            format!(
                "Update {} selected quotations. Only non-empty fields are applied to every selected quotation.",
                self.selected_count
            )
        };
        modal_keyed(
            DRAFT_INVOICE_BULK_EDIT_MODAL,
            "!max-w-6xl w-full",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Bulk edit quotations",
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
        app_scaffold_with_sidebar("Quotation", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Generic)]
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

fn customer_link(bill_to_individual: bool, party_id: i64, customer_name: &str) -> Markup {
    if party_id > 0 {
        let href = if bill_to_individual {
            ContactDetailRouteTag::new(party_id).url()
        } else {
            CompanyDetailRouteTag::new(party_id).url()
        };
        field_link(FieldLink {
            href: &href,
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
            "Select quotations",
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

#[derive(Generic)]
pub struct InvoicePayModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub form: InvoicePaymentForm,
    pub tax_items: Vec<ManyToManyItem>,
    pub error: String,
}

impl RenderTemplate for InvoicePayModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_invoices.InvoicePaymentForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed(
            INVOICE_PAY_MODAL,
            "!max-w-6xl w-full",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Record payment",
                    subtitle: "Settle this posted invoice",
                    classes: "@container",
                    attrs: form_hx_post_url(
                        INVOICE_PAY_MODAL,
                        &modal_create_post_url(
                            PostedInvoicePayPostRouteTag::new(self.form.posted_invoice_id),
                            form_name,
                            &self.refresh_table,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: InvoicePaymentForm::render_inputs(
                        &FormCtx::form::<InvoicePaymentForm>(CsrfToken::current())
                            .value(
                                InvoicePaymentFormField::PostedInvoiceId,
                                self.form.posted_invoice_id.to_string(),
                            )
                            .value(InvoicePaymentFormField::Amount, &self.form.amount)
                            .value(InvoicePaymentFormField::Datetime, &self.form.datetime)
                            .value(
                                InvoicePaymentFormField::JournalLines,
                                &self.form.journal_lines,
                            )
                            .m2m(InvoicePaymentFormField::Taxes, &self.tax_items),
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
                },
            ),
        )
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
            form method="post" action=(PostedInvoiceCancelRouteTag::new(self.id).url()) {
                (csrf_hidden_field(&CsrfToken::current()))
                (CancelInvoiceForm::render_inputs(
                    &FormCtx::form::<CancelInvoiceForm>(CsrfToken::current())
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
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let label = invoice_number_label(self.id, "");
        let detail_url = PostedInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, Some("Cancel"));
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
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
                (csrf_hidden_field(&CsrfToken::current()))
                input type="hidden" name="ids" value=(self.ids);
                (CancelInvoiceForm::render_inputs(
                    &FormCtx::form::<CancelInvoiceForm>(CsrfToken::current())
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
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &InvoiceDefaultRouteTag.url(),
            invoices_list_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
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
    pub can_edit: bool,
}

impl InvoicePreferencesPage {
    fn body(&self) -> Markup {
        html! {
            (field_title(FieldTitle { value: "Invoice preferences", classes: "" }))
            p { "Invoice accounts come from each product's journal entry template." }
        }
    }
}

impl RenderAppPane for InvoicePreferencesPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &InvoicePreferencesRouteTag.url(),
            invoice_preferences_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
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

/// Confirmation dialog for an invoice state transition (post, new draft).
pub struct ConfirmInvoiceTransition<'a> {
    pub modal_uid: &'a str,
    pub title: &'a str,
    pub message: &'a str,
    pub submit_label: &'a str,
    pub post_url: &'a str,
    pub can_submit: bool,
}

pub fn confirm_invoice_transition(opts: ConfirmInvoiceTransition<'_>) -> Markup {
    let onclick = format!("document.getElementById('{}').remove()", opts.modal_uid);
    let form_attrs = form_hx_post_main_url(opts.post_url);
    modal(lariv_core::components::Modal {
        uid: opts.modal_uid,
        classes: "!max-w-lg",
        children: html! {
            div class="flex flex-col gap-2" {
                h2 class="text-xl font-bold" { (opts.title) }
                p class="my-2" { (opts.message) }
                div class="flex justify-end gap-2 mt-2" {
                    button type="button" class="btn btn-ghost" onclick=(onclick) { "Cancel" }
                    @if opts.can_submit {
                        (PreEscaped(format!(
                            r#"<form class="contents"{}>"#,
                            form_attrs.as_string(),
                        )))
                        (button_submit(ButtonSubmit {
                            label: opts.submit_label,
                            ..Default::default()
                        }))
                        (PreEscaped("</form>"))
                    }
                }
            }
        },
        ..Default::default()
    })
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
            format!("#{}", DRAFT_INVOICE_DELETE_MODAL.id())
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            DRAFT_INVOICE_DELETE_MODAL.id()
        } else {
            self.modal_uid.as_str()
        };
        let post_url = DraftInvoiceDeletePostRouteTag::new(self.id).url();
        modal(lariv_core::components::Modal {
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
            format!("#{}", DRAFT_INVOICE_BULK_DELETE_MODAL.id())
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            DRAFT_INVOICE_BULK_DELETE_MODAL.id()
        } else {
            self.modal_uid.as_str()
        };
        let post_url = DraftInvoiceBulkDeletePostRouteTag.url();
        let form_attrs = form_hx_post_selector(&post_url, &target);
        modal(lariv_core::components::Modal {
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
                        (csrf_hidden_field(&CsrfToken::current()))
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

#[cfg(test)]
mod transition_confirm_tests {
    use super::ConfirmInvoiceTransition;
    use super::confirm_invoice_transition;

    #[test]
    fn state_transition_confirm_is_a_dialog() {
        let html = confirm_invoice_transition(ConfirmInvoiceTransition {
            modal_uid: "draft-invoice-post-modal",
            title: "Post invoice",
            message: "Post this quotation? This will create a posted invoice.",
            submit_label: "Post invoice",
            post_url: "/dashboard/finance-invoices/i/4/post",
            can_submit: true,
        })
        .into_string();

        assert!(html.contains(r#"<dialog id="draft-invoice-post-modal""#));
        assert!(html.contains("Post this quotation? This will create a posted invoice."));
        assert!(html.contains(r#"hx-post="/dashboard/finance-invoices/i/4/post""#));
        assert!(html.contains(">Post invoice</button>"));
        assert!(html.contains(">Cancel</button>"));
        assert!(!html.contains("hx-confirm"));
        assert!(!html.contains("confirm("));
    }
}
