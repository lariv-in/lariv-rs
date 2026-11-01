//! Breadcrumb trails for HR holiday pages.

use maud::Markup;

use lariv_core::components::{Crumb, breadcrumbs};

use super::routes::HolidayListRouteTag;

pub fn holidays_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Holidays",
        href: None,
    }])
}

pub fn holiday_crumbs(title: &str) -> Markup {
    let list_url = HolidayListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Holidays",
            href: Some(&list_url),
        },
        Crumb {
            label: title,
            href: None,
        },
    ])
}
