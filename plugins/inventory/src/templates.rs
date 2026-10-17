//! Maud pages for stocks, movements, and movement lines.

use frunk::Generic;
use lariv_formula::VariableType;
use maud::{Markup, PreEscaped, html};

use lariv_core::components::table::table_grid_content;
use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DataTable, DataTableDisplay, DeleteConfirmation, DetailHeader,
    FieldText, FilterClauseView, FormOpts, LayoutMain, LayoutSidebar, ListFilterBuilder,
    ManyToManyItem, ObjectList, PaginationPage, ShellChrome, ShellScaffold, SidebarMenu,
    SidebarMenuItem, SlotCapability, SlotRegistrar, SwapKey, TableButtonCreate, TableButtonFilter,
    TableColumnHeader, TableListContent, TablePagination, TableRow, button_modal_form,
    button_modal_route, button_submit, column_sort_url, container_column, data_table,
    data_table_list_refresh, data_table_rows, delete_confirmation, detail, detail_header,
    field_text, filter_error_banner, form, form_hx_get_picker_route, form_hx_get_route,
    form_hx_post_main, form_hx_post_selector, form_hx_post_url, label, layout_main, layout_sidebar,
    list_filter_builder, modal, modal_keyed, page_size_only_filter_form_with_extras,
    pagination_pages, row_attr_navigate, row_attr_select_extra, row_attr_select_multi,
    shell_scaffold, sidebar_menu, sidebar_menu_item_pane, sort_indicator, table_button_columns,
    table_button_create, table_button_filter, table_create_button, table_list_content,
    table_pagination, table_pagination_picker,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::http::{FkSelectGet, RouteUrl};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::picker::{RenderPickerSelect, picker_create_button};
use lariv_core::template::{
    RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar,
};
use lariv_core::web::{
    modal_create_href_for_picker, modal_create_href_for_table, modal_create_post_query,
    modal_edit_post_url,
};

use super::movement_lines::{MovementLineDisplay, field_movement_lines};

use super::crumbs::{movement_crumbs, movements_list_crumbs, stock_crumbs, stocks_list_crumbs};
use super::detail_menu::{movement_detail_menu, stock_detail_menu};
use super::forms::{
    InventoryPreferencesForm, InventoryPreferencesFormField, MovementPartyForm,
    MovementPartyFormField, ProcessMovementForm, ProcessMovementFormField, StockForm,
    StockFormField,
};

fn data_table_with_filter_errors<K: SwapKey>(
    title: &str,
    actions: Markup,
    headers: &[TableColumnHeader<'_>],
    rows: &[TableRow],
    pagination: Markup,
    refresh_url: &str,
    errors: &[FilterRowError],
    instance_id: Option<&str>,
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
    let body_only = instance_id.is_some();
    data_table(DataTable {
        uid: K::ID,
        title: if body_only { "" } else { title },
        classes: "w-full",
        actions: if body_only {
            Markup::default()
        } else {
            toolbar
        },
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
        refresh_url: if body_only { "" } else { refresh_url },
        column_keys: &column_keys,
        instance_id: instance_id.unwrap_or(""),
        body_only,
        ..Default::default()
    })
}
use super::keys::{
    MovementCreateModalKey, MovementDeleteModalKey, MovementEditModalKey, MovementInSelectModalKey,
    MovementInSelectTableKey, MovementProcessSelectModalKey, MovementProcessSelectTableKey,
    MovementTableKey, StockCreateModalKey, StockDeleteModalKey, StockEditModalKey,
    StockLinesTableKey, StockSelectModalKey, StockSelectTableKey, StockTableKey,
};
use super::routes::{
    InventoryMutate, MovementCreateGetRouteTag, MovementCreateInPostRouteTag,
    MovementCreateOutPostRouteTag, MovementCreateProcessPostRouteTag, MovementDefaultRouteTag,
    MovementDeleteGetRouteTag, MovementEditGetRouteTag, MovementEditInPostRouteTag,
    MovementEditOutPostRouteTag, MovementEditProcessPostRouteTag, MovementInFkSelectRouteTag,
    MovementPdfModalRouteTag, MovementProcessFkSelectRouteTag, StockCreatePostRouteTag,
    StockDefaultRouteTag, StockDeleteGetRouteTag, StockEditGetRouteTag, StockEditPostRouteTag,
    StockFkSelectRouteTag,
};

lariv_core::define_register_items! {
    plugin: InventoryTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        StockListIdx: StockListPageTag => StockListPage,
        StockDetailIdx: StockDetailPageTag => StockDetailPage,
        StockEditModalIdx: StockEditModalPageTag => StockEditModalPage,
        StockCreateModalIdx: StockCreateModalPageTag => StockCreateModalPage,
        StockSelectIdx: StockSelectPageTag => StockSelectPage,
        MovementListIdx: MovementListPageTag => MovementListPage,
        MovementDetailIdx: MovementDetailPageTag => MovementDetailPage,
        MovementEditModalIdx: MovementEditModalPageTag => MovementEditModalPage,
        MovementCreateModalIdx: MovementCreateModalPageTag => MovementCreateModalPage,
        ConfirmDeleteIdx: InventoryConfirmDeletePageTag => ConfirmDeletePage,
        InventoryPreferencesIdx: InventoryPreferencesPageTag => InventoryPreferencesPage,
    ]
}

lariv_core::define_register_items! {
    plugin: InventoryTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

fn app_scaffold(
    title: &str,
    chrome: &ShellChrome,
    sidebar: Markup,
    crumbs: Markup,
    body: Markup,
) -> Markup {
    shell_scaffold(ShellScaffold {
        title,
        registry_head: chrome.head.clone(),
        topbar_items: chrome.topbar_items.clone(),
        right_sidebar: chrome.right_sidebar.clone(),
        sidebar,
        breadcrumbs: crumbs,
        body,
        ..Default::default()
    })
}

fn scaffold_pane(
    sidebar: Markup,
    crumbs: Markup,
    body: Markup,
) -> lariv_core::components::AppLayoutHtml {
    layout_sidebar(LayoutSidebar {
        sidebar,
        breadcrumbs: crumbs,
        content: body,
    })
}

fn scaffold_main(crumbs: Markup, body: Markup) -> lariv_core::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

fn inventory_menu(active: &str) -> Markup {
    sidebar_menu(SidebarMenu {
        title: "Inventory",
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Stocks",
                url: &StockDefaultRouteTag.url(),
                active: active == "stocks",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Movements",
                url: &MovementDefaultRouteTag.url(),
                active: active == "movements",
                ..Default::default()
            }))
            @if can_mutate() {
                (sidebar_menu_item_pane(SidebarMenuItem {
                    title: "Preferences",
                    url: &crate::routes::InventoryPrefsGetRouteTag.url(),
                    active: active == "preferences",
                    ..Default::default()
                }))
            }
        },
    })
}

fn can_mutate() -> bool {
    lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<
        InventoryMutate,
    >())
}

fn fk_value(id: i64) -> String {
    if id <= 0 {
        String::new()
    } else {
        id.to_string()
    }
}

fn qty_type_choices() -> Vec<(String, String)> {
    VariableType::all()
        .iter()
        .map(|ty| (ty.as_str().to_string(), ty.label().to_string()))
        .collect()
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

fn render_picker_pagination<M: SwapKey>(
    path_and_query: &str,
    number: u32,
    num_pages: u32,
) -> Markup {
    let owned = pagination_pages(path_and_query, number, num_pages, false);
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
    table_pagination_picker(TablePagination {
        pages: &pages,
        hx_target: M::SELECTOR,
    })
}

fn stock_qty_type_token(raw: &str) -> &'static str {
    VariableType::parse_name(raw)
        .map(|ty| ty.as_str())
        .unwrap_or("quantity")
}

fn stock_form_inputs(
    name: &str,
    company_id: i64,
    company_display: &str,
    qty_type: &str,
    qty_unit: &str,
) -> Markup {
    let types = qty_type_choices();
    let qty_type = stock_qty_type_token(qty_type);
    let x_data = format!("{{ qtyType: '{qty_type}' }}");
    StockForm::render_inputs(
        &FormCtx::form::<StockForm>(CsrfToken::current())
            .x_data(&x_data)
            .value(StockFormField::Name, name)
            .value(StockFormField::CompanyId, fk_value(company_id))
            .display(StockFormField::CompanyId, company_display)
            .value(StockFormField::QtyType, qty_type)
            .choices(StockFormField::QtyType, &types)
            .value(StockFormField::QtyUnit, qty_unit),
    )
}

struct MovementFormValues<'a> {
    kind: &'a str,
    number: &'a str,
    datetime: &'a str,
    vehicle_out_datetime: &'a str,
    customer_id: &'a str,
    customer_display: &'a str,
    vehicle_type: &'a str,
    vehicle_number: &'a str,
    eway_bill: &'a str,
    driver_id: &'a str,
    driver_display: &'a str,
    source_ins: &'a [ManyToManyItem],
    source_processes: &'a [ManyToManyItem],
    delivery_challan: &'a str,
    delivery_challan_display: &'a str,
    lines_json: &'a str,
}

fn movement_form_inputs(values: MovementFormValues<'_>) -> Markup {
    let x_data = format!("{{ movementKind: '{}' }}", values.kind);
    MovementPartyForm::render_inputs(
        &FormCtx::form::<MovementPartyForm>(CsrfToken::current())
            .x_data(&x_data)
            .value(MovementPartyFormField::MovementKind, values.kind)
            .value(MovementPartyFormField::Number, values.number)
            .value(MovementPartyFormField::Datetime, values.datetime)
            .value(
                MovementPartyFormField::VehicleOutDatetime,
                values.vehicle_out_datetime,
            )
            .value(MovementPartyFormField::CustomerId, values.customer_id)
            .display(MovementPartyFormField::CustomerId, values.customer_display)
            .value(MovementPartyFormField::VehicleType, values.vehicle_type)
            .value(MovementPartyFormField::VehicleNumber, values.vehicle_number)
            .value(MovementPartyFormField::EwayBill, values.eway_bill)
            .value(MovementPartyFormField::DriverId, values.driver_id)
            .display(MovementPartyFormField::DriverId, values.driver_display)
            .m2m(MovementPartyFormField::SourceInIds, values.source_ins)
            .m2m(
                MovementPartyFormField::SourceProcessIds,
                values.source_processes,
            )
            .value(
                MovementPartyFormField::DeliveryChallan,
                values.delivery_challan,
            )
            .display(
                MovementPartyFormField::DeliveryChallan,
                values.delivery_challan_display,
            )
            .value(MovementPartyFormField::LinesJson, values.lines_json),
    )
}

fn process_form_inputs(
    number: &str,
    datetime: &str,
    input_lines_json: &str,
    output_lines_json: &str,
) -> Markup {
    ProcessMovementForm::render_inputs(
        &FormCtx::form::<ProcessMovementForm>(CsrfToken::current())
            .value(ProcessMovementFormField::Number, number)
            .value(ProcessMovementFormField::Datetime, datetime)
            .value(ProcessMovementFormField::InputLinesJson, input_lines_json)
            .value(ProcessMovementFormField::OutputLinesJson, output_lines_json),
    )
}

#[derive(Clone)]
pub struct StockRow {
    pub id: i64,
    pub name: String,
    pub company: String,
    pub qty_type: String,
    pub qty_unit: String,
    pub qty: String,
    pub detail_href: String,
}

#[derive(Generic)]
pub struct StockListPage {
    pub stocks: ObjectList<StockRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl StockListPage {
    pub fn render_table(&self) -> Markup {
        self.render_table_inner(None)
    }

    pub fn render_table_rows(&self, instance_id: &str) -> Markup {
        self.render_table_inner(Some(instance_id))
    }

    fn render_table_inner(&self, rows_instance: Option<&str>) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Company",
                label: "Company",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Type",
                label: "Type",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Unit",
                label: "Unit",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Qty",
                label: "Qty",
                sort_url: None,
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .stocks
            .items
            .iter()
            .map(|row| TableRow {
                attrs: row_attr_navigate(&row.detail_href),
                cells: vec![
                    field_text(FieldText {
                        value: &row.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.company,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.qty_type,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.qty_unit,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.qty,
                        classes: "",
                    }),
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let pagination = render_pagination::<StockTableKey>(
            &self.path_and_query,
            self.stocks.number,
            self.stocks.num_pages,
        );
        if let Some(instance_id) = rows_instance {
            return data_table_with_filter_errors::<StockTableKey>(
                "",
                html! {},
                &headers,
                &rows,
                pagination,
                "",
                &self.filter_errors,
                Some(instance_id),
            );
        }
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route::<StockTableKey, StockDefaultRouteTag>(
                        StockDefaultRouteTag,
                    ),
                    apply_label: "Apply",
                    extra: html! {},
                }),
                ..Default::default()
            }))
        };
        if can_mutate() {
            actions = html! {
                (actions)
                (table_create_button::<StockTableKey, StockCreateModalKey>(
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        data_table_with_filter_errors::<StockTableKey>(
            "Stocks",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
            None,
        )
    }
}

impl RenderAppPane for StockListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            inventory_menu("stocks"),
            stocks_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(stocks_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for StockListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Stocks — Lariv",
            chrome,
            inventory_menu("stocks"),
            stocks_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Clone)]
pub struct StockLineRow {
    pub when: String,
    pub movement_type: String,
    pub qty: String,
    pub qty_type: String,
    pub detail_href: String,
}

#[derive(Generic)]
pub struct StockDetailPage {
    pub id: i64,
    pub name: String,
    pub company: String,
    pub qty_type: String,
    pub qty_unit: String,
    pub qty: String,
    pub lines: ObjectList<StockLineRow>,
    pub path_and_query: String,
    pub page_size: u32,
}

impl StockDetailPage {
    pub fn render_lines(&self) -> Markup {
        self.render_lines_inner(None)
    }

    pub fn render_lines_rows(&self, instance_id: &str) -> Markup {
        self.render_lines_inner(Some(instance_id))
    }

    fn render_lines_inner(&self, rows_instance: Option<&str>) -> Markup {
        let headers = [
            TableColumnHeader {
                key: "When",
                label: "When",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Type",
                label: "Type",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Qty",
                label: "Qty",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "QtyType",
                label: "Qty type",
                sort_url: None,
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .lines
            .items
            .iter()
            .map(|row| TableRow {
                attrs: row_attr_navigate(&row.detail_href),
                cells: vec![
                    field_text(FieldText {
                        value: &row.when,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.movement_type,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.qty,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.qty_type,
                        classes: "",
                    }),
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let pagination = render_pagination::<StockLinesTableKey>(
            &self.path_and_query,
            self.lines.number,
            self.lines.num_pages,
        );
        if let Some(instance_id) = rows_instance {
            return data_table_rows::<StockLinesTableKey>(&headers, &rows, pagination, instance_id);
        }
        data_table_list_refresh::<StockLinesTableKey>(
            "Movements",
            html! {},
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }

    fn actions(&self) -> Markup {
        if can_mutate() {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_inventory.StockEditForm",
                    href: &StockEditGetRouteTag::new(self.id).url(),
                    form_post_url: &StockEditPostRouteTag::new(self.id).path(),
                    modal_uid: StockEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        }
    }

    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.name,
                        actions: self.actions(),
                    }))
                    (label("Company", field_text(FieldText { value: &self.company, classes: "" })))
                    (label("Type", field_text(FieldText { value: &self.qty_type, classes: "" })))
                    (label("Unit", field_text(FieldText { value: &self.qty_unit, classes: "" })))
                    (label("Qty", field_text(FieldText { value: &self.qty, classes: "" })))
                }))
            }))
            div class="mt-6" { (self.render_lines()) }
        }
    }
}

impl RenderAppPane for StockDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            stock_detail_menu(&self.name, self.id),
            stock_crumbs(&self.name, self.id),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(stock_crumbs(&self.name, self.id), self.body())
    }
}

impl RenderTemplate for StockDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Stock — Lariv",
            chrome,
            stock_detail_menu(&self.name, self.id),
            stock_crumbs(&self.name, self.id),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct StockCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub name: String,
    pub company_id: i64,
    pub company_display: String,
    pub qty_type: String,
    pub qty_unit: String,
    pub error: String,
}

impl RenderTemplate for StockCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<StockCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New stock" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<StockCreateModalKey>(&lariv_core::web::modal_create_post_query(
                        StockCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                        &self.target_input,
                    )),
                    form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                    inputs: stock_form_inputs(&self.name, self.company_id, &self.company_display, &self.qty_type, &self.qty_unit),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create stock", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct StockEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub company_id: i64,
    pub company_display: String,
    pub qty_type: String,
    pub qty_unit: String,
    pub error: String,
}

impl RenderTemplate for StockEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = StockDeleteGetRouteTag::new(self.id).url();
        modal_keyed::<StockEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit stock" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<StockEditModalKey>(&modal_edit_post_url(
                        StockEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                    inputs: stock_form_inputs(&self.name, self.company_id, &self.company_display, &self.qty_type, &self.qty_unit),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_inventory.StockDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: StockDeleteModalKey::ID,
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
pub struct StockSelectPage {
    pub stocks: ObjectList<StockRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub target_input: String,
    pub limit_sources: bool,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl RenderPickerSelect<StockSelectTableKey, StockSelectModalKey> for StockSelectPage {
    fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [TableColumnHeader {
            key: "Name",
            label: &name_label,
            sort_url: Some(&name_sort),
            push_url: false,
        }];
        let rows: Vec<TableRow> = self
            .stocks
            .items
            .iter()
            .map(|row| {
                let extra = [
                    ("qty_type", row.qty_type.as_str()),
                    ("qty_unit", row.qty_unit.as_str()),
                ];
                TableRow {
                    attrs: row_attr_select_extra(
                        &self.target_input,
                        &row.id.to_string(),
                        &row.name,
                        &extra,
                    ),
                    cells: vec![field_text(FieldText {
                        value: &row.name,
                        classes: "",
                    })],
                    key: String::new(),
                    children: Vec::new(),
                }
            })
            .collect();
        let target_input = self.target_input.clone();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_picker_route::<
                        StockSelectTableKey,
                        StockSelectModalKey,
                        StockFkSelectRouteTag,
                    >(StockFkSelectRouteTag)
                    .set("hx-push-url", "false"),
                    apply_label: "Apply",
                    extra: html! {
                        input type="hidden" name="target_input" value=(target_input) {}
                    },
                }),
                ..Default::default()
            }))
        };
        if can_mutate() && !self.limit_sources {
            actions = html! {
                (actions)
                (picker_create_button::<StockCreateModalKey>(&self.target_input, Some("plus"), "btn-square btn-outline btn-sm"))
            };
        }
        data_table_with_filter_errors::<StockSelectTableKey>(
            "Select stock",
            actions,
            &headers,
            &rows,
            render_picker_pagination::<StockSelectModalKey>(
                &self.path_and_query,
                self.stocks.number,
                self.stocks.num_pages,
            ),
            &self.path_and_query,
            &self.filter_errors,
            None,
        )
    }
}

impl RenderTemplate for StockSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

#[derive(Clone)]
pub struct MovementRow {
    pub id: i64,
    pub number: String,
    pub customer: String,
    pub customer_href: String,
    pub datetime: String,
    pub vehicle: String,
    pub driver: String,
    pub created_by: String,
    pub created_at: String,
    pub sources: Vec<(String, String)>,
    pub detail_href: String,
}

fn movement_create_opener(href: &str) -> Markup {
    table_button_create(TableButtonCreate {
        href,
        modal_uid: MovementCreateModalKey::ID,
        ..Default::default()
    })
}

fn movement_create_button(kind: &str) -> Markup {
    movement_create_opener(&modal_create_href_for_table::<MovementTableKey>(
        &MovementCreateGetRouteTag::new(kind.to_string()).url(),
        "p_inventory.MovementCreateForm",
    ))
}

fn movement_picker_create_button(kind: &str, target_input: &str) -> Markup {
    movement_create_opener(&modal_create_href_for_picker(
        &MovementCreateGetRouteTag::new(kind.to_string()).url(),
        "p_inventory.MovementCreateForm",
        target_input,
    ))
}

fn movement_tab_href(tab: &str) -> String {
    lariv_core::http::RouteQueryBuilder::new(MovementDefaultRouteTag)
        .query("tab", tab)
        .build()
}

fn source_links(sources: &[(String, String)]) -> Markup {
    if sources.is_empty() {
        return field_text(FieldText {
            value: "—",
            classes: "",
        });
    }
    html! {
        @for (label, href) in sources {
            (party_link(label, href))
        }
    }
}

fn party_link(name: &str, href: &str) -> Markup {
    if href.is_empty() {
        field_text(FieldText {
            value: name,
            classes: "",
        })
    } else {
        html! {
            a href=(href) class="link" { (name) }
        }
    }
}

#[derive(Generic)]
pub struct MovementListPage {
    pub tab: String,
    pub movements: ObjectList<MovementRow>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl MovementListPage {
    fn tab_link(&self, tab: &str, label: &str) -> Markup {
        let active = self.tab == tab;
        let cls = if active { "tab tab-active" } else { "tab" };
        let escape_attr = lariv_core::components::attrs::escape_attr;
        let href = movement_tab_href(tab);
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

    fn body(&self) -> Markup {
        html! {
            (container_column("", html! {
                div class="tabs tabs-boxed mb-4" {
                    (self.tab_link("in", "In"))
                    (self.tab_link("out", "Out"))
                    (self.tab_link("process", "Process"))
                }
                (self.render_table())
            }))
        }
    }

    pub fn render_table(&self) -> Markup {
        self.render_table_inner(None)
    }

    pub fn render_table_rows(&self, instance_id: &str) -> Markup {
        self.render_table_inner(Some(instance_id))
    }

    fn render_table_inner(&self, rows_instance: Option<&str>) -> Markup {
        let number_sort = column_sort_url(&self.path_and_query, "Number", &self.sort);
        let when_sort = column_sort_url(&self.path_and_query, "Datetime", &self.sort);
        let created_sort = column_sort_url(&self.path_and_query, "CreatedAt", &self.sort);
        let number_label = format!("Number{}", sort_indicator(&self.sort, "Number"));
        let created_label = format!("Created at{}", sort_indicator(&self.sort, "CreatedAt"));
        let party = self.tab != "process";
        let when_name = if party {
            "Vehicle in time"
        } else {
            "Date & time"
        };
        let when_label = format!("{when_name}{}", sort_indicator(&self.sort, "Datetime"));
        let sourced = self.tab == "out";
        let mut headers = vec![
            TableColumnHeader {
                key: "Number",
                label: &number_label,
                sort_url: Some(&number_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Datetime",
                label: &when_label,
                sort_url: Some(&when_sort),
                push_url: true,
            },
        ];
        if party {
            headers.extend([
                TableColumnHeader {
                    key: "Customer",
                    label: "Customer",
                    sort_url: None,
                    push_url: false,
                },
                TableColumnHeader {
                    key: "Vehicle",
                    label: "Vehicle",
                    sort_url: None,
                    push_url: false,
                },
                TableColumnHeader {
                    key: "Driver",
                    label: "Driver",
                    sort_url: None,
                    push_url: false,
                },
            ]);
        }
        if sourced {
            headers.push(TableColumnHeader {
                key: "From",
                label: "From",
                sort_url: None,
                push_url: false,
            });
        }
        headers.extend([
            TableColumnHeader {
                key: "CreatedBy",
                label: "Created by",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "CreatedAt",
                label: &created_label,
                sort_url: Some(&created_sort),
                push_url: true,
            },
        ]);
        let rows: Vec<TableRow> = self
            .movements
            .items
            .iter()
            .map(|row| {
                let mut cells = vec![
                    field_text(FieldText {
                        value: &row.number,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.datetime,
                        classes: "",
                    }),
                ];
                if party {
                    cells.extend([
                        party_link(&row.customer, &row.customer_href),
                        field_text(FieldText {
                            value: &row.vehicle,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &row.driver,
                            classes: "",
                        }),
                    ]);
                }
                if sourced {
                    cells.push(source_links(&row.sources));
                }
                cells.extend([
                    field_text(FieldText {
                        value: &row.created_by,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.created_at,
                        classes: "",
                    }),
                ]);
                TableRow {
                    attrs: row_attr_navigate(&row.detail_href),
                    cells,
                    key: String::new(),
                    children: Vec::new(),
                }
            })
            .collect();
        let pagination = render_pagination::<MovementTableKey>(
            &self.path_and_query,
            self.movements.number,
            self.movements.num_pages,
        );
        if let Some(instance_id) = rows_instance {
            return data_table_with_filter_errors::<MovementTableKey>(
                "",
                html! {},
                &headers,
                &rows,
                pagination,
                "",
                &[],
                Some(instance_id),
            );
        }
        let filter = table_button_filter(TableButtonFilter {
            panel: page_size_only_filter_form_with_extras::<
                MovementTableKey,
                MovementDefaultRouteTag,
            >(
                self.page_size,
                html! {
                    input type="hidden" name="tab" value=(self.tab.as_str()) {}
                },
            ),
            ..Default::default()
        });
        let create = if can_mutate() {
            movement_create_button(&self.tab)
        } else {
            html! {}
        };
        let actions = html! {
            (filter)
            (create)
        };
        data_table_with_filter_errors::<MovementTableKey>(
            "Movements",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &[],
            None,
        )
    }
}

impl RenderAppPane for MovementListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            inventory_menu("movements"),
            movements_list_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(movements_list_crumbs(), self.body())
    }
}

impl RenderTemplate for MovementListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Movements — Lariv",
            chrome,
            inventory_menu("movements"),
            movements_list_crumbs(),
            self.body(),
        )
    }
}

#[derive(Clone)]
pub struct MovementLineRow {
    pub stock: String,
    pub qty_display: String,
    pub qty_unit: String,
    pub qty_type_label: String,
}

#[derive(Generic)]
pub struct MovementDetailPage {
    pub id: i64,
    pub kind: String,
    pub number: String,
    pub datetime: String,
    pub vehicle_out_datetime: String,
    pub movement_type: String,
    pub show_party: bool,
    pub show_pdf: bool,
    pub show_challan: bool,
    pub customer: String,
    pub customer_href: String,
    pub vehicle: String,
    pub eway_bill: String,
    pub driver: String,
    pub driver_href: String,
    pub delivery_challan: String,
    pub delivery_challan_href: String,
    pub created_by: String,
    pub created_at: String,
    pub show_source: bool,
    pub sources: Vec<(String, String)>,
    pub lines: Vec<MovementLineRow>,
    pub output_lines: Vec<MovementLineRow>,
}

impl MovementDetailPage {
    fn lines_table(&self, lines: &[MovementLineRow]) -> Markup {
        let rows: Vec<MovementLineDisplay<'_>> = lines
            .iter()
            .map(|row| MovementLineDisplay {
                stock: &row.stock,
                qty: &row.qty_display,
                unit: &row.qty_unit,
                qty_type: &row.qty_type_label,
            })
            .collect();
        field_movement_lines(&rows)
    }

    fn actions(&self) -> Markup {
        let edit = button_modal_form(ButtonModalForm {
            name: "p_inventory.MovementEditForm",
            href: &MovementEditGetRouteTag::new(self.kind.clone(), self.id).url(),
            form_post_url: &movement_edit_post(&self.kind, self.id),
            modal_uid: MovementEditModalKey::ID,
            label: "Edit",
            classes: "btn-outline",
            ..Default::default()
        });
        if self.show_pdf {
            let pdf = button_modal_route(
                MovementPdfModalRouteTag::new(self.kind.clone(), self.id),
                "PDF",
                "btn-outline",
            );
            if can_mutate() {
                html! { (pdf) (edit) }
            } else {
                html! { (pdf) }
            }
        } else if can_mutate() {
            html! { (edit) }
        } else {
            html! {}
        }
    }

    fn body(&self) -> Markup {
        let title = if self.number.is_empty() {
            self.datetime.clone()
        } else {
            self.number.clone()
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &title,
                        actions: self.actions(),
                    }))
                    (label("Number", field_text(FieldText { value: &self.number, classes: "" })))
                    (label(
                        if self.show_party { "Vehicle in time" } else { "Date & time" },
                        field_text(FieldText { value: &self.datetime, classes: "" }),
                    ))
                    @if self.show_party {
                        (label("Vehicle out datetime", field_text(FieldText { value: &self.vehicle_out_datetime, classes: "" })))
                    }
                    (label("Type", field_text(FieldText { value: &self.movement_type, classes: "" })))
                    @if self.show_source {
                        (label("From", source_links(&self.sources)))
                    }
                    (label("Created by", field_text(FieldText { value: &self.created_by, classes: "" })))
                    (label("Created at", field_text(FieldText { value: &self.created_at, classes: "" })))
                    @if self.show_party {
                        (label("Customer", party_link(&self.customer, &self.customer_href)))
                        (label("Vehicle", field_text(FieldText { value: &self.vehicle, classes: "" })))
                        (label("E-way bill", field_text(FieldText { value: &self.eway_bill, classes: "" })))
                        (label("Driver", party_link(&self.driver, &self.driver_href)))
                    }
                    @if self.show_challan {
                        (label("Delivery challan", party_link(&self.delivery_challan, &self.delivery_challan_href)))
                    }
                    @if self.kind == "process" {
                        div class="mt-4" {
                            (label("Input", html! {}))
                            (self.lines_table(&self.lines))
                        }
                        div class="mt-4" {
                            (label("Output", html! {}))
                            (self.lines_table(&self.output_lines))
                        }
                    } @else {
                        div class="mt-4" { (self.lines_table(&self.lines)) }
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for MovementDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            movement_detail_menu(&self.number, &self.kind, self.id),
            movement_crumbs(&self.number, &self.kind, self.id),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(
            movement_crumbs(&self.number, &self.kind, self.id),
            self.body(),
        )
    }
}

impl RenderTemplate for MovementDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Movement — Lariv",
            chrome,
            movement_detail_menu(&self.number, &self.kind, self.id),
            movement_crumbs(&self.number, &self.kind, self.id),
            self.body(),
        )
    }
}

fn movement_edit_post(kind: &str, id: i64) -> String {
    match kind {
        "out" => MovementEditOutPostRouteTag::new(id).path(),
        "process" => MovementEditProcessPostRouteTag::new(id).path(),
        _ => MovementEditInPostRouteTag::new(id).path(),
    }
}

fn movement_create_post(kind: &str, form_name: &str, refresh: &str, target_input: &str) -> String {
    match kind {
        "out" => modal_create_post_query(
            MovementCreateOutPostRouteTag,
            form_name,
            refresh,
            target_input,
        ),
        "process" => modal_create_post_query(
            MovementCreateProcessPostRouteTag,
            form_name,
            refresh,
            target_input,
        ),
        _ => modal_create_post_query(
            MovementCreateInPostRouteTag,
            form_name,
            refresh,
            target_input,
        ),
    }
}

fn movement_title(kind: &str, edit: bool) -> String {
    let action = if edit { "Edit" } else { "New" };
    match kind {
        "out" => format!("{action} stock movement out"),
        "process" => format!("{action} process"),
        _ => format!("{action} stock movement in"),
    }
}

#[derive(Generic)]
pub struct MovementCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub kind: String,
    pub number: String,
    pub datetime: String,
    pub vehicle_out_datetime: String,
    pub customer_id: String,
    pub customer_display: String,
    pub vehicle_type: String,
    pub vehicle_number: String,
    pub eway_bill: String,
    pub driver_id: String,
    pub driver_display: String,
    pub source_ins: Vec<ManyToManyItem>,
    pub source_processes: Vec<ManyToManyItem>,
    pub delivery_challan: String,
    pub delivery_challan_display: String,
    pub lines_json: String,
    pub input_lines_json: String,
    pub output_lines_json: String,
    pub error: String,
}

fn movement_values_from_create(page: &MovementCreateModalPage) -> MovementFormValues<'_> {
    MovementFormValues {
        kind: &page.kind,
        number: &page.number,
        datetime: &page.datetime,
        vehicle_out_datetime: &page.vehicle_out_datetime,
        customer_id: &page.customer_id,
        customer_display: &page.customer_display,
        vehicle_type: &page.vehicle_type,
        vehicle_number: &page.vehicle_number,
        eway_bill: &page.eway_bill,
        driver_id: &page.driver_id,
        driver_display: &page.driver_display,
        source_ins: &page.source_ins,
        source_processes: &page.source_processes,
        delivery_challan: &page.delivery_challan,
        delivery_challan_display: &page.delivery_challan_display,
        lines_json: &page.lines_json,
    }
}

impl RenderTemplate for MovementCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let title = movement_title(&self.kind, false);
        let inputs = if self.kind == "process" {
            process_form_inputs(
                &self.number,
                &self.datetime,
                &self.input_lines_json,
                &self.output_lines_json,
            )
        } else {
            movement_form_inputs(movement_values_from_create(self))
        };
        modal_keyed::<MovementCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { (title) }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<MovementCreateModalKey>(&movement_create_post(
                        &self.kind,
                        &self.form_name,
                        &self.refresh_table,
                        &self.target_input,
                    )),
                    form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                    inputs,
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create movement", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct MovementEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub kind: String,
    pub number: String,
    pub datetime: String,
    pub vehicle_out_datetime: String,
    pub customer_id: String,
    pub customer_display: String,
    pub vehicle_type: String,
    pub vehicle_number: String,
    pub eway_bill: String,
    pub driver_id: String,
    pub driver_display: String,
    pub source_ins: Vec<ManyToManyItem>,
    pub source_processes: Vec<ManyToManyItem>,
    pub delivery_challan: String,
    pub delivery_challan_display: String,
    pub lines_json: String,
    pub input_lines_json: String,
    pub output_lines_json: String,
    pub error: String,
}

fn movement_values_from_edit(page: &MovementEditModalPage) -> MovementFormValues<'_> {
    MovementFormValues {
        kind: &page.kind,
        number: &page.number,
        datetime: &page.datetime,
        vehicle_out_datetime: &page.vehicle_out_datetime,
        customer_id: &page.customer_id,
        customer_display: &page.customer_display,
        vehicle_type: &page.vehicle_type,
        vehicle_number: &page.vehicle_number,
        eway_bill: &page.eway_bill,
        driver_id: &page.driver_id,
        driver_display: &page.driver_display,
        source_ins: &page.source_ins,
        source_processes: &page.source_processes,
        delivery_challan: &page.delivery_challan,
        delivery_challan_display: &page.delivery_challan_display,
        lines_json: &page.lines_json,
    }
}

impl RenderTemplate for MovementEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = MovementDeleteGetRouteTag::new(self.kind.clone(), self.id).url();
        let title = movement_title(&self.kind, true);
        let post = match self.kind.as_str() {
            "out" => {
                modal_edit_post_url(MovementEditOutPostRouteTag::new(self.id), &self.form_name)
            }
            "process" => modal_edit_post_url(
                MovementEditProcessPostRouteTag::new(self.id),
                &self.form_name,
            ),
            _ => modal_edit_post_url(MovementEditInPostRouteTag::new(self.id), &self.form_name),
        };
        let inputs = if self.kind == "process" {
            process_form_inputs(
                &self.number,
                &self.datetime,
                &self.input_lines_json,
                &self.output_lines_json,
            )
        } else {
            movement_form_inputs(movement_values_from_edit(self))
        };
        modal_keyed::<MovementEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { (title) }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<MovementEditModalKey>(&post),
                    form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                    inputs,
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_inventory.MovementDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: MovementDeleteModalKey::ID,
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
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub id: i64,
    pub post_url: String,
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", self.modal_uid);
        modal(lariv_core::components::Modal {
            uid: self.modal_uid.as_str(),
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: &self.message,
                attrs: form_hx_post_selector(&self.post_url, &target),
                form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

#[derive(Generic)]
pub struct InventoryPreferencesPage {
    pub movement_number_format: String,
    pub company_name: String,
    pub company_address: String,
    pub company_phone: String,
    pub company_email: String,
    pub company_gstin: String,
    pub terms_and_conditions: String,
    pub logo_vnode_id: String,
    pub logo_display: String,
    pub signature_vnode_id: String,
    pub signature_display: String,
    pub movement_in_template: String,
    pub movement_out_template: String,
    pub error: String,
}

impl InventoryPreferencesPage {
    pub fn from_model(
        prefs: &crate::entities::preferences::Model,
        logo_display: String,
        signature_display: String,
        error: String,
    ) -> Self {
        Self {
            movement_number_format: prefs.movement_number_format.clone().unwrap_or_default(),
            company_name: prefs.company_name.clone().unwrap_or_default(),
            company_address: prefs.company_address.clone().unwrap_or_default(),
            company_phone: prefs.company_phone.clone().unwrap_or_default(),
            company_email: prefs.company_email.clone().unwrap_or_default(),
            company_gstin: prefs.company_gstin.clone().unwrap_or_default(),
            terms_and_conditions: prefs.terms_and_conditions.clone().unwrap_or_default(),
            logo_vnode_id: crate::logic::preferences::id_value(prefs.logo_vnode_id),
            logo_display,
            signature_vnode_id: crate::logic::preferences::id_value(prefs.signature_vnode_id),
            signature_display,
            movement_in_template: prefs.movement_in_template.clone().unwrap_or_default(),
            movement_out_template: prefs.movement_out_template.clone().unwrap_or_default(),
            error,
        }
    }

    pub fn from_form(
        form: InventoryPreferencesForm,
        logo_display: String,
        signature_display: String,
        error: String,
    ) -> Self {
        Self {
            movement_number_format: form.movement_number_format,
            company_name: form.company_name,
            company_address: form.company_address,
            company_phone: form.company_phone,
            company_email: form.company_email,
            company_gstin: form.company_gstin,
            terms_and_conditions: form.terms_and_conditions,
            logo_vnode_id: form.logo_vnode_id,
            logo_display,
            signature_vnode_id: form.signature_vnode_id,
            signature_display,
            movement_in_template: form.movement_in_template,
            movement_out_template: form.movement_out_template,
            error,
        }
    }

    pub fn empty(error: String) -> Self {
        Self {
            movement_number_format: String::new(),
            company_name: String::new(),
            company_address: String::new(),
            company_phone: String::new(),
            company_email: String::new(),
            company_gstin: String::new(),
            terms_and_conditions: String::new(),
            logo_vnode_id: String::new(),
            logo_display: String::new(),
            signature_vnode_id: String::new(),
            signature_display: String::new(),
            movement_in_template: String::new(),
            movement_out_template: String::new(),
            error,
        }
    }

    fn body(&self) -> Markup {
        let inputs = InventoryPreferencesForm::render_inputs(
            &FormCtx::form::<InventoryPreferencesForm>(CsrfToken::current())
                .value(
                    InventoryPreferencesFormField::MovementNumberFormat,
                    &self.movement_number_format,
                )
                .value(
                    InventoryPreferencesFormField::CompanyName,
                    &self.company_name,
                )
                .value(
                    InventoryPreferencesFormField::CompanyAddress,
                    &self.company_address,
                )
                .value(
                    InventoryPreferencesFormField::CompanyPhone,
                    &self.company_phone,
                )
                .value(
                    InventoryPreferencesFormField::CompanyEmail,
                    &self.company_email,
                )
                .value(
                    InventoryPreferencesFormField::CompanyGstin,
                    &self.company_gstin,
                )
                .value(
                    InventoryPreferencesFormField::TermsAndConditions,
                    &self.terms_and_conditions,
                )
                .value(
                    InventoryPreferencesFormField::LogoVnodeId,
                    &self.logo_vnode_id,
                )
                .display(
                    InventoryPreferencesFormField::LogoVnodeId,
                    &self.logo_display,
                )
                .value(
                    InventoryPreferencesFormField::SignatureVnodeId,
                    &self.signature_vnode_id,
                )
                .display(
                    InventoryPreferencesFormField::SignatureVnodeId,
                    &self.signature_display,
                )
                .value(
                    InventoryPreferencesFormField::MovementInTemplate,
                    &self.movement_in_template,
                )
                .value(
                    InventoryPreferencesFormField::MovementOutTemplate,
                    &self.movement_out_template,
                ),
        );
        form(
            &CsrfToken::current(),
            FormOpts {
                attrs: form_hx_post_main(crate::routes::InventoryPrefsPostRouteTag),
                title: "Inventory preferences",
                subtitle: "Number format, company details, and Typst templates for stock movement PDFs. Leave a template blank to use the built-in challan.",
                form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                inputs,
                actions: html! {
                    (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                },
                ..Default::default()
            },
        )
    }
}

impl RenderAppPane for InventoryPreferencesPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            inventory_menu("preferences"),
            crate::crumbs::preferences_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(crate::crumbs::preferences_crumbs(), self.body())
    }
}

pub struct MovementSourceSelectPage {
    pub title: String,
    pub rows: ObjectList<(i64, String, String)>,
    pub target_input: String,
    pub path_and_query: String,
    pub page_size: u32,
}

fn source_picker_table<K, M, R>(page: &MovementSourceSelectPage, route: R, kind: &str) -> Markup
where
    K: SwapKey,
    M: SwapKey,
    R: RouteUrl + FkSelectGet<K, M> + Copy,
{
    let when_label = if kind == "process" {
        "Date & time"
    } else {
        "Vehicle in time"
    };
    let headers = [
        TableColumnHeader {
            key: "Number",
            label: "Number",
            sort_url: None,
            push_url: false,
        },
        TableColumnHeader {
            key: "Datetime",
            label: when_label,
            sort_url: None,
            push_url: false,
        },
    ];
    let rows: Vec<TableRow> = page
        .rows
        .items
        .iter()
        .map(|(id, label, when)| TableRow {
            attrs: row_attr_select_multi(&page.target_input, &id.to_string(), label),
            cells: vec![
                field_text(FieldText {
                    value: label,
                    classes: "",
                }),
                field_text(FieldText {
                    value: when,
                    classes: "",
                }),
            ],
            key: String::new(),
            children: Vec::new(),
        })
        .collect();
    let target_input = page.target_input.clone();
    let fields: &[FilterField] = &[];
    let filter_rows: &[FilterClauseView] = &[];
    let filter = table_button_filter(TableButtonFilter {
        panel: list_filter_builder(ListFilterBuilder {
            fields,
            rows: filter_rows,
            page_size: page.page_size,
            sort: "",
            attrs: form_hx_get_picker_route::<K, M, R>(route).set("hx-push-url", "false"),
            apply_label: "Apply",
            extra: html! {
                input type="hidden" name="target_input" value=(target_input) {}
            },
        }),
        ..Default::default()
    });
    let create = if can_mutate() {
        movement_picker_create_button(kind, &page.target_input)
    } else {
        html! {}
    };
    let actions = html! {
        (filter)
        (create)
    };
    data_table_with_filter_errors::<K>(
        &page.title,
        actions,
        &headers,
        &rows,
        render_picker_pagination::<M>(&page.path_and_query, page.rows.number, page.rows.num_pages),
        &page.path_and_query,
        &[],
        None,
    )
}

impl RenderPickerSelect<MovementInSelectTableKey, MovementInSelectModalKey>
    for MovementSourceSelectPage
{
    fn render_table(&self) -> Markup {
        source_picker_table::<
            MovementInSelectTableKey,
            MovementInSelectModalKey,
            MovementInFkSelectRouteTag,
        >(self, MovementInFkSelectRouteTag, "in")
    }
}

impl RenderPickerSelect<MovementProcessSelectTableKey, MovementProcessSelectModalKey>
    for MovementSourceSelectPage
{
    fn render_table(&self) -> Markup {
        source_picker_table::<
            MovementProcessSelectTableKey,
            MovementProcessSelectModalKey,
            MovementProcessFkSelectRouteTag,
        >(self, MovementProcessFkSelectRouteTag, "process")
    }
}

impl RenderTemplate for InventoryPreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Inventory preferences — Lariv",
            chrome,
            inventory_menu("preferences"),
            crate::crumbs::preferences_crumbs(),
            self.body(),
        )
    }
}
