//! Breadcrumb trails for job posting and applicant pages.

use maud::Markup;

use lariv_core::components::{Crumb, breadcrumbs};

use super::routes::{ApplicantListRouteTag, JobFormListRouteTag};

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

pub fn applicants_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Applicants",
        href: None,
    }])
}

pub fn applicant_crumbs(name: &str) -> Markup {
    let list_url = ApplicantListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Applicants",
            href: Some(&list_url),
        },
        Crumb {
            label: name,
            href: None,
        },
    ])
}
