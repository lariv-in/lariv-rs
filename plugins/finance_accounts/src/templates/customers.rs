//! Accounting chrome around the customer plugin list table.

use maud::Markup;

use lariv_core::components::{Crumb, ShellChrome, breadcrumbs};
use lariv_core::template::{RenderAppPane, RenderTemplate};
use lariv_plugin_customer::templates::CustomerListPage;

use crate::routes::CustomerListRouteTag;

use super::common::{app_scaffold, layout_main_with_crumbs, layout_with_sidebar_crumbs};

fn customers_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Customers",
        href: None,
    }])
}

/// Finance hub page that renders [`CustomerListPage`]'s table under Accounting chrome.
pub struct CustomersListPage {
    pub inner: CustomerListPage,
}

impl CustomersListPage {
    pub fn render_table(&self) -> Markup {
        self.inner.render_table::<CustomerListRouteTag>()
    }

    fn body(&self) -> Markup {
        self.render_table()
    }
}

impl RenderAppPane for CustomersListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &self.inner.path_and_query,
            customers_list_crumbs(),
            self.body(),
        )
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(customers_list_crumbs(), self.body())
    }
}

impl RenderTemplate for CustomersListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Customers",
            chrome,
            customers_list_crumbs(),
            self.body(),
            &self.inner.path_and_query,
        )
    }
}
