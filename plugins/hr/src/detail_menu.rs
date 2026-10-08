//! Entity detail sidebars for HR record pages.

use maud::{Markup, html};

use lariv_core::components::{SidebarMenu, SidebarMenuItem, sidebar_menu, sidebar_menu_item_pane};

use super::routes::{
    ApplicantDetailRouteTag, EmployeeDetailRouteTag, EmployeeLeaveJournalRouteTag,
    ExEmployeeDetailRouteTag,
};

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

pub fn applicant_detail_menu(display_name: &str, applicant_id: i64, active: &str) -> Markup {
    entity_detail_menu(
        format!("Applicant: {display_name}"),
        ApplicantDetailRouteTag::new(applicant_id).url(),
        active,
    )
}

pub fn employee_detail_menu(display_name: &str, employee_id: i64, active: &str) -> Markup {
    detail_sidebar_menu(
        format!("Employee: {display_name}"),
        &[
            DetailMenuNavItem {
                title: "Detail",
                url: EmployeeDetailRouteTag::new(employee_id).url(),
                active: active == "detail",
            },
            DetailMenuNavItem {
                title: "Leaves journal",
                url: EmployeeLeaveJournalRouteTag::new(employee_id).url(),
                active: active == "leave-journal",
            },
        ],
    )
}

pub fn ex_employee_detail_menu(display_name: &str, ex_employee_id: i64, active: &str) -> Markup {
    entity_detail_menu(
        format!("Ex-employee: {display_name}"),
        ExEmployeeDetailRouteTag::new(ex_employee_id).url(),
        active,
    )
}

#[cfg(test)]
mod tests {
    use super::employee_detail_menu;

    #[test]
    fn employee_menu_links_to_the_leaves_journal() {
        let html = employee_detail_menu("Ada", 4, "leave-journal").into_string();
        assert!(html.contains("Leaves journal"));
        assert!(html.contains("/hr/employees/4/leave-journal"));
        assert!(html.contains("Detail"));
    }
}
