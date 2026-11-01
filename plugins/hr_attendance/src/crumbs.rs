//! Breadcrumb trails for HR attendance pages.

use maud::Markup;

use lariv_core::components::{Crumb, breadcrumbs};

use super::routes::AttendanceListRouteTag;

pub fn attendances_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Attendance",
        href: None,
    }])
}

pub fn attendance_crumbs(title: &str) -> Markup {
    let list_url = AttendanceListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Attendance",
            href: Some(&list_url),
        },
        Crumb {
            label: title,
            href: None,
        },
    ])
}
