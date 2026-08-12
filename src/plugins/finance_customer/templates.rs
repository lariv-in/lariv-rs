//! Finance accounting chrome for customer pages.

use maud::Markup;

use crate::{
    components::ShellChrome,
    template::{RenderAppPane, RenderTemplate},
};

use crate::plugins::finance_accounts::accounting_detail_menu::{
    DetailMenuNavItem, detail_sidebar_menu,
};
use crate::plugins::finance_accounts::templates::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs,
};
use crate::plugins::customer::routes::CustomerDetailRouteTag;
use crate::plugins::customer::templates::{
    CustomerDetailPage, CustomerListPage, customer_crumbs, customers_list_crumbs,
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

impl RenderAppPane for CustomerListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &self.path_and_query,
            customers_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(customers_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for CustomerListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Customers",
            chrome,
            customers_list_crumbs(),
            self.render_table(),
            &self.path_and_query,
        )
    }
}

impl CustomerDetailPage {
    pub(crate) fn finance_menu(&self) -> Markup {
        customer_detail_menu(self.id, &self.name)
    }
}

impl RenderAppPane for CustomerDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = customer_crumbs(self.id, &self.name, None);
        layout_with_entity_sidebar_crumbs(self.finance_menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(customer_crumbs(self.id, &self.name, None), self.body())
    }
}

impl RenderTemplate for CustomerDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = customer_crumbs(self.id, &self.name, None);
        app_scaffold_with_sidebar("Customer", chrome, self.finance_menu(), crumbs, self.body())
    }
}
