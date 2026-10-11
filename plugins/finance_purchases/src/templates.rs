use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_plugin_contacts::routes::{CompanyDetailRouteTag, ContactDetailRouteTag};
use lariv_plugin_finance_accounts::routes::JournalEntryDetailRouteTag;

use lariv_core::components::{
    ButtonDeletePost, ButtonModalForm, ButtonSubmit, Crumb, DeleteConfirmation, DetailHeader,
    FieldLink, FieldText, FieldTextarea, FieldTitle, FormOpts, ManyToManyItem, ObjectList,
    PaginationPage, ShellChrome, SlotCapability, SlotRegistrar, SwapKey, TableButtonFilter,
    TableColumnHeader, TablePagination, TableRow, breadcrumbs, button_delete_post_route,
    button_modal_form, button_modal_route, button_submit, column_sort_url, container_column,
    container_row, data_table_list_refresh, delete_confirmation, detail, detail_header, field_link,
    field_text, field_textarea, field_title, form, form_hx_get_route, form_hx_post_main_url,
    form_hx_post_selector, form_hx_post_url, label, modal, modal_keyed,
    pagination_pages, row_attr_navigate,
    row_attr_select_multi, sort_indicator, table_button_bulk_actions, table_button_filter,
    table_pagination, with_list_filter_common,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm, csrf_hidden_field};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::picker::RenderPickerSelect;
use lariv_core::template::{
    RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar,
};
use lariv_core::web::{modal_create_post_url, modal_edit_post_url};

use lariv_plugin_finance_accounts::accounting_detail_menu::{
    DetailMenuNavItem, detail_sidebar_menu,
};
use lariv_plugin_finance_accounts::templates::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs,
};

use crate::components::{self, field_purchase_lines, fiscal_year_environment_selector};
use crate::logic::PaymentTermLineDisplayRow;
use crate::logic::purchase_line_editor::PurchaseLineDisplayRow;

use super::forms::{
    CancelPurchaseForm, CancelPurchaseFormField, DraftPurchaseBulkEditForm,
    DraftPurchaseBulkEditFormField, DraftPurchaseForm, DraftPurchaseFormField,
    PurchaseHubFilterForm, PurchaseHubFilterFormField, PurchaseHubFilterFormFlag,
    individual_is_on, individual_x_data, with_contact_company_prefill,
    PurchasePreferencesForm, PurchasePreferencesFormField,
};
use super::hub_filter::HubFilterInput;
use super::keys::{
    DraftPurchaseBulkDeleteModalKey, DraftPurchaseBulkEditModalKey, DraftPurchaseCreateModalKey,
    DraftPurchaseDeleteModalKey, DraftPurchaseEditModalKey, DraftPurchaseSelectModalKey,
    DraftPurchaseSelectTableKey, PurchaseHubTableKey,
};
use super::routes::{
    CancelledPurchaseBulkNewDraftRouteTag, CancelledPurchaseDetailRouteTag,
    CancelledPurchaseNewDraftRouteTag, CancelledPurchasePdfModalRouteTag,
    DraftPurchaseBulkDeleteGetRouteTag, DraftPurchaseBulkDeletePostRouteTag,
    DraftPurchaseBulkEditGetRouteTag, DraftPurchaseBulkEditPostRouteTag,
    DraftPurchaseBulkPostRouteTag, DraftPurchaseCreateGetRouteTag, DraftPurchaseCreatePostRouteTag,
    DraftPurchaseDeleteGetRouteTag, DraftPurchaseDeletePostRouteTag, DraftPurchaseDetailRouteTag,
    DraftPurchaseEditGetRouteTag, DraftPurchaseEditPostRouteTag, DraftPurchasePdfModalRouteTag,
    DraftPurchasePostRouteTag, PostedPurchaseBulkCancelGetRouteTag,
    PostedPurchaseBulkCancelPostRouteTag, PostedPurchaseCancelGetRouteTag,
    PostedPurchaseCancelRouteTag, PostedPurchaseDetailRouteTag, PostedPurchasePdfModalRouteTag,
    PurchaseBulkPdfsRouteTag, PurchaseDefaultRouteTag, PurchasePreferencesPostRouteTag,
    PurchasePreferencesRouteTag,
};

lariv_core::define_register_items! {
    plugin: FinancePurchasesTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        PurchaseHubIdx: PurchaseHubPageTag => PurchaseHubPage,
        DraftPurchaseEditModalIdx: DraftPurchaseEditModalPageTag => DraftPurchaseEditModalPage,
        DraftPurchaseCreateModalIdx: DraftPurchaseCreateModalPageTag => DraftPurchaseCreateModalPage,
        DraftPurchaseBulkEditModalIdx: DraftPurchaseBulkEditModalPageTag => DraftPurchaseBulkEditModalPage,
        DraftPurchaseDetailIdx: DraftPurchaseDetailPageTag => DraftPurchaseDetailPage,
        DraftPurchaseSelectIdx: DraftPurchaseSelectPageTag => DraftPurchaseSelectPage,
        PostedPurchaseDetailIdx: PostedPurchaseDetailPageTag => PostedPurchaseDetailPage,
        CancelledPurchaseDetailIdx: CancelledPurchaseDetailPageTag => CancelledPurchaseDetailPage,
        CancelPurchaseIdx: CancelPurchasePageTag => CancelPurchasePage,
        CancelBulkPurchaseIdx: CancelBulkPurchasePageTag => CancelBulkPurchasePage,
        PurchasePreferencesIdx: PurchasePreferencesPageTag => PurchasePreferencesPage,
        ConfirmDeleteIdx: DraftPurchaseConfirmDeletePageTag => ConfirmDeletePage,
        ConfirmBulkDeleteIdx: DraftPurchaseConfirmBulkDeletePageTag => ConfirmBulkDeletePage,
    ]
}

lariv_core::define_register_items! {
    plugin: FinancePurchasesTag;
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

fn purchases_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Purchases",
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

fn purchase_preferences_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Purchase preferences",
        href: None,
    }])
}

fn purchase_number_label(id: i64, number: &str) -> String {
    if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    }
}

fn draft_purchase_label(id: i64, number: &str) -> String {
    if number.is_empty() {
        format!("Draft #{id}")
    } else {
        format!("Draft {number}")
    }
}

fn purchase_section_crumbs(label: &str, detail_url: &str, action: Option<&str>) -> Markup {
    let list_url = PurchaseDefaultRouteTag.url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Purchases",
                href: Some(&list_url),
            },
            Crumb {
                label: label,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Purchases",
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
    lariv_core::http::RouteQueryBuilder::new(PurchaseDefaultRouteTag)
        .query("tab", tab)
        .build()
}

fn filter_value(raw: &Option<String>) -> &str {
    raw.as_deref().unwrap_or("")
}

const PURCHASE_HUB_FILTER_PANEL: &str = "card w-[28rem] max-w-[90vw] max-h-[70vh] overflow-y-auto my-1.5 card-body shadow dropdown-content border border-base-300 rounded-box z-2 bg-base-100";

fn purchase_hub_filter_form(
    tab: &str,
    sort: &str,
    page_size: u32,
    filters: &HubFilterInput,
    product_display: &str,
    extra_filters: &str,
) -> Markup {
    form(
        &CsrfToken::current(),
        FormOpts {
            classes: "@container",
            attrs: form_hx_get_route::<PurchaseHubTableKey, PurchaseDefaultRouteTag>(
                PurchaseDefaultRouteTag,
            ),
            inputs: html! {
                (with_list_filter_common(
                    html! {
                    (PurchaseHubFilterForm::render_inputs(
                        &FormCtx::form::<PurchaseHubFilterForm>(CsrfToken::current())
                            .flag(PurchaseHubFilterFormFlag::Posted, tab == "posted")
                            .value(PurchaseHubFilterFormField::Id, filter_value(&filters.id))
                            .value(PurchaseHubFilterFormField::Number, filter_value(&filters.number))
                            .value(PurchaseHubFilterFormField::Vendor, filter_value(&filters.vendor))
                            .value(
                                PurchaseHubFilterFormField::OpenBalanceMin,
                                filter_value(&filters.open_balance_min),
                            )
                            .value(
                                PurchaseHubFilterFormField::OpenBalanceMax,
                                filter_value(&filters.open_balance_max),
                            )
                            .value(
                                PurchaseHubFilterFormField::DatetimeFrom,
                                filter_value(&filters.datetime_from),
                            )
                            .value(
                                PurchaseHubFilterFormField::DatetimeTo,
                                filter_value(&filters.datetime_to),
                            )
                            .value(
                                PurchaseHubFilterFormField::DeliveryDateFrom,
                                filter_value(&filters.delivery_date_from),
                            )
                            .value(
                                PurchaseHubFilterFormField::DeliveryDateTo,
                                filter_value(&filters.delivery_date_to),
                            )
                            .value(
                                PurchaseHubFilterFormField::UntaxedMin,
                                filter_value(&filters.untaxed_min),
                            )
                            .value(
                                PurchaseHubFilterFormField::UntaxedMax,
                                filter_value(&filters.untaxed_max),
                            )
                            .value(PurchaseHubFilterFormField::TotalMin, filter_value(&filters.total_min))
                            .value(PurchaseHubFilterFormField::TotalMax, filter_value(&filters.total_max))
                            .value(PurchaseHubFilterFormField::TaxMin, filter_value(&filters.tax_min))
                            .value(PurchaseHubFilterFormField::TaxMax, filter_value(&filters.tax_max))
                            .value(
                                PurchaseHubFilterFormField::ProductId,
                                filter_value(&filters.product_id),
                            )
                            .display(PurchaseHubFilterFormField::ProductId, product_display)
                            .value(
                                PurchaseHubFilterFormField::ProductCountMin,
                                filter_value(&filters.product_count_min),
                            )
                            .value(
                                PurchaseHubFilterFormField::ProductCountMax,
                                filter_value(&filters.product_count_max),
                            )
                            .value(
                                PurchaseHubFilterFormField::FinalDueFrom,
                                filter_value(&filters.final_due_from),
                            )
                            .value(
                                PurchaseHubFilterFormField::FinalDueTo,
                                filter_value(&filters.final_due_to),
                            ),
                    ))
                    (PreEscaped(extra_filters))
                    },
                    page_size,
                ))
                input type="hidden" name="tab" value=(tab) {}
                input type="hidden" name="sort" value=(sort) {}
            },
            actions: html! {
                (container_row("flex gap-2", html! {
                    (button_submit(ButtonSubmit { label: "Apply Filters", ..Default::default() }))
                    (PreEscaped(
                        r#"<button type="button" class="btn btn-ghost" onclick="const form=this.closest('form'); form.querySelectorAll('[x-data]').forEach(el => { const data = window.Alpine && Alpine.$data(el); if (data && typeof data.clear === 'function') data.clear(); }); form.querySelectorAll('input:not([type=hidden]):not([name=page_size]),select:not([name=page_size]),textarea:not([name=page_size])').forEach(el => { el.value = ''; });">Clear</button>"#,
                    ))
                }))
            },
            ..Default::default()
        },
    )
}

fn draft_purchase_detail_menu(id: i64, number: &str) -> Markup {
    let label = if number.is_empty() {
        format!("Draft #{id}")
    } else {
        format!("Draft {number}")
    };
    let detail_url = DraftPurchaseDetailRouteTag::new(id).url();
    let nav = vec![DetailMenuNavItem {
        title: "Draft Purchase Detail",
        url: detail_url,
        active: true,
    }];
    detail_sidebar_menu(format!("Purchase: {label}"), &nav, None, html! {})
}

fn posted_purchase_detail_menu(id: i64, number: &str) -> Markup {
    let label = if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    };
    detail_sidebar_menu(
        format!("Posted purchase: {label}"),
        &[DetailMenuNavItem {
            title: "Posted Purchase Detail",
            url: PostedPurchaseDetailRouteTag::new(id).url(),
            active: true,
        }],
        None,
        html! {},
    )
}

fn cancelled_purchase_detail_menu(id: i64, number: &str) -> Markup {
    let label = if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    };
    detail_sidebar_menu(
        format!("Cancelled purchase: {label}"),
        &[DetailMenuNavItem {
            title: "Cancelled Purchase Detail",
            url: CancelledPurchaseDetailRouteTag::new(id).url(),
            active: true,
        }],
        None,
        html! {},
    )
}

pub struct PurchaseRow {
    pub id: i64,
    /// Draft purchase id when the row can be linked to site (and other) addons.
    pub draft_purchase_id: Option<i64>,
    pub number: String,
    pub datetime: String,
    pub delivery_date: String,
    pub detail_href: String,
    pub vendor_name: String,
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
pub struct PurchaseHubPage {
    pub purchases: ObjectList<PurchaseRow>,
    pub tab: String,
    pub sort: String,
    pub path_and_query: String,
    pub fiscal_years: Vec<components::FiscalYearOption>,
    pub selected_fiscal_year_start: Option<i32>,
    pub can_edit: bool,
    pub extra_columns: Vec<super::hub_table_addon::PurchaseHubExtraColumn>,
    pub page_size: u32,
    pub filters: HubFilterInput,
    /// Label for the selected product filter, empty when none is selected.
    pub product_display: String,
    /// Pre-rendered addon filter fields (foreign-key pickers need a database lookup).
    pub extra_filters: String,
}

impl PurchaseHubPage {
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
        "Alpine.$data($el.closest('[data-purchase-hub-selection]'))"
    }

    fn selection_x_data() -> String {
        let bulk_delete =
            lariv_core::http::trailing_slash(&DraftPurchaseBulkDeleteGetRouteTag.path());
        let bulk_edit = lariv_core::http::trailing_slash(&DraftPurchaseBulkEditGetRouteTag.path());
        let bulk_post = lariv_core::http::trailing_slash(&DraftPurchaseBulkPostRouteTag.path());
        let bulk_cancel =
            lariv_core::http::trailing_slash(&PostedPurchaseBulkCancelGetRouteTag.path());
        let bulk_new_draft =
            lariv_core::http::trailing_slash(&CancelledPurchaseBulkNewDraftRouteTag.path());
        let bulk_pdfs = lariv_core::http::trailing_slash(&PurchaseBulkPdfsRouteTag.path());
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
                return '{bulk_edit}?ids=' + ids.join(',') + '&refresh=purchase-hub-table';
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
                if (!confirm('Post selected draft purchases? This will create posted purchases.')) return;
                htmx.ajax('POST', href, {{
                    target: '#app-layout',
                    select: '#app-layout',
                    swap: 'outerHTML',
                    push: true,
                    source: el,
                }});
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
                if (!confirm('Create new draft purchases from the selected cancelled purchases? The cancelled records will be unchanged.')) return;
                htmx.ajax('POST', href, {{
                    target: '#app-layout',
                    select: '#app-layout',
                    swap: 'outerHTML',
                    push: true,
                    source: el,
                }});
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
                r#"<div data-purchase-hub-selection x-data="{}">"#,
                lariv_core::components::attrs::escape_attr(&Self::selection_x_data()),
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
        let vendor_sort = column_sort_url(&self.path_and_query, "Vendor", &self.sort);
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
        let vendor_label = format!("Vendor{}", sort_indicator(&self.sort, "Vendor"));
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
            .purchases
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
                key: "Vendor",
                label: &vendor_label,
                sort_url: Some(&vendor_sort),
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
            .purchases
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
                        value: &inv.vendor_name,
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

        let pagination = render_pagination::<PurchaseHubTableKey>(
            &self.path_and_query,
            self.purchases.number,
            self.purchases.num_pages,
        );

        let draft_create = if self.can_edit && drafts_hub {
            button_modal_form(ButtonModalForm {
                name: "p_finance_purchases.DraftPurchaseCreateForm",
                href: &DraftPurchaseCreateGetRouteTag.url(),
                form_post_url: &DraftPurchaseCreatePostRouteTag.path(),
                modal_uid: DraftPurchaseCreateModalKey::ID,
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
            table_button_bulk_actions(html! {
                (PreEscaped(items))
            })
        } else {
            html! {}
        };

        // Keep create inside the table so refresh id resolves and hx-swap is not lost.
        let page_size_filter = table_button_filter(TableButtonFilter {
            panel: purchase_hub_filter_form(
                &self.tab,
                &self.sort,
                self.page_size,
                &self.filters,
                &self.product_display,
                &self.extra_filters,
            ),
            content_classes: PURCHASE_HUB_FILTER_PANEL.into(),
        });
        let actions = html! {
            (page_size_filter)
            (draft_create)
            (bulk_actions)
        };

        // Bare table only — selection Alpine state lives outside so pagination swaps
        // do not reset checkboxes.
        data_table_list_refresh::<PurchaseHubTableKey>(
            "Purchases",
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
                }
                (table)
            }))
        }
    }
}

impl RenderAppPane for PurchaseHubPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(&self.path_and_query, purchases_list_crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(purchases_list_crumbs(), self.body())
    }
}

impl RenderTemplate for PurchaseHubPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Finance Purchases",
            chrome,
            purchases_list_crumbs(),
            self.body(),
            &self.path_and_query,
        )
    }
}

/// Edit draft purchase form (modal). Create uses [`DraftPurchaseCreateModalPage`].
#[derive(Generic)]
pub struct DraftPurchaseEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub form: DraftPurchaseForm,
    pub error: String,
    pub individual_display: String,
    pub company_display: String,
    pub tax_items: Vec<ManyToManyItem>,
    pub purchase_lines_preview: String,
    pub extra_inputs: String,
}

impl RenderTemplate for DraftPurchaseEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = DraftPurchaseDeleteGetRouteTag::new(self.id).url();
        modal_keyed::<DraftPurchaseEditModalKey>(
            "!max-w-6xl w-full",
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit draft purchase" }
                (form(&CsrfToken::current(), FormOpts {
                    classes: "@container",
                    attrs: form_hx_post_url::<DraftPurchaseEditModalKey>(&modal_edit_post_url(
                        DraftPurchaseEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (with_contact_company_prefill(DraftPurchaseForm::render_inputs(&FormCtx::form::<DraftPurchaseForm>(CsrfToken::current())
                            .x_data(&individual_x_data(&self.form.vendor_is_individual))
                            .checked(
                                DraftPurchaseFormField::VendorIsIndividual,
                                individual_is_on(&self.form.vendor_is_individual),
                            )
                            .value(DraftPurchaseFormField::Number, &self.form.number)
                            .value(DraftPurchaseFormField::Reference, &self.form.reference)
                            .value(DraftPurchaseFormField::PaymentReference, &self.form.payment_reference)
                            .value(DraftPurchaseFormField::BankAccount, &self.form.bank_account)
                            .value(DraftPurchaseFormField::Remarks, &self.form.remarks)
                            .value(DraftPurchaseFormField::Datetime, &self.form.datetime)
                            .value(DraftPurchaseFormField::DeliveryDate, &self.form.delivery_date)
                            .value(DraftPurchaseFormField::VendorIsIndividual, &self.form.vendor_is_individual)
                            .value(DraftPurchaseFormField::VendorContactId, &self.form.vendor_contact_id.to_string())
                            .value(DraftPurchaseFormField::VendorCompanyId, &self.form.vendor_company_id.to_string())
                            .value(DraftPurchaseFormField::PaymentTermLinesJson, &self.form.payment_term_lines_json)
                            .value(DraftPurchaseFormField::PurchaseLinesJson, &self.form.purchase_lines_json)
                            .display(DraftPurchaseFormField::VendorContactId, &self.individual_display)
                            .display(DraftPurchaseFormField::VendorCompanyId, &self.company_display)
                            .display(DraftPurchaseFormField::PurchaseLinesJson, &self.purchase_lines_preview)
                            .m2m(DraftPurchaseFormField::Taxes, &self.tax_items))))
                        (PreEscaped(&self.extra_inputs))
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_finance_purchases.DraftPurchaseDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: DraftPurchaseDeleteModalKey::ID,
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
pub struct DraftPurchaseCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub form: DraftPurchaseForm,
    pub individual_display: String,
    pub company_display: String,
    pub tax_items: Vec<ManyToManyItem>,
    pub purchase_lines_preview: String,
    pub extra_inputs: String,
    pub error: String,
}

impl RenderTemplate for DraftPurchaseCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_purchases.DraftPurchaseCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed::<DraftPurchaseCreateModalKey>(
            "!max-w-6xl w-full",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create draft purchase",
                    subtitle: "Create a new draft purchase",
                    classes: "@container",
                    attrs: form_hx_post_url::<DraftPurchaseCreateModalKey>(&modal_create_post_url(
                        DraftPurchaseCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        (with_contact_company_prefill(DraftPurchaseForm::render_inputs(
                            &FormCtx::form::<DraftPurchaseForm>(CsrfToken::current())
                                .x_data(&individual_x_data(&self.form.vendor_is_individual))
                                .checked(
                                    DraftPurchaseFormField::VendorIsIndividual,
                                    individual_is_on(&self.form.vendor_is_individual),
                                )
                                .value(DraftPurchaseFormField::Number, &self.form.number)
                                .value(DraftPurchaseFormField::Reference, &self.form.reference)
                                .value(
                                    DraftPurchaseFormField::PaymentReference,
                                    &self.form.payment_reference,
                                )
                                .value(DraftPurchaseFormField::BankAccount, &self.form.bank_account)
                                .value(DraftPurchaseFormField::Remarks, &self.form.remarks)
                                .value(DraftPurchaseFormField::Datetime, &self.form.datetime)
                                .value(
                                    DraftPurchaseFormField::DeliveryDate,
                                    &self.form.delivery_date,
                                )
                                .value(
                                    DraftPurchaseFormField::VendorIsIndividual,
                                    &self.form.vendor_is_individual,
                                )
                                .value(
                                    DraftPurchaseFormField::VendorContactId,
                                    &self.form.vendor_contact_id.to_string(),
                                )
                                .value(
                                    DraftPurchaseFormField::VendorCompanyId,
                                    &self.form.vendor_company_id.to_string(),
                                )
                                .value(
                                    DraftPurchaseFormField::PaymentTermLinesJson,
                                    &self.form.payment_term_lines_json,
                                )
                                .value(
                                    DraftPurchaseFormField::PurchaseLinesJson,
                                    &self.form.purchase_lines_json,
                                )
                                .display(
                                    DraftPurchaseFormField::VendorContactId,
                                    &self.individual_display,
                                )
                                .display(
                                    DraftPurchaseFormField::VendorCompanyId,
                                    &self.company_display,
                                )
                                .display(
                                    DraftPurchaseFormField::PurchaseLinesJson,
                                    &self.purchase_lines_preview,
                                )
                                .m2m(DraftPurchaseFormField::Taxes, &self.tax_items),
                        )))
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

/// Bulk-edit selected draft purchases (blank form; non-empty fields apply to all).
#[derive(Generic)]
pub struct DraftPurchaseBulkEditModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub ids: String,
    pub selected_count: usize,
    pub form: DraftPurchaseBulkEditForm,
    pub individual_display: String,
    pub company_display: String,
    pub tax_items: Vec<ManyToManyItem>,
    pub purchase_lines_preview: String,
    pub extra_inputs: String,
    pub error: String,
    pub can_submit: bool,
}

impl RenderTemplate for DraftPurchaseBulkEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_purchases.DraftPurchaseBulkEditForm"
        } else {
            self.form_name.as_str()
        };
        let subtitle = if self.selected_count == 1 {
            "Update the selected draft purchase. Only non-empty fields are applied.".to_string()
        } else {
            format!(
                "Update {} selected draft purchases. Only non-empty fields are applied to every selected draft.",
                self.selected_count
            )
        };
        modal_keyed::<DraftPurchaseBulkEditModalKey>(
            "!max-w-6xl w-full",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Bulk edit draft purchases",
                    subtitle: &subtitle,
                    classes: "@container",
                    attrs: form_hx_post_url::<DraftPurchaseBulkEditModalKey>(
                        &modal_create_post_url(
                            DraftPurchaseBulkEditPostRouteTag,
                            form_name,
                            &self.refresh_table,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {
                        input type="hidden" name="ids" value=(self.ids);
                        (with_contact_company_prefill(DraftPurchaseBulkEditForm::render_inputs(
                            &FormCtx::form::<DraftPurchaseBulkEditForm>(CsrfToken::current())
                                .x_data(&individual_x_data(&self.form.vendor_is_individual))
                                .checked(
                                    DraftPurchaseBulkEditFormField::VendorIsIndividual,
                                    individual_is_on(&self.form.vendor_is_individual),
                                )
                                .value(DraftPurchaseBulkEditFormField::Number, &self.form.number)
                                .value(DraftPurchaseBulkEditFormField::Reference, &self.form.reference)
                                .value(
                                    DraftPurchaseBulkEditFormField::PaymentReference,
                                    &self.form.payment_reference,
                                )
                                .value(
                                    DraftPurchaseBulkEditFormField::BankAccount,
                                    &self.form.bank_account,
                                )
                                .value(DraftPurchaseBulkEditFormField::Remarks, &self.form.remarks)
                                .value(DraftPurchaseBulkEditFormField::Datetime, &self.form.datetime)
                                .value(
                                    DraftPurchaseBulkEditFormField::DeliveryDate,
                                    &self.form.delivery_date,
                                )
                                .value(
                                    DraftPurchaseBulkEditFormField::VendorIsIndividual,
                                    &self.form.vendor_is_individual,
                                )
                                .value(
                                    DraftPurchaseBulkEditFormField::VendorContactId,
                                    &self.form.vendor_contact_id.to_string(),
                                )
                                .value(
                                    DraftPurchaseBulkEditFormField::VendorCompanyId,
                                    &self.form.vendor_company_id.to_string(),
                                )
                                .value(
                                    DraftPurchaseBulkEditFormField::PaymentTermLinesJson,
                                    &self.form.payment_term_lines_json,
                                )
                                .value(
                                    DraftPurchaseBulkEditFormField::PurchaseLinesJson,
                                    &self.form.purchase_lines_json,
                                )
                                .display(
                                    DraftPurchaseBulkEditFormField::VendorContactId,
                                    &self.individual_display,
                                )
                                .display(
                                    DraftPurchaseBulkEditFormField::VendorCompanyId,
                                    &self.company_display,
                                )
                                .display(
                                    DraftPurchaseBulkEditFormField::PurchaseLinesJson,
                                    &self.purchase_lines_preview,
                                )
                                .m2m(DraftPurchaseBulkEditFormField::Taxes, &self.tax_items),
                        )))
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
pub struct DraftPurchaseDetailPage {
    pub id: i64,
    pub number: String,
    pub reference: String,
    pub payment_reference: String,
    pub bank_account: String,
    pub remarks: String,
    pub datetime: String,
    pub delivery_date: String,
    pub vendor_is_individual: bool,
    pub vendor_id: i64,
    pub vendor_name: String,
    pub payment_term_rows: Vec<PaymentTermLineDisplayRow>,
    pub tax_labels: String,
    pub extra_detail: String,
    pub line_rows: Vec<PurchaseLineDisplayRow>,
    pub can_edit: bool,
    pub error: Option<String>,
}

impl DraftPurchaseDetailPage {
    fn body(&self) -> Markup {
        let actions = html! {
            (button_modal_route(DraftPurchasePdfModalRouteTag::new(self.id), "PDF", "btn-outline"))
            @if self.can_edit {
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_purchases.DraftPurchaseEditForm",
                    href: &DraftPurchaseEditGetRouteTag::new(self.id).url(),
                    form_post_url: &DraftPurchaseEditPostRouteTag::new(self.id).path(),
                    modal_uid: DraftPurchaseEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
                (button_delete_post_route(
                    DraftPurchasePostRouteTag::new(self.id),
                    ButtonDeletePost {
                        label: "Post purchase",
                        confirm: "Post this draft purchase? This will create a posted purchase.",
                        classes: "btn-primary",
                    },
                ))
            }
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &format!("Draft purchase #{}", self.id),
                        actions,
                    }))
                    @if let Some(e) = &self.error {
                        p class="text-error mb-2" { (e) }
                    }
                    (label("Number", field_text(FieldText { value: &self.number, classes: "" })))
                    (label("Reference", field_text(FieldText { value: &self.reference, classes: "" })))
                    (label("Payment reference", field_text(FieldText { value: &self.payment_reference, classes: "" })))
                    (label("Account", field_textarea(FieldTextarea { value: &self.bank_account, classes: "" })))
                    (label("Remarks", field_textarea(FieldTextarea { value: &self.remarks, classes: "" })))
                    (label("Date", field_text(FieldText { value: &self.datetime, classes: "" })))
                    (label("Delivery date", field_text(FieldText { value: &self.delivery_date, classes: "" })))
                    (label("Vendor", vendor_link(self.vendor_is_individual, self.vendor_id, &self.vendor_name)))
                    (label("Payment schedule", field_payment_term_schedule(&self.payment_term_rows)))
                    (label("Taxes", field_text(FieldText { value: &self.tax_labels, classes: "" })))
                    (PreEscaped(&self.extra_detail))
                    (field_purchase_lines(&self.line_rows))
                }))
            }))
        }
    }

    fn menu(&self) -> Markup {
        draft_purchase_detail_menu(self.id, &self.number)
    }
}

impl RenderAppPane for DraftPurchaseDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let label = draft_purchase_label(self.id, &self.number);
        let detail_url = DraftPurchaseDetailRouteTag::new(self.id).url();
        let crumbs = purchase_section_crumbs(&label, &detail_url, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        let label = draft_purchase_label(self.id, &self.number);
        let detail_url = DraftPurchaseDetailRouteTag::new(self.id).url();
        layout_main_with_crumbs(
            purchase_section_crumbs(&label, &detail_url, None),
            self.body(),
        )
    }
}

impl RenderTemplate for DraftPurchaseDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = draft_purchase_label(self.id, &self.number);
        let detail_url = DraftPurchaseDetailRouteTag::new(self.id).url();
        let crumbs = purchase_section_crumbs(&label, &detail_url, None);
        app_scaffold_with_sidebar("Draft Purchase", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Generic)]
pub struct PostedPurchaseDetailPage {
    pub id: i64,
    pub number: String,
    pub reference: String,
    pub payment_reference: String,
    pub bank_account: String,
    pub remarks: String,
    pub datetime: String,
    pub delivery_date: String,
    pub vendor_is_individual: bool,
    pub vendor_id: i64,
    pub vendor_name: String,
    pub payment_term_rows: Vec<PaymentTermLineDisplayRow>,
    pub tax_labels: String,
    pub line_rows: Vec<PurchaseLineDisplayRow>,
    pub journal_entry_id: i64,
    pub can_edit: bool,
}

impl PostedPurchaseDetailPage {
    fn body(&self) -> Markup {
        let actions = html! {
            (button_modal_route(PostedPurchasePdfModalRouteTag::new(self.id), "PDF", "btn-outline"))
            @if self.can_edit {
                a class="btn btn-error" href=(PostedPurchaseCancelGetRouteTag::new(self.id).url()) { "Cancel" }
            }
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &format!("Posted purchase {}", self.number),
                        actions,
                    }))
                    (label("Reference", field_text(FieldText { value: &self.reference, classes: "" })))
                    (label("Payment reference", field_text(FieldText { value: &self.payment_reference, classes: "" })))
                    (label("Account", field_textarea(FieldTextarea { value: &self.bank_account, classes: "" })))
                    (label("Remarks", field_textarea(FieldTextarea { value: &self.remarks, classes: "" })))
                    (label("Date", field_text(FieldText { value: &self.datetime, classes: "" })))
                    (label("Delivery date", field_text(FieldText { value: &self.delivery_date, classes: "" })))
                    (label("Vendor", vendor_link(self.vendor_is_individual, self.vendor_id, &self.vendor_name)))
                    (label("Payment schedule", field_payment_term_schedule(&self.payment_term_rows)))
                    (label("Taxes", field_text(FieldText { value: &self.tax_labels, classes: "" })))
                    (label("Journal entry", journal_entry_link(self.journal_entry_id)))
                    (field_purchase_lines(&self.line_rows))
                }))
            }))
        }
    }

    fn menu(&self) -> Markup {
        posted_purchase_detail_menu(self.id, &self.number)
    }
}

impl RenderAppPane for PostedPurchaseDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let label = purchase_number_label(self.id, &self.number);
        let detail_url = PostedPurchaseDetailRouteTag::new(self.id).url();
        let crumbs = purchase_section_crumbs(&label, &detail_url, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        let label = purchase_number_label(self.id, &self.number);
        let detail_url = PostedPurchaseDetailRouteTag::new(self.id).url();
        layout_main_with_crumbs(
            purchase_section_crumbs(&label, &detail_url, None),
            self.body(),
        )
    }
}

impl RenderTemplate for PostedPurchaseDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = purchase_number_label(self.id, &self.number);
        let detail_url = PostedPurchaseDetailRouteTag::new(self.id).url();
        let crumbs = purchase_section_crumbs(&label, &detail_url, None);
        app_scaffold_with_sidebar("Posted Purchase", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Clone)]
pub struct CancelledPurchaseDetailPage {
    pub id: i64,
    pub number: String,
    pub reference: String,
    pub payment_reference: String,
    pub bank_account: String,
    pub remarks: String,
    pub datetime: String,
    pub delivery_date: String,
    pub vendor_is_individual: bool,
    pub vendor_id: i64,
    pub vendor_name: String,
    pub payment_term_rows: Vec<PaymentTermLineDisplayRow>,
    pub tax_labels: String,
    pub line_rows: Vec<PurchaseLineDisplayRow>,
    pub posted_purchase_label: String,
    pub posted_purchase_href: Option<String>,
    pub reason: String,
    pub reversal_label: String,
    pub reversal_href: Option<String>,
    pub can_edit: bool,
}

impl CancelledPurchaseDetailPage {
    fn body(&self) -> Markup {
        let actions = html! {
            (button_modal_route(CancelledPurchasePdfModalRouteTag::new(self.id), "PDF", "btn-outline"))
            @if self.can_edit {
                (button_delete_post_route(
                    CancelledPurchaseNewDraftRouteTag::new(self.id),
                    ButtonDeletePost {
                        label: "New draft from cancelled",
                        confirm: "Create a new draft purchase from this cancelled purchase? The cancelled record will be unchanged.",
                        classes: "btn-primary",
                    },
                ))
            }
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &format!("Cancelled purchase {}", self.number),
                        actions,
                    }))
                    (label("Reference", field_text(FieldText { value: &self.reference, classes: "" })))
                    (label("Payment reference", field_text(FieldText { value: &self.payment_reference, classes: "" })))
                    (label("Account", field_textarea(FieldTextarea { value: &self.bank_account, classes: "" })))
                    (label("Remarks", field_textarea(FieldTextarea { value: &self.remarks, classes: "" })))
                    (label("Date", field_text(FieldText { value: &self.datetime, classes: "" })))
                    (label("Delivery date", field_text(FieldText { value: &self.delivery_date, classes: "" })))
                    (label("Vendor", vendor_link(self.vendor_is_individual, self.vendor_id, &self.vendor_name)))
                    (label("Payment schedule", field_payment_term_schedule(&self.payment_term_rows)))
                    (label("Taxes", field_text(FieldText { value: &self.tax_labels, classes: "" })))
                    (label("Reason", field_textarea(FieldTextarea { value: &self.reason, classes: "" })))
                    (label("Posted purchase", cancelled_detail_link(&self.posted_purchase_href, &self.posted_purchase_label)))
                    (label("Reversal", cancelled_detail_link(&self.reversal_href, &self.reversal_label)))
                    (field_purchase_lines(&self.line_rows))
                }))
            }))
        }
    }

    fn menu(&self) -> Markup {
        cancelled_purchase_detail_menu(self.id, &self.number)
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

fn vendor_link(vendor_is_individual: bool, party_id: i64, vendor_name: &str) -> Markup {
    if party_id > 0 {
        let href = if vendor_is_individual {
            ContactDetailRouteTag::new(party_id).url()
        } else {
            CompanyDetailRouteTag::new(party_id).url()
        };
        field_link(FieldLink {
            href: &href,
            label: vendor_name,
            classes: "link link-hover",
        })
    } else if vendor_name.is_empty() {
        field_text(FieldText {
            value: "—",
            classes: "",
        })
    } else {
        field_text(FieldText {
            value: vendor_name,
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

impl RenderAppPane for CancelledPurchaseDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let label = purchase_number_label(self.id, &self.number);
        let detail_url = CancelledPurchaseDetailRouteTag::new(self.id).url();
        let crumbs = purchase_section_crumbs(&label, &detail_url, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        let label = purchase_number_label(self.id, &self.number);
        let detail_url = CancelledPurchaseDetailRouteTag::new(self.id).url();
        layout_main_with_crumbs(
            purchase_section_crumbs(&label, &detail_url, None),
            self.body(),
        )
    }
}

impl RenderTemplate for CancelledPurchaseDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = purchase_number_label(self.id, &self.number);
        let detail_url = CancelledPurchaseDetailRouteTag::new(self.id).url();
        let crumbs = purchase_section_crumbs(&label, &detail_url, None);
        app_scaffold_with_sidebar(
            "Cancelled Purchase",
            chrome,
            self.menu(),
            crumbs,
            self.body(),
        )
    }
}

#[derive(Clone)]
pub struct DraftPurchaseSelectRow {
    pub id: i64,
    pub number: String,
    pub datetime: String,
    pub vendor_name: String,
}

#[derive(Generic)]
pub struct DraftPurchaseSelectPage {
    pub purchases: ObjectList<DraftPurchaseSelectRow>,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl RenderPickerSelect<DraftPurchaseSelectTableKey, DraftPurchaseSelectModalKey>
    for DraftPurchaseSelectPage
{
    fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "Purchases"
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
                key: "Vendor",
                label: "Vendor",
                sort_url: None,
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .purchases
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
                        value: &inv.vendor_name,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let pagination = render_pagination::<DraftPurchaseSelectTableKey>(
            &self.path_and_query,
            self.purchases.number,
            self.purchases.num_pages,
        );
        data_table_list_refresh::<DraftPurchaseSelectTableKey>(
            "Select purchases",
            html! {},
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for DraftPurchaseSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

#[derive(Generic)]
pub struct CancelPurchasePage {
    pub id: i64,
    pub form: CancelPurchaseForm,
    pub can_edit: bool,
}

impl CancelPurchasePage {
    fn body(&self) -> Markup {
        html! {
            (field_title(FieldTitle { value: &format!("Cancel posted purchase #{}", self.id), classes: "" }))
            form method="post" action=(PostedPurchaseCancelRouteTag::new(self.id).url()) {
                (csrf_hidden_field(&CsrfToken::current()))
                (CancelPurchaseForm::render_inputs(
                    &FormCtx::form::<CancelPurchaseForm>(CsrfToken::current())
                        .value(CancelPurchaseFormField::Reason, &self.form.reason),
                ))
                (button_submit(ButtonSubmit { label: "Cancel purchase", ..Default::default() }))
            }
        }
    }

    fn menu(&self) -> Markup {
        posted_purchase_detail_menu(self.id, "")
    }
}

impl RenderAppPane for CancelPurchasePage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let label = purchase_number_label(self.id, "");
        let detail_url = PostedPurchaseDetailRouteTag::new(self.id).url();
        let crumbs = purchase_section_crumbs(&label, &detail_url, Some("Cancel"));
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        let label = purchase_number_label(self.id, "");
        let detail_url = PostedPurchaseDetailRouteTag::new(self.id).url();
        layout_main_with_crumbs(
            purchase_section_crumbs(&label, &detail_url, Some("Cancel")),
            self.body(),
        )
    }
}

impl RenderTemplate for CancelPurchasePage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = purchase_number_label(self.id, "");
        let detail_url = PostedPurchaseDetailRouteTag::new(self.id).url();
        let crumbs = purchase_section_crumbs(&label, &detail_url, Some("Cancel"));
        app_scaffold_with_sidebar("Cancel Purchase", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Generic)]
pub struct CancelBulkPurchasePage {
    pub ids: String,
    pub count: usize,
    pub form: CancelPurchaseForm,
    pub can_edit: bool,
    pub error: String,
}

impl CancelBulkPurchasePage {
    fn title(&self) -> String {
        if self.count == 1 {
            "Cancel 1 posted purchase".into()
        } else {
            format!("Cancel {} posted purchases", self.count)
        }
    }

    fn body(&self) -> Markup {
        let title = self.title();
        let post_url = PostedPurchaseBulkCancelPostRouteTag.url();
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
                (CancelPurchaseForm::render_inputs(
                    &FormCtx::form::<CancelPurchaseForm>(CsrfToken::current())
                        .value(CancelPurchaseFormField::Reason, &self.form.reason),
                ))
                (button_submit(ButtonSubmit {
                    label: "Cancel purchases",
                    classes: "btn-error",
                    ..Default::default()
                }))
                (PreEscaped("</form>"))
            }
        }
    }
}

impl RenderAppPane for CancelBulkPurchasePage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &PurchaseDefaultRouteTag.url(),
            purchases_list_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(purchases_list_crumbs(), self.body())
    }
}

impl RenderTemplate for CancelBulkPurchasePage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Cancel Purchases",
            chrome,
            purchases_list_crumbs(),
            self.body(),
            &PurchaseDefaultRouteTag.url(),
        )
    }
}

#[derive(Generic)]
pub struct PurchasePreferencesPage {
    pub form: PurchasePreferencesForm,
    pub can_edit: bool,
}

impl PurchasePreferencesPage {
    fn body(&self) -> Markup {
        html! {
            (field_title(FieldTitle { value: "Purchase preferences", classes: "" }))
            form method="post" action=(PurchasePreferencesPostRouteTag.url()) {
                (PurchasePreferencesForm::render_inputs(&FormCtx::form::<PurchasePreferencesForm>(CsrfToken::current())
                    .value(
                        PurchasePreferencesFormField::AccountPayableId,
                        &self.form.account_payable_id,
                    )
                    .value(
                        PurchasePreferencesFormField::AccountExpenseId,
                        &self.form.account_expense_id,
                    )
                    .value(
                        PurchasePreferencesFormField::AccountInputTaxId,
                        &self.form.account_input_tax_id,
                    )
                    .value(PurchasePreferencesFormField::JournalId, &self.form.journal_id)))
                (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
            }
        }
    }
}

impl RenderAppPane for PurchasePreferencesPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &PurchasePreferencesRouteTag.url(),
            purchase_preferences_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(purchase_preferences_crumbs(), self.body())
    }
}

impl RenderTemplate for PurchasePreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Purchase Preferences",
            chrome,
            purchase_preferences_crumbs(),
            self.body(),
            &PurchaseDefaultRouteTag.url(),
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
            format!("#{}", DraftPurchaseDeleteModalKey::ID)
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            DraftPurchaseDeleteModalKey::ID
        } else {
            self.modal_uid.as_str()
        };
        let post_url = DraftPurchaseDeletePostRouteTag::new(self.id).url();
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
            format!("#{}", DraftPurchaseBulkDeleteModalKey::ID)
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            DraftPurchaseBulkDeleteModalKey::ID
        } else {
            self.modal_uid.as_str()
        };
        let post_url = DraftPurchaseBulkDeletePostRouteTag.url();
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
