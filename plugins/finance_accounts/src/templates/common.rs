use maud::Markup;

use lariv_core::components::{
    AppLayoutHtml, LayoutMain, LayoutSidebar, MainContentHtml, layout_main, layout_sidebar,
};

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

pub fn render_pagination<K: lariv_core::components::SwapKey>(
    path_and_query: &str,
    number: u32,
    num_pages: u32,
) -> Markup {
    use lariv_core::components::{PaginationPage, TablePagination, pagination_pages, table_pagination};

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

/// Pagination for FK picker modals — swaps the closest table (`outerHTML`).
pub fn render_picker_pagination<M: lariv_core::components::SwapKey>(
    path_and_query: &str,
    number: u32,
    num_pages: u32,
) -> Markup {
    use lariv_core::components::{
        PaginationPage, TablePagination, pagination_pages, table_pagination_picker,
    };

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
