//! Breadcrumb trails for HR leave pages.

use maud::Markup;

use lariv_core::components::{Crumb, breadcrumbs};
use lariv_plugin_hr::routes::EmployeeDetailRouteTag;

pub fn leaves_list_crumbs(label: &str) -> Markup {
    breadcrumbs(&[Crumb { label, href: None }])
}

pub fn leave_crumbs(parent: &str, parent_href: &str, title: &str) -> Markup {
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

pub fn employee_leave_journal_crumbs(name: &str, employee_id: i64) -> Markup {
    let detail = EmployeeDetailRouteTag::new(employee_id).url();
    breadcrumbs(&[
        Crumb {
            label: "People",
            href: None,
        },
        Crumb {
            label: name,
            href: Some(&detail),
        },
        Crumb {
            label: "Leaves journal",
            href: None,
        },
    ])
}
