use maud::{Markup, html};

use lariv_core::components::{
    Crumb, DataTable, DataTableDisplay, FieldPhone, FieldText, FilterClauseView, ListFilterBuilder,
    ObjectList, PaginationPage, ShellChrome, SidebarMenu, SidebarNavLink, SwapKey,
    TableButtonFilter, TableColumnHeader, TableListContent, TablePagination, TableRow, breadcrumbs,
    column_sort_url, data_table, field_phone, field_text, filter_error_banner, form_hx_get_route,
    list_filter_builder, row_attr_navigate_route, sidebar_menu, sidebar_nav_items_pane,
    sort_indicator, table_button_columns, table_button_filter, table_create_button,
    table_list_content, table_pagination,
};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::RenderTemplate;

use crate::create_modals::USER_CREATE;
use crate::keys::USER_TABLE;

use crate::routes::{UsersDetailRouteTag, UsersListRouteTag};

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

fn render_pagination(
    key: SwapKey,
    path_and_query: &str,
    number: u32,
    num_pages: u32,
    push_url: bool,
) -> Markup {
    let hx_target = key.selector();
    let owned =
        lariv_core::components::pagination_pages(path_and_query, number, num_pages, push_url);
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

fn users_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Users",
        href: None,
    }])
}

fn user_menu(current_path: &str) -> Markup {
    let users_url = UsersListRouteTag.url();
    let links = [SidebarNavLink {
        key: "users",
        title: "All Users",
        url: &users_url,
        icon_name: None,
        match_prefixes: &[],
    }];
    sidebar_menu(SidebarMenu {
        title: "Users",
        children: sidebar_nav_items_pane(&links, current_path),
    })
}

#[derive(Clone)]
pub struct UserRow {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub last_login_time: String,
}

pub struct UserListPage {
    pub users: ObjectList<UserRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
    pub show_last_login: bool,
}

impl UserListPage {
    /// Fine-grained table fragment for HTMX swaps targeting [`USER_TABLE`].
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let email_sort = column_sort_url(&self.path_and_query, "Email", &self.sort);
        let phone_sort = column_sort_url(&self.path_and_query, "Phone", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let email_label = format!("Email{}", sort_indicator(&self.sort, "Email"));
        let phone_label = format!("Phone{}", sort_indicator(&self.sort, "Phone"));
        let last_login_sort = column_sort_url(&self.path_and_query, "Last login", &self.sort);
        let last_login_label = format!("Last login{}", sort_indicator(&self.sort, "Last login"));
        let mut headers = vec![
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Email",
                label: &email_label,
                sort_url: Some(&email_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Phone",
                label: &phone_label,
                sort_url: Some(&phone_sort),
                push_url: true,
            },
        ];
        if self.show_last_login {
            headers.push(TableColumnHeader {
                key: "Last login",
                label: &last_login_label,
                sort_url: Some(&last_login_sort),
                push_url: true,
            });
        }
        let rows: Vec<TableRow> = self
            .users
            .items
            .iter()
            .map(|u| {
                let mut cells = vec![
                    field_text(FieldText {
                        value: &u.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &u.email,
                        classes: "",
                    }),
                    field_phone(FieldPhone {
                        value: &u.phone,
                        classes: "",
                    }),
                ];
                if self.show_last_login {
                    cells.push(field_text(FieldText {
                        value: &u.last_login_time,
                        classes: "",
                    }));
                }
                TableRow {
                    attrs: row_attr_navigate_route(UsersDetailRouteTag::new(u.id)),
                    cells,
                    key: String::new(),
                    children: Vec::new(),
                }
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route(UsersListRouteTag),
                    apply_label: "Apply Filters",
                    extra: html! {},
                }),
                ..Default::default()
            }))
            (table_create_button(USER_TABLE, USER_CREATE,
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };
        let pagination = render_pagination(
            USER_TABLE,
            &self.path_and_query,
            self.users.number,
            self.users.num_pages,
            true,
        );
        filtered_data_table(
            USER_TABLE,
            "",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl lariv_core::template::RenderAppPane for UserListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            user_menu(&self.path_and_query),
            users_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(users_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for UserListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Users — Lariv",
            chrome,
            user_menu(&self.path_and_query),
            users_list_crumbs(),
            self.render_table(),
        )
    }
}
