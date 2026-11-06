use maud::{Markup, html};

use lariv_core::components::table::table_grid_content;
use lariv_core::components::{
    AppLayoutHtml, DataTable, DataTableDisplay, LayoutMain, LayoutSidebar, MainContentHtml,
    SwapKey, TableColumnHeader, TableListContent, TableRow, data_table, filter_error_banner,
    layout_main, layout_sidebar, table_button_columns, table_list_content,
};
pub use lariv_core::components::{render_pagination, render_picker_pagination};
use lariv_core::list_filters::FilterRowError;

use lariv_plugin_finance_common::finance_page_title;

use crate::accounting_sidebar;

pub fn app_scaffold(
    page: &str,
    chrome: &lariv_core::components::ShellChrome,
    crumbs: Markup,
    body: Markup,
    current_path: &str,
) -> Markup {
    app_scaffold_with_sidebar(
        page,
        chrome,
        accounting_sidebar::accounting_sidebar(current_path),
        crumbs,
        body,
    )
}

pub fn app_scaffold_with_sidebar(
    page: &str,
    chrome: &lariv_core::components::ShellChrome,
    sidebar: Markup,
    crumbs: Markup,
    body: Markup,
) -> Markup {
    lariv_core::components::shell_scaffold(lariv_core::components::ShellScaffold {
        title: &finance_page_title(page),
        registry_head: chrome.head.clone(),
        topbar_items: chrome.topbar_items.clone(),
        right_sidebar: chrome.right_sidebar.clone(),
        sidebar,
        breadcrumbs: crumbs,
        body,
        ..Default::default()
    })
}

pub fn layout_main_content(content: Markup) -> MainContentHtml {
    layout_main_with_crumbs(Markup::default(), content)
}

pub fn layout_main_with_crumbs(crumbs: Markup, content: Markup) -> MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content,
    })
}

pub fn layout_with_sidebar(current_path: &str, content: Markup) -> AppLayoutHtml {
    layout_with_sidebar_crumbs(current_path, Markup::default(), content)
}

pub fn layout_with_sidebar_crumbs(
    current_path: &str,
    crumbs: Markup,
    content: Markup,
) -> AppLayoutHtml {
    layout_with_entity_sidebar_crumbs(
        accounting_sidebar::accounting_sidebar(current_path),
        crumbs,
        content,
    )
}

pub fn layout_with_entity_sidebar(sidebar: Markup, content: Markup) -> AppLayoutHtml {
    layout_with_entity_sidebar_crumbs(sidebar, Markup::default(), content)
}

pub fn layout_with_entity_sidebar_crumbs(
    sidebar: Markup,
    crumbs: Markup,
    content: Markup,
) -> AppLayoutHtml {
    layout_sidebar(LayoutSidebar {
        sidebar,
        breadcrumbs: crumbs,
        content,
    })
}

pub fn data_table_with_filter_errors(
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
    data_table(DataTable {
        uid: key.id(),
        title,
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
        pagination,
        refresh_url,
        column_keys: &column_keys,
        ..Default::default()
    })
}
