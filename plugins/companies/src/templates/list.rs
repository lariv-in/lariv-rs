use maud::{Markup, html};

use lariv_core::components::picker::{RenderPickerSelect, picker_create_button};
use lariv_core::components::{
    Crumb, DataTable, DataTableDisplay, FieldText, FilterClauseView, ListFilterBuilder, ObjectList,
    ShellChrome, SidebarMenu, SidebarMenuItem, SwapKey, TableButtonFilter, TableColumnHeader,
    TableListContent, TableRow, breadcrumbs, column_sort_url, data_table, field_text,
    filter_error_banner, form_hx_get_picker_route, form_hx_get_route, list_filter_builder,
    render_pagination, render_picker_pagination, row_attr_navigate_route, row_attr_select,
    sidebar_menu, sidebar_menu_item_pane, sort_indicator, table_button_columns,
    table_button_filter, table_create_button, table_list_content,
};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use crate::keys::{COMPANY_SELECT_MODAL, COMPANY_SELECT_TABLE, COMPANY_TABLE};
use crate::routes::{
    CompaniesMutate, CompanyDefaultRouteTag, CompanyDetailRouteTag, CompanyFkSelectRouteTag,
};

use super::create::COMPANY_CREATE;
use super::{app_scaffold, scaffold_main, scaffold_pane};

pub(crate) fn filtered_data_table(
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

fn companies_menu() -> Markup {
    sidebar_menu(SidebarMenu {
        title: "Companies",
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Companies",
                url: &CompanyDefaultRouteTag.url(),
                active: true,
                ..Default::default()
            }))
        },
    })
}

fn companies_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Companies",
        href: None,
    }])
}

#[derive(Clone)]
pub struct CompanyRow {
    pub id: i64,
    pub name: String,
    pub website: String,
}

pub struct CompanyListPage {
    pub companies: ObjectList<CompanyRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl CompanyListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [TableColumnHeader {
            key: "Name",
            label: &name_label,
            sort_url: Some(&name_sort),
            push_url: true,
        }];
        let rows: Vec<TableRow> = self
            .companies
            .items
            .iter()
            .map(|a| TableRow {
                attrs: row_attr_navigate_route(CompanyDetailRouteTag::new(a.id)),
                cells: vec![field_text(FieldText {
                    value: &a.name,
                    classes: "",
                })],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route(
                        CompanyDefaultRouteTag,
                    ),
                    apply_label: "Apply",
                    extra: html! {},
                }),
                ..Default::default()
            }))
        };
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<CompaniesMutate>(),
        ) {
            actions = html! {
                (actions)
                (table_create_button(COMPANY_TABLE, COMPANY_CREATE,
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        filtered_data_table(
            COMPANY_TABLE,
            "Companies",
            actions,
            &headers,
            &rows,
            render_pagination(
                COMPANY_TABLE,
                &self.path_and_query,
                self.companies.number,
                self.companies.num_pages,
            ),
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl RenderAppPane for CompanyListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            companies_menu(),
            companies_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(companies_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for CompanyListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Companies — Lariv",
            chrome,
            companies_menu(),
            companies_list_crumbs(),
            self.render_table(),
        )
    }
}

pub struct CompanySelectPage {
    pub companies: ObjectList<CompanyRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl RenderPickerSelect for CompanySelectPage {
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
            .companies
            .items
            .iter()
            .map(|a| TableRow {
                attrs: row_attr_select(&self.target_input, &a.id.to_string(), &a.name),
                cells: vec![field_text(FieldText {
                    value: &a.name,
                    classes: "",
                })],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_picker_route(CompanyFkSelectRouteTag)
                    .set("hx-push-url", "false"),
                    apply_label: "Apply",
                    extra: html! {
                        input type="hidden" name="target_input" value=(self.target_input) {}
                    },
                }),
                ..Default::default()
            }))
        };
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<CompaniesMutate>(),
        ) {
            actions = html! {
                (actions)
                (picker_create_button(COMPANY_CREATE,
                    &self.target_input,
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        filtered_data_table(
            COMPANY_SELECT_TABLE,
            "Select company",
            actions,
            &headers,
            &rows,
            render_picker_pagination(
                COMPANY_SELECT_MODAL,
                &self.path_and_query,
                self.companies.number,
                self.companies.num_pages,
            ),
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl RenderTemplate for CompanySelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal(COMPANY_SELECT_MODAL).into_inner()
    }
}
