//! Finance accounting chrome for customer pages.

use maud::Markup;

use lariv_core::components::ShellChrome;

use lariv_plugin_customer::routes::CustomerDetailRouteTag;
use lariv_plugin_customer::templates::{
    CustomerDetailPage, CustomerListPage, customer_crumbs, customers_list_crumbs,
};
use lariv_plugin_finance_accounts::accounting_detail_menu::{
    DetailMenuNavItem, detail_sidebar_menu,
};
use lariv_plugin_finance_accounts::templates::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs,
};

fn customer_detail_menu(id: i64, name: &str) -> Markup {
    let menu_title = format!("Customer: {name}");
    let detail_url = CustomerDetailRouteTag::new(id).url();
    let nav = vec![DetailMenuNavItem {
        title: "Customer Detail",
        url: detail_url,
        active: true,
    }];
    detail_sidebar_menu(menu_title, &nav, None, maud::html! {})
}

pub fn register() {
    lariv_plugin_customer::templates::register_finance_chrome(
        list_pane,
        list_main,
        list_page,
        detail_pane,
        detail_main,
        detail_page,
    );
}

fn list_pane(page: &CustomerListPage) -> lariv_core::components::AppLayoutHtml {
    layout_with_sidebar_crumbs(
        &page.path_and_query,
        customers_list_crumbs(),
        page.render_table(),
    )
}

fn list_main(page: &CustomerListPage) -> lariv_core::components::MainContentHtml {
    layout_main_with_crumbs(customers_list_crumbs(), page.render_table())
}

fn list_page(page: &CustomerListPage, chrome: &ShellChrome) -> Markup {
    app_scaffold(
        "Customers",
        chrome,
        customers_list_crumbs(),
        page.render_table(),
        &page.path_and_query,
    )
}

fn detail_pane(page: &CustomerDetailPage) -> lariv_core::components::AppLayoutHtml {
    let crumbs = customer_crumbs(page.id, &page.name, None);
    layout_with_entity_sidebar_crumbs(customer_detail_menu(page.id, &page.name), crumbs, page.body())
}

fn detail_main(page: &CustomerDetailPage) -> lariv_core::components::MainContentHtml {
    layout_main_with_crumbs(customer_crumbs(page.id, &page.name, None), page.body())
}

fn detail_page(page: &CustomerDetailPage, chrome: &ShellChrome) -> Markup {
    let crumbs = customer_crumbs(page.id, &page.name, None);
    app_scaffold_with_sidebar(
        "Customer",
        chrome,
        customer_detail_menu(page.id, &page.name),
        crumbs,
        page.body(),
    )
}
