//! Maud pages for stocks, movements, and movement lines.

use frunk::Generic;
use lariv_formula::VariableType;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DeleteConfirmation, DetailHeader, FieldText, FormOpts,
    LayoutMain, LayoutSidebar, ObjectList,
    PaginationPage, ShellChrome, ShellScaffold, SidebarMenu, SidebarMenuItem, SlotCapability,
    SlotRegistrar, SwapKey, TableButtonFilter, TableColumnHeader, TablePagination, TableRow,
    button_modal_form, button_modal_route, button_submit, column_sort_url, container_column,
    data_table_list_refresh, data_table_rows, delete_confirmation, detail, detail_header,
    field_text, form, form_hx_get_picker_route, form_hx_get_route, form_hx_post_main,
    form_hx_post_selector, form_hx_post_url, label, layout_main, layout_sidebar, modal,
    modal_keyed, pagination_pages, row_attr_navigate, row_attr_select_extra, shell_scaffold,
    sidebar_menu, sidebar_menu_item_pane, sort_indicator, table_button_filter, table_create_button,
    table_pagination, table_pagination_picker, with_list_filter_common,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::picker::{RenderPickerSelect, picker_create_button};
use lariv_core::template::{
    RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar,
};
use lariv_core::web::{modal_create_post_url, modal_edit_post_url};

use super::movement_lines::{MovementLineDisplay, field_movement_lines};

use super::crumbs::{movement_crumbs, movements_list_crumbs, stock_crumbs, stocks_list_crumbs};
use super::detail_menu::{movement_detail_menu, stock_detail_menu};
use super::forms::{
    InventoryPreferencesForm, InventoryPreferencesFormField, MovementFilterForm,
    MovementFilterFormField, MovementForm, MovementFormField, StockFilterForm,
    StockFilterFormField, StockForm, StockFormField,
};
use super::keys::{
    MovementCreateModalKey, MovementDeleteModalKey, MovementEditModalKey, MovementTableKey,
    StockCreateModalKey, StockDeleteModalKey, StockEditModalKey, StockLinesTableKey,
    StockSelectModalKey, StockSelectTableKey, StockTableKey,
};
use super::movement_type::MovementType;
use super::routes::{
    InventoryMutate, MovementCreatePostRouteTag, MovementDefaultRouteTag,
    MovementDeleteGetRouteTag, MovementDeletePostRouteTag, MovementEditGetRouteTag,
    MovementEditPostRouteTag, StockCreatePostRouteTag, StockDefaultRouteTag,
    StockDeleteGetRouteTag, StockDeletePostRouteTag, StockEditGetRouteTag, StockEditPostRouteTag,
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

fn choice_pairs(choices: &[(&str, &str)]) -> Vec<(String, String)> {
    choices
        .iter()
        .map(|(key, label)| ((*key).to_string(), (*label).to_string()))
        .collect()
}

fn qty_type_choices() -> Vec<(String, String)> {
    VariableType::all()
        .iter()
        .map(|ty| (ty.as_str().to_string(), ty.label().to_string()))
        .collect()
}

fn movement_type_choices() -> Vec<(String, String)> {
    choice_pairs(MovementType::choices())
}

fn movement_type_filter_choices() -> Vec<(String, String)> {
    let mut choices = vec![("".to_string(), "Any".to_string())];
    choices.extend(movement_type_choices());
    choices
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
    number: &'a str,
    datetime: &'a str,
    movement_type: &'a str,
    bill_to_individual: bool,
    customer_individual: &'a str,
    customer_individual_display: &'a str,
    customer_company: &'a str,
    customer_company_display: &'a str,
    vehicle_type: &'a str,
    vehicle_number: &'a str,
    eway_bill: &'a str,
    driver_id: &'a str,
    driver_display: &'a str,
    lines_json: &'a str,
}

fn movement_form_inputs(values: MovementFormValues<'_>) -> Markup {
    let types = movement_type_choices();
    let bill = if values.bill_to_individual {
        "true"
    } else {
        "false"
    };
    let x_data = format!("{{ billToIndividual: {bill} }}");
    MovementForm::render_inputs(
        &FormCtx::form::<MovementForm>(CsrfToken::current())
            .x_data(&x_data)
            .value(MovementFormField::Number, values.number)
            .value(MovementFormField::Datetime, values.datetime)
            .value(MovementFormField::MovementType, values.movement_type)
            .choices(MovementFormField::MovementType, &types)
            .checked(
                MovementFormField::BillToIndividual,
                values.bill_to_individual,
            )
            .value(
                MovementFormField::BillToIndividual,
                if values.bill_to_individual { "on" } else { "" },
            )
            .value(
                MovementFormField::CustomerIndividual,
                values.customer_individual,
            )
            .display(
                MovementFormField::CustomerIndividual,
                values.customer_individual_display,
            )
            .value(MovementFormField::CustomerCompany, values.customer_company)
            .display(
                MovementFormField::CustomerCompany,
                values.customer_company_display,
            )
            .value(MovementFormField::VehicleType, values.vehicle_type)
            .value(MovementFormField::VehicleNumber, values.vehicle_number)
            .value(MovementFormField::EwayBill, values.eway_bill)
            .value(MovementFormField::DriverId, values.driver_id)
            .display(MovementFormField::DriverId, values.driver_display)
            .value(MovementFormField::LinesJson, values.lines_json),
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
    pub filter_name: String,
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
            })
            .collect();
        let pagination = render_pagination::<StockTableKey>(
            &self.path_and_query,
            self.stocks.number,
            self.stocks.num_pages,
        );
        if let Some(instance_id) = rows_instance {
            return data_table_rows::<StockTableKey>(&headers, &rows, pagination, instance_id);
        }
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_route::<StockTableKey, StockDefaultRouteTag>(StockDefaultRouteTag),
                    inputs: with_list_filter_common(
                        StockFilterForm::render_inputs(
                            &FormCtx::form::<StockFilterForm>(CsrfToken::current())
                                .value(StockFilterFormField::Name, &self.filter_name),
                        ),
                        self.page_size,
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                    },
                    ..Default::default()
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
        data_table_list_refresh::<StockTableKey>(
            "Stocks",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
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
    pub filter_name: String,
    pub target_input: String,
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
                }
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_picker_route::<StockSelectTableKey, StockSelectModalKey, StockFkSelectRouteTag>(
                        StockFkSelectRouteTag,
                    ).set("hx-push-url", "false"),
                    inputs: html! {
                        (with_list_filter_common(
                            StockFilterForm::render_inputs(
                                &FormCtx::form::<StockFilterForm>(CsrfToken::current())
                                    .value(StockFilterFormField::Name, &self.filter_name),
                            ),
                            self.page_size,
                        ))
                        input type="hidden" name="target_input" value=(self.target_input) {}
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
        };
        if can_mutate() {
            actions = html! {
                (actions)
                (picker_create_button::<StockCreateModalKey>(&self.target_input, Some("plus"), "btn-square btn-outline btn-sm"))
            };
        }
        data_table_list_refresh::<StockSelectTableKey>(
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
    pub movement_type: String,
    pub vehicle: String,
    pub driver: String,
    pub detail_href: String,
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
    pub movements: ObjectList<MovementRow>,
    pub filter_type: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl MovementListPage {
    pub fn render_table(&self) -> Markup {
        self.render_table_inner(None)
    }

    pub fn render_table_rows(&self, instance_id: &str) -> Markup {
        self.render_table_inner(Some(instance_id))
    }

    fn render_table_inner(&self, rows_instance: Option<&str>) -> Markup {
        let number_sort = column_sort_url(&self.path_and_query, "Number", &self.sort);
        let when_sort = column_sort_url(&self.path_and_query, "Datetime", &self.sort);
        let type_sort = column_sort_url(&self.path_and_query, "Type", &self.sort);
        let number_label = format!("Number{}", sort_indicator(&self.sort, "Number"));
        let when_label = format!("Date & time{}", sort_indicator(&self.sort, "Datetime"));
        let type_label = format!("Type{}", sort_indicator(&self.sort, "Type"));
        let headers = [
            TableColumnHeader {
                key: "Number",
                label: &number_label,
                sort_url: Some(&number_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Customer",
                label: "Customer",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Datetime",
                label: &when_label,
                sort_url: Some(&when_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Type",
                label: &type_label,
                sort_url: Some(&type_sort),
                push_url: true,
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
        ];
        let rows: Vec<TableRow> = self
            .movements
            .items
            .iter()
            .map(|row| TableRow {
                attrs: row_attr_navigate(&row.detail_href),
                cells: vec![
                    field_text(FieldText {
                        value: &row.number,
                        classes: "",
                    }),
                    party_link(&row.customer, &row.customer_href),
                    field_text(FieldText {
                        value: &row.datetime,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.movement_type,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.vehicle,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.driver,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let pagination = render_pagination::<MovementTableKey>(
            &self.path_and_query,
            self.movements.number,
            self.movements.num_pages,
        );
        if let Some(instance_id) = rows_instance {
            return data_table_rows::<MovementTableKey>(&headers, &rows, pagination, instance_id);
        }
        let types = movement_type_filter_choices();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_route::<MovementTableKey, MovementDefaultRouteTag>(MovementDefaultRouteTag),
                    inputs: with_list_filter_common(
                        MovementFilterForm::render_inputs(
                            &FormCtx::form::<MovementFilterForm>(CsrfToken::current())
                                .value(MovementFilterFormField::MovementType, &self.filter_type)
                                .choices(MovementFilterFormField::MovementType, &types),
                        ),
                        self.page_size,
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
        };
        if can_mutate() {
            actions = html! {
                (actions)
                (table_create_button::<MovementTableKey, MovementCreateModalKey>(
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        data_table_list_refresh::<MovementTableKey>(
            "Movements",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for MovementListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            inventory_menu("movements"),
            movements_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(movements_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for MovementListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Movements — Lariv",
            chrome,
            inventory_menu("movements"),
            movements_list_crumbs(),
            self.render_table(),
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
    pub number: String,
    pub datetime: String,
    pub movement_type: String,
    pub customer: String,
    pub customer_href: String,
    pub vehicle: String,
    pub eway_bill: String,
    pub driver: String,
    pub driver_href: String,
    pub lines: Vec<MovementLineRow>,
}

impl MovementDetailPage {
    fn lines_table(&self) -> Markup {
        let rows: Vec<MovementLineDisplay<'_>> = self
            .lines
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
        let pdf = button_modal_route(
            crate::routes::MovementPdfModalRouteTag::new(self.id),
            "PDF",
            "btn-outline",
        );
        if can_mutate() {
            html! {
                (pdf)
                (button_modal_form(ButtonModalForm {
                    name: "p_inventory.MovementEditForm",
                    href: &MovementEditGetRouteTag::new(self.id).url(),
                    form_post_url: &MovementEditPostRouteTag::new(self.id).path(),
                    modal_uid: MovementEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        } else {
            html! { (pdf) }
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
                    (label("Date & time", field_text(FieldText { value: &self.datetime, classes: "" })))
                    (label("Type", field_text(FieldText { value: &self.movement_type, classes: "" })))
                    (label("Customer", party_link(&self.customer, &self.customer_href)))
                    (label("Vehicle", field_text(FieldText { value: &self.vehicle, classes: "" })))
                    (label("E-way bill", field_text(FieldText { value: &self.eway_bill, classes: "" })))
                    (label("Driver", party_link(&self.driver, &self.driver_href)))
                    div class="mt-4" { (self.lines_table()) }
                }))
            }))
        }
    }
}

impl RenderAppPane for MovementDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            movement_detail_menu(&self.number, self.id),
            movement_crumbs(&self.number, self.id),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(movement_crumbs(&self.number, self.id), self.body())
    }
}

impl RenderTemplate for MovementDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Movement — Lariv",
            chrome,
            movement_detail_menu(&self.number, self.id),
            movement_crumbs(&self.number, self.id),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct MovementCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub number: String,
    pub datetime: String,
    pub movement_type: String,
    pub bill_to_individual: bool,
    pub customer_individual: String,
    pub customer_individual_display: String,
    pub customer_company: String,
    pub customer_company_display: String,
    pub vehicle_type: String,
    pub vehicle_number: String,
    pub eway_bill: String,
    pub driver_id: String,
    pub driver_display: String,
    pub lines_json: String,
    pub error: String,
}

fn movement_values_from_create(page: &MovementCreateModalPage) -> MovementFormValues<'_> {
    MovementFormValues {
        number: &page.number,
        datetime: &page.datetime,
        movement_type: &page.movement_type,
        bill_to_individual: page.bill_to_individual,
        customer_individual: &page.customer_individual,
        customer_individual_display: &page.customer_individual_display,
        customer_company: &page.customer_company,
        customer_company_display: &page.customer_company_display,
        vehicle_type: &page.vehicle_type,
        vehicle_number: &page.vehicle_number,
        eway_bill: &page.eway_bill,
        driver_id: &page.driver_id,
        driver_display: &page.driver_display,
        lines_json: &page.lines_json,
    }
}

impl RenderTemplate for MovementCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<MovementCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New movement" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<MovementCreateModalKey>(&modal_create_post_url(
                        MovementCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                    inputs: movement_form_inputs(movement_values_from_create(self)),
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
    pub number: String,
    pub datetime: String,
    pub movement_type: String,
    pub bill_to_individual: bool,
    pub customer_individual: String,
    pub customer_individual_display: String,
    pub customer_company: String,
    pub customer_company_display: String,
    pub vehicle_type: String,
    pub vehicle_number: String,
    pub eway_bill: String,
    pub driver_id: String,
    pub driver_display: String,
    pub lines_json: String,
    pub error: String,
}

fn movement_values_from_edit(page: &MovementEditModalPage) -> MovementFormValues<'_> {
    MovementFormValues {
        number: &page.number,
        datetime: &page.datetime,
        movement_type: &page.movement_type,
        bill_to_individual: page.bill_to_individual,
        customer_individual: &page.customer_individual,
        customer_individual_display: &page.customer_individual_display,
        customer_company: &page.customer_company,
        customer_company_display: &page.customer_company_display,
        vehicle_type: &page.vehicle_type,
        vehicle_number: &page.vehicle_number,
        eway_bill: &page.eway_bill,
        driver_id: &page.driver_id,
        driver_display: &page.driver_display,
        lines_json: &page.lines_json,
    }
}

impl RenderTemplate for MovementEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = MovementDeleteGetRouteTag::new(self.id).url();
        modal_keyed::<MovementEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit movement" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<MovementEditModalKey>(&modal_edit_post_url(
                        MovementEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                    inputs: movement_form_inputs(movement_values_from_edit(self)),
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
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", self.modal_uid);
        let post_url = if self.modal_uid == StockDeleteModalKey::ID {
            StockDeletePostRouteTag::new(self.id).url()
        } else {
            MovementDeletePostRouteTag::new(self.id).url()
        };
        modal(lariv_core::components::Modal {
            uid: self.modal_uid.as_str(),
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: &self.message,
                attrs: form_hx_post_selector(&post_url, &target),
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
