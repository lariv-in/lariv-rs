use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    Crumb, DataTable, DataTableDisplay, FieldText, FilterClauseView, ListFilterBuilder, ObjectList,
    ShellChrome, SidebarMenu, SidebarNavLink, SwapKey, TableButtonFilter, TableColumnHeader,
    TableListContent, TableRow, breadcrumbs, column_sort_url, data_table, field_text,
    filter_error_banner, form_hx_get_route, list_filter_builder, render_pagination_push,
    row_attr_navigate_route, sidebar_menu, sidebar_nav_items_pane, sort_indicator,
    table_button_columns, table_button_filter, table_create_button, table_list_content,
};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::RenderTemplate;

use crate::keys::TEST_CASE_TABLE;
use crate::routes::{TestCaseDetailRouteTag, TestCaseListRouteTag};

use super::create::TEST_CASE_CREATE;
use super::{app_scaffold, scaffold_main, scaffold_pane};

fn data_table_filtered(
    key: SwapKey,
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
        .map(|h| h.key)
        .filter(|k| !k.is_empty())
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
        title: "",
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

pub(crate) fn list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Assistant Tester",
        href: None,
    }])
}

pub(crate) fn list_menu(current_path: &str) -> Markup {
    let list_url = TestCaseListRouteTag.url();
    let links = [SidebarNavLink {
        key: "cases",
        title: "Test Cases",
        url: &list_url,
        icon_name: None,
        match_prefixes: &[],
    }];
    sidebar_menu(SidebarMenu {
        title: "Assistant Tester",
        children: sidebar_nav_items_pane(&links, current_path),
    })
}

/// Row payload for [`TestCaseListPage`].
#[derive(Clone)]
pub struct TestCaseRow {
    pub id: i64,
    pub name: String,
    pub author_name: String,
    pub updated_at: String,
}

#[derive(Generic)]
pub struct TestCaseListPage {
    pub cases: ObjectList<TestCaseRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl TestCaseListPage {
    /// Fine-grained table fragment for HTMX swaps targeting [`TEST_CASE_TABLE`].
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let updated_sort = column_sort_url(&self.path_and_query, "UpdatedAt", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let updated_label = format!("Updated At{}", sort_indicator(&self.sort, "UpdatedAt"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Author",
                label: "Author",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "UpdatedAt",
                label: &updated_label,
                sort_url: Some(&updated_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .cases
            .items
            .iter()
            .map(|case| TableRow {
                attrs: row_attr_navigate_route(TestCaseDetailRouteTag::new(case.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &case.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &case.author_name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &case.updated_at,
                        classes: "",
                    }),
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route(TestCaseListRouteTag),
                    apply_label: "Apply Filters",
                    extra: html! {},
                }),
                ..Default::default()
            }))
            (table_create_button(
                TEST_CASE_TABLE,
                TEST_CASE_CREATE,
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };
        let pagination = render_pagination_push(
            TEST_CASE_TABLE,
            &self.path_and_query,
            self.cases.number,
            self.cases.num_pages,
            true,
        );
        data_table_filtered(
            TEST_CASE_TABLE,
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl lariv_core::template::RenderAppPane for TestCaseListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            list_menu(&self.path_and_query),
            list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for TestCaseListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            chrome,
            list_menu(&self.path_and_query),
            list_crumbs(),
            self.render_table(),
        )
    }
}
