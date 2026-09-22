//! Breadcrumb trails for HR pages.

use maud::Markup;

use crate::components::{Crumb, breadcrumbs};

use super::routes::{ApplicantHubRouteTag, JobFormListRouteTag};

fn hub_tab_url(tab: &str) -> String {
    crate::http::RouteQueryBuilder::new(ApplicantHubRouteTag)
        .query("tab", tab)
        .build()
}

pub fn hub_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "People",
        href: None,
    }])
}

pub fn job_forms_crumbs(label: &str) -> Markup {
    let list_url = JobFormListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Job postings",
            href: Some(&list_url),
        },
        Crumb { label, href: None },
    ])
}

pub fn applicant_crumbs(name: &str) -> Markup {
    breadcrumbs(&[
        Crumb {
            label: "People",
            href: Some(&hub_tab_url("applicants")),
        },
        Crumb {
            label: name,
            href: None,
        },
    ])
}

pub fn probation_crumbs(name: &str) -> Markup {
    breadcrumbs(&[
        Crumb {
            label: "People",
            href: Some(&hub_tab_url("probation")),
        },
        Crumb {
            label: name,
            href: None,
        },
    ])
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
