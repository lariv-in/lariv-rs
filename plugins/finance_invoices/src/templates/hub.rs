//! Invoice hub list and table helpers.

use maud::{Markup, PreEscaped, html};

use lariv_core::components::{
    ButtonDownload, ButtonModalForm, DataTable, DataTableDisplay, FieldText, FilterClauseView,
    HtmlAttrs, ListFilterBuilder, ObjectList, ShellChrome, SwapKey, TableButtonFilter,
    TableColumnHeader, TableListContent, TableRow, button_download, button_modal_form,
    column_sort_url, container_column, data_table, field_text, filter_error_banner,
    form_hx_get_picker_route, form_hx_get_route, form_hx_get_url, list_filter_builder,
    render_pagination_push, row_attr_navigate, sort_indicator, table_button_bulk_actions,
    table_button_columns, table_button_filter, table_list_content,
};
use lariv_core::http::{FkSelectGet, RouteQueryBuilder, RouteUrl};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use lariv_plugin_finance_accounts::templates::{
    app_scaffold, layout_main_with_crumbs, layout_with_sidebar_crumbs,
};

use crate::components::{self, fiscal_year_environment_selector};
use crate::keys::{DRAFT_INVOICE_CREATE_MODAL, QUOTATION_INVOICE_CREATE_MODAL};

use super::common::{invoices_list_crumbs, tab_href};
use crate::routes::{
    CancelledInvoiceBulkNewDraftRouteTag, DraftInvoiceBulkDeleteGetRouteTag,
    DraftInvoiceBulkEditGetRouteTag, DraftInvoiceBulkPostRouteTag, DraftInvoiceCreateGetRouteTag,
    DraftInvoiceCreatePostRouteTag, InvoiceBulkPdfsRouteTag, InvoiceDefaultRouteTag,
    InvoiceExcelExportRouteTag, PostedInvoiceBulkCancelGetRouteTag,
    PostedInvoiceBulkNewDraftRouteTag, QuotationInvoiceBulkCreateDraftGetRouteTag,
    QuotationInvoiceBulkDeleteGetRouteTag, QuotationInvoiceBulkEditGetRouteTag,
    QuotationInvoiceCreateGetRouteTag, QuotationInvoiceCreatePostRouteTag,
};
use lariv_core::list_filters::{FilterField, FilterRowError};

pub(super) fn invoice_select_filter_form<R>(
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
    /// Cell values for registered [`crate::hub_table_addon`] columns, in order.
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
    pub extra_columns: Vec<crate::hub_table_addon::InvoiceHubExtraColumn>,
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

    fn quotations_hub(&self) -> bool {
        self.tab != "drafts" && self.tab != "posted" && self.tab != "cancelled"
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

    fn selection_x_data(&self) -> String {
        let (bulk_delete, bulk_edit) = if self.quotations_hub() {
            (
                lariv_core::http::trailing_slash(&QuotationInvoiceBulkDeleteGetRouteTag.path()),
                lariv_core::http::trailing_slash(&QuotationInvoiceBulkEditGetRouteTag.path()),
            )
        } else {
            (
                lariv_core::http::trailing_slash(&DraftInvoiceBulkDeleteGetRouteTag.path()),
                lariv_core::http::trailing_slash(&DraftInvoiceBulkEditGetRouteTag.path()),
            )
        };
        let bulk_create_draft =
            lariv_core::http::trailing_slash(&QuotationInvoiceBulkCreateDraftGetRouteTag.path());
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
                const tab = params.get('tab') || 'quotations';
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
            bulkCreateDraftHref() {{
                const ids = this.selectedIds();
                if (ids.length < 1) return '#';
                return '{bulk_create_draft}?ids=' + ids.join(',');
            }},
            requestBulkCreateDraft(el) {{
                const href = this.bulkCreateDraftHref();
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
                lariv_core::components::attrs::escape_attr(&self.selection_x_data()),
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
        let quotations_hub = self.quotations_hub();
        let drafts_hub = self.drafts_hub();
        let cancelled_hub = self.cancelled_hub();
        let show_customer = quotations_hub || drafts_hub || posted_hub;
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

        let pagination = render_pagination_push(
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
        let quotation_create_href = match self.create_customer_id.filter(|id| *id > 0) {
            Some(id) => RouteQueryBuilder::new(QuotationInvoiceCreateGetRouteTag)
                .query("CustomerID", id)
                .build(),
            None => QuotationInvoiceCreateGetRouteTag.url(),
        };
        let quotation_create = if self.can_edit && quotations_hub {
            button_modal_form(ButtonModalForm {
                name: "p_finance_invoices.QuotationInvoiceCreateForm",
                href: &quotation_create_href,
                form_post_url: &QuotationInvoiceCreatePostRouteTag.path(),
                modal_uid: QUOTATION_INVOICE_CREATE_MODAL.id(),
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            })
        } else {
            html! {}
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
            if quotations_hub {
                items.push_str(&item("Edit selected", "btn-ghost", "requestBulkEdit"));
                items.push_str(&item("Create draft", "btn-ghost", "requestBulkCreateDraft"));
                items.push_str(&item(
                    "Delete selected",
                    "btn-ghost text-error",
                    "requestBulkDelete",
                ));
            }
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
            (quotation_create)
            (draft_create)
            (bulk_actions)
        };

        // Bare table only — selection Alpine state lives outside so pagination swaps
        // do not reset checkboxes.
        let table_title = if !self.heading.is_empty() {
            self.heading.as_str()
        } else if quotations_hub {
            "Quotations"
        } else if drafts_hub {
            "Draft invoices"
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
                    (self.tab_link("quotations", "Quotations"))
                    (self.tab_link("drafts", "Drafts"))
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
