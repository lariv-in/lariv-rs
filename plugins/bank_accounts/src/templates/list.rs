use maud::{Markup, html};

use lariv_core::components::picker::RenderPickerSelect;
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

use crate::keys::{
    BANK_ACCOUNT_TABLE, BANK_SELECT_MODAL, BANK_SELECT_TABLE, MICR_SELECT_MODAL, MICR_SELECT_TABLE,
};
use crate::routes::{
    BankAccountDefaultRouteTag, BankAccountDetailRouteTag, BankAccountsMutate,
    BankFkSelectRouteTag, BankMicrSelectRouteTag,
};

use super::create::BANK_ACCOUNT_CREATE;
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

fn accounts_menu() -> Markup {
    sidebar_menu(SidebarMenu {
        title: "Bank Accounts",
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Bank Accounts",
                url: &BankAccountDefaultRouteTag.url(),
                active: true,
                ..Default::default()
            }))
        },
    })
}

fn accounts_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Bank Accounts",
        href: None,
    }])
}

#[derive(Clone)]
pub struct BankAccountRow {
    pub id: i64,
    pub name: String,
    pub account_number: String,
    pub ifsc_code: String,
    pub micr_code: String,
}

pub struct BankAccountListPage {
    pub accounts: ObjectList<BankAccountRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl BankAccountListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let account_sort = column_sort_url(&self.path_and_query, "Account Number", &self.sort);
        let ifsc_sort = column_sort_url(&self.path_and_query, "IFSC", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let account_label = format!(
            "Account Number{}",
            sort_indicator(&self.sort, "Account Number")
        );
        let ifsc_label = format!("IFSC{}", sort_indicator(&self.sort, "IFSC"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Account Number",
                label: &account_label,
                sort_url: Some(&account_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "IFSC",
                label: &ifsc_label,
                sort_url: Some(&ifsc_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .accounts
            .items
            .iter()
            .map(|a| TableRow {
                attrs: row_attr_navigate_route(BankAccountDetailRouteTag::new(a.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &a.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &a.account_number,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &a.ifsc_code,
                        classes: "",
                    }),
                ],
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
                    attrs: form_hx_get_route(BankAccountDefaultRouteTag),
                    apply_label: "Apply",
                    extra: html! {},
                }),
                ..Default::default()
            }))
        };
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<BankAccountsMutate>(),
        ) {
            actions = html! {
                (actions)
                (table_create_button(BANK_ACCOUNT_TABLE, BANK_ACCOUNT_CREATE,
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        filtered_data_table(
            BANK_ACCOUNT_TABLE,
            "Bank Accounts",
            actions,
            &headers,
            &rows,
            render_pagination(
                BANK_ACCOUNT_TABLE,
                &self.path_and_query,
                self.accounts.number,
                self.accounts.num_pages,
            ),
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl RenderAppPane for BankAccountListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(accounts_menu(), accounts_list_crumbs(), self.render_table())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(accounts_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for BankAccountListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Bank Accounts — Lariv",
            chrome,
            accounts_menu(),
            accounts_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Clone)]
pub struct BankRow {
    pub ifsc: String,
    pub bank_name: String,
    pub branch_name: String,
    pub city: String,
    pub display: String,
}

pub struct BankSelectPage {
    pub banks: ObjectList<BankRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl RenderPickerSelect for BankSelectPage {
    fn render_table(&self) -> Markup {
        let ifsc_sort = column_sort_url(&self.path_and_query, "IFSC", &self.sort);
        let bank_sort = column_sort_url(&self.path_and_query, "Bank", &self.sort);
        let branch_sort = column_sort_url(&self.path_and_query, "Branch", &self.sort);
        let ifsc_label = format!("IFSC{}", sort_indicator(&self.sort, "IFSC"));
        let bank_label = format!("Bank{}", sort_indicator(&self.sort, "Bank"));
        let branch_label = format!("Branch{}", sort_indicator(&self.sort, "Branch"));
        let headers = [
            TableColumnHeader {
                key: "IFSC",
                label: &ifsc_label,
                sort_url: Some(&ifsc_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Bank",
                label: &bank_label,
                sort_url: Some(&bank_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Branch",
                label: &branch_label,
                sort_url: Some(&branch_sort),
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .banks
            .items
            .iter()
            .map(|b| TableRow {
                attrs: row_attr_select(&self.target_input, &b.ifsc, &b.display),
                cells: vec![
                    field_text(FieldText {
                        value: &b.ifsc,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &b.bank_name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &b.branch_name,
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
                    attrs: form_hx_get_picker_route(BankFkSelectRouteTag)
                        .set("hx-push-url", "false"),
                    apply_label: "Apply",
                    extra: html! {
                        input type="hidden" name="target_input" value=(self.target_input) {}
                    },
                }),
                ..Default::default()
            }))
        };
        filtered_data_table(
            BANK_SELECT_TABLE,
            "Select bank / IFSC",
            actions,
            &headers,
            &rows,
            render_picker_pagination(
                BANK_SELECT_MODAL,
                &self.path_and_query,
                self.banks.number,
                self.banks.num_pages,
            ),
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl RenderTemplate for BankSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal(BANK_SELECT_MODAL).into_inner()
    }
}

pub struct MicrSelectPage {
    pub codes: ObjectList<String>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl RenderPickerSelect for MicrSelectPage {
    fn render_table(&self) -> Markup {
        let micr_sort = column_sort_url(&self.path_and_query, "MICR", &self.sort);
        let micr_label = format!("MICR{}", sort_indicator(&self.sort, "MICR"));
        let headers = [TableColumnHeader {
            key: "MICR",
            label: &micr_label,
            sort_url: Some(&micr_sort),
            push_url: false,
        }];
        let rows: Vec<TableRow> = self
            .codes
            .items
            .iter()
            .map(|code| TableRow {
                attrs: row_attr_select(&self.target_input, code, code),
                cells: vec![field_text(FieldText {
                    value: code,
                    classes: "",
                })],
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
                    attrs: form_hx_get_picker_route(BankMicrSelectRouteTag)
                        .set("hx-push-url", "false"),
                    apply_label: "Apply",
                    extra: html! {
                        input type="hidden" name="target_input" value=(self.target_input) {}
                    },
                }),
                ..Default::default()
            }))
        };
        filtered_data_table(
            MICR_SELECT_TABLE,
            "Select MICR",
            actions,
            &headers,
            &rows,
            render_picker_pagination(
                MICR_SELECT_MODAL,
                &self.path_and_query,
                self.codes.number,
                self.codes.num_pages,
            ),
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl RenderTemplate for MicrSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal(MICR_SELECT_MODAL).into_inner()
    }
}
