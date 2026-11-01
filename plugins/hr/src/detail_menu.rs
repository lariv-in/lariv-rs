//! Entity detail sidebars for HR record pages.

use maud::{Markup, html};

use lariv_core::components::{SidebarMenu, SidebarMenuItem, sidebar_menu, sidebar_menu_item_pane};

use super::hr_employee_detail::employee_detail_links;
use super::routes::{EmployeeDetailRouteTag, ExEmployeeDetailRouteTag};

struct DetailMenuNavItem {
    title: &'static str,
    url: String,
    active: bool,
}

fn detail_sidebar_menu(menu_title: String, nav_items: &[DetailMenuNavItem]) -> Markup {
    sidebar_menu(SidebarMenu {
        title: menu_title.as_str(),
        children: {
            let mut children = Markup::default();
            for item in nav_items {
                children = html! {
                    (children)
                    (sidebar_menu_item_pane(SidebarMenuItem {
                        title: item.title,
                        url: &item.url,
                        active: item.active,
                        ..Default::default()
                    }))
                };
            }
            children
        },
    })
}

fn entity_detail_menu(menu_title: String, detail_url: String, active: &str) -> Markup {
    detail_sidebar_menu(
        menu_title,
        &[DetailMenuNavItem {
            title: "Detail",
            url: detail_url,
            active: active == "detail",
        }],
    )
}

pub fn employee_detail_menu(display_name: &str, employee_id: i64, active: &str) -> Markup {
    let mut items = vec![DetailMenuNavItem {
        title: "Detail",
        url: EmployeeDetailRouteTag::new(employee_id).url(),
        active: active == "detail",
    }];
    for link in employee_detail_links() {
        items.push(DetailMenuNavItem {
            title: link.title,
            url: (link.url_fn)(employee_id),
            active: active == link.key,
        });
    }
    detail_sidebar_menu(format!("Employee: {display_name}"), &items)
}

pub fn ex_employee_detail_menu(display_name: &str, ex_employee_id: i64, active: &str) -> Markup {
    entity_detail_menu(
        format!("Ex-employee: {display_name}"),
        ExEmployeeDetailRouteTag::new(ex_employee_id).url(),
        active,
    )
}
