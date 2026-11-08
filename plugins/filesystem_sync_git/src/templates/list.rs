use maud::{Markup, html};

use lariv_core::components::{
    Crumb, DataTable, DataTableDisplay, FieldText, FilterClauseView, ListFilterBuilder, ObjectList,
    ShellChrome, TableButtonFilter, TableColumnHeader, TableListContent, TableRow, breadcrumbs,
    column_sort_url, data_table, field_text, filter_error_banner, form_hx_get_route,
    list_filter_builder, render_pagination, row_attr_navigate_route, sort_indicator,
    table_button_columns, table_button_filter, table_create_button, table_list_content,
};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use lariv_plugin_filesystem::routes::VNodeListRouteTag;
use lariv_plugin_filesystem::templates::main_menu;

use crate::keys::GIT_SYNC_TABLE;
use crate::routes::{FilesystemSyncGitAdmin, GitSyncDetailRouteTag, GitSyncListRouteTag};

use super::modals::GIT_SYNC_CREATE;
use super::{app_scaffold, scaffold_main, scaffold_pane};

#[derive(Clone)]
pub struct GitSyncRow {
    pub id: i64,
    pub git_url: String,
    pub git_identifier: String,
    pub last_synced: String,
    pub subpath: String,
    pub directory: String,
}

pub struct GitSyncListPage {
    pub rows: ObjectList<GitSyncRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

fn menu() -> Markup {
    main_menu(&GitSyncListRouteTag.url())
}

fn crumbs() -> Markup {
    let list_url = VNodeListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Filesystem",
            href: Some(&list_url),
        },
        Crumb {
            label: "Git Sync",
            href: None,
        },
    ])
}

impl GitSyncListPage {
    pub fn render_table(&self) -> Markup {
        let url_sort = column_sort_url(&self.path_and_query, "URL", &self.sort);
        let identifier_sort = column_sort_url(&self.path_and_query, "Identifier", &self.sort);
        let url_label = format!("URL{}", sort_indicator(&self.sort, "URL"));
        let identifier_label =
            format!("Git identifier{}", sort_indicator(&self.sort, "Identifier"));
        let headers = [
            TableColumnHeader {
                key: "URL",
                label: &url_label,
                sort_url: Some(&url_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Identifier",
                label: &identifier_label,
                sort_url: Some(&identifier_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Last synced",
                label: "Last synced",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Subpath",
                label: "Subpath",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Directory",
                label: "Directory",
                sort_url: None,
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .rows
            .items
            .iter()
            .map(|row| {
                let subpath = if row.subpath.is_empty() {
                    "(repository root)"
                } else {
                    row.subpath.as_str()
                };
                TableRow {
                    attrs: row_attr_navigate_route(GitSyncDetailRouteTag::new(row.id)),
                    cells: vec![
                        field_text(FieldText {
                            value: &row.git_url,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &row.git_identifier,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &row.last_synced,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: subpath,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &row.directory,
                            classes: "",
                        }),
                    ],
                    key: String::new(),
                    children: Vec::new(),
                }
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route(GitSyncListRouteTag),
                    apply_label: "Apply",
                    extra: html! {},
                }),
                ..Default::default()
            }))
        };
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<FilesystemSyncGitAdmin>(),
        ) {
            actions = html! {
                (actions)
                (table_create_button(
                    GIT_SYNC_TABLE,
                    GIT_SYNC_CREATE,
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        let hx_target = GIT_SYNC_TABLE.selector();
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
        let column_keys: Vec<&str> = headers
            .iter()
            .map(|header| header.key)
            .filter(|key| !key.is_empty())
            .collect();
        let toolbar = html! {
            (table_button_columns(&headers))
            (actions)
        };
        data_table(DataTable {
            uid: GIT_SYNC_TABLE.id(),
            title: "Git Sync",
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
            pagination: render_pagination(
                GIT_SYNC_TABLE,
                &self.path_and_query,
                self.rows.number,
                self.rows.num_pages,
            ),
            oob: false,
            refresh_url: &self.path_and_query,
            column_keys: &column_keys,
            instance_id: "",
            body_only: false,
        })
    }
}

impl RenderAppPane for GitSyncListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(menu(), crumbs(), self.render_table())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(crumbs(), self.render_table())
    }
}

impl RenderTemplate for GitSyncListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Git Sync — Lariv",
            chrome,
            menu(),
            crumbs(),
            self.render_table(),
        )
    }
}
