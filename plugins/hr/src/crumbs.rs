//! Breadcrumb trails for HR pages.

use maud::Markup;

use lariv_core::components::{Crumb, breadcrumbs};

use super::routes::PeopleHubRouteTag;

fn hub_tab_url(tab: &str) -> String {
    lariv_core::http::RouteQueryBuilder::new(PeopleHubRouteTag)
        .query("tab", tab)
        .build()
}

pub fn hub_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "People",
        href: None,
    }])
}

pub fn preferences_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Preferences",
        href: None,
    }])
}

pub fn employee_crumbs(name: &str) -> Markup {
    breadcrumbs(&[
        Crumb {
            label: "People",
            href: Some(&hub_tab_url("employees")),
        },
        Crumb {
            label: name,
            href: None,
        },
    ])
}

pub fn ex_employee_crumbs(name: &str) -> Markup {
    breadcrumbs(&[
        Crumb {
            label: "People",
            href: Some(&hub_tab_url("ex_employees")),
        },
        Crumb {
            label: name,
            href: None,
        },
    ])
}
