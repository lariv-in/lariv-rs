//! Read-only audit log list.

use maud::{Markup, html};

use lariv_core::capability::define_register_items;
use lariv_core::components::{
    Crumb, DataTable, DataTableDisplay, FieldText, FilterClauseView, JsonDiff, LayoutMain,
    LayoutSidebar, ListFilterBuilder, ObjectList, ShellChrome, ShellScaffold, SidebarMenu,
    SidebarNavLink, TableButtonFilter, TableColumnHeader, TableListContent, TableRow, breadcrumbs,
    column_sort_url, data_table, field_text, filter_error_banner, form_hx_get_url, json_diff,
    layout_main, layout_sidebar, list_filter_builder, render_pagination_push, shell_scaffold,
    sidebar_menu, sidebar_nav_items_pane, sort_indicator, table_button_columns,
    table_button_filter, table_list_content,
};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar};

use super::description::log_description_sides;
use super::keys::LOG_TABLE;
use super::routes::LogListRouteTag;

define_register_items! {
    plugin: LoggingTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        LogListIdx: LogListPageTag => LogListPage,
    ]
}

fn app_scaffold(chrome: &ShellChrome, sidebar: Markup, crumbs: Markup, body: Markup) -> Markup {
    shell_scaffold(ShellScaffold {
        title: "Lariv",
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

fn list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Logs",
        href: None,
    }])
}

fn list_menu(current_path: &str) -> Markup {
    let list_url = LogListRouteTag.url();
    let links = [SidebarNavLink {
        key: "logs",
        title: "Logs",
        url: &list_url,
        icon_name: None,
        match_prefixes: &[],
    }];
    sidebar_menu(SidebarMenu {
        title: "Logs",
        children: sidebar_nav_items_pane(&links, current_path),
    })
}

/// One row of the audit log table.
#[derive(Clone)]
pub struct LogRow {
    pub created_at: String,
    pub table_name: String,
    pub row_id: String,
    pub log_kind: String,
    pub user: String,
    pub description: serde_json::Value,
}

/// Read-only audit log page.
pub struct LogListPage {
    pub logs: ObjectList<LogRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
    /// GET target for the filter form. The global list uses [`LogListRouteTag`].
    pub filter_form_url: String,
}

impl LogListPage {
    /// Table fragment for HTMX swaps targeting [`LOG_TABLE`].
    pub fn render_table(&self) -> Markup {
        let created_sort = column_sort_url(&self.path_and_query, "CreatedAt", &self.sort);
        let table_sort = column_sort_url(&self.path_and_query, "TableName", &self.sort);
        let kind_sort = column_sort_url(&self.path_and_query, "LogKind", &self.sort);
        let created_label = format!("When{}", sort_indicator(&self.sort, "CreatedAt"));
        let table_label = format!("Table{}", sort_indicator(&self.sort, "TableName"));
        let kind_label = format!("Kind{}", sort_indicator(&self.sort, "LogKind"));
        let headers = [
            TableColumnHeader {
                key: "CreatedAt",
                label: &created_label,
                sort_url: Some(&created_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "TableName",
                label: &table_label,
                sort_url: Some(&table_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "RowId",
                label: "Row",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "LogKind",
                label: &kind_label,
                sort_url: Some(&kind_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "User",
                label: "User",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Description",
                label: "Description",
                sort_url: None,
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .logs
            .items
            .iter()
            .map(|row| TableRow {
                attrs: Default::default(),
                cells: vec![
                    field_text(FieldText {
                        value: &row.created_at,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.table_name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.row_id,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.log_kind,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &row.user,
                        classes: "",
                    }),
                    {
                        let (left, right) = log_description_sides(&row.log_kind, &row.description);
                        json_diff(JsonDiff {
                            left: &left,
                            right: &right,
                        })
                    },
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let column_keys: Vec<&str> = headers.iter().map(|header| header.key).collect();
        let hx_target = LOG_TABLE.selector();
        let list = html! {
            (filter_error_banner(&self.filter_errors))
            (table_list_content(TableListContent {
                headers: &headers,
                rows: &rows,
                hx_target: &hx_target,
            }))
        };
        let grid = html! {
            (filter_error_banner(&self.filter_errors))
            (lariv_core::components::table::table_grid_content(&headers, &rows))
        };
        let actions = html! {
            (table_button_columns(&headers))
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_url(LOG_TABLE, &self.filter_form_url),
                    apply_label: "Apply Filters",
                    extra: html! {},
                }),
                ..Default::default()
            }))
        };
        let pagination = render_pagination_push(
            LOG_TABLE,
            &self.path_and_query,
            self.logs.number,
            self.logs.num_pages,
            true,
        );
        data_table(DataTable {
            uid: LOG_TABLE.id(),
            title: "",
            subtitle: "",
            classes: "w-full",
            actions,
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
            refresh_url: &self.path_and_query,
            column_keys: &column_keys,
            instance_id: "",
            body_only: false,
        })
    }
}

impl lariv_core::template::RenderAppPane for LogListPage {
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

impl RenderTemplate for LogListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            chrome,
            list_menu(&self.path_and_query),
            list_crumbs(),
            self.render_table(),
        )
    }
}
