//! Breadcrumb trails for overtime pages.

use maud::Markup;

use lariv_core::components::{Crumb, breadcrumbs};

pub fn overtime_list_crumbs(label: &str) -> Markup {
    breadcrumbs(&[Crumb { label, href: None }])
}

pub fn overtime_crumbs(parent: &str, parent_href: &str, title: &str) -> Markup {
    breadcrumbs(&[
        Crumb {
            label: parent,
            href: Some(parent_href),
        },
        Crumb {
            label: title,
            href: None,
        },
    ])
}
