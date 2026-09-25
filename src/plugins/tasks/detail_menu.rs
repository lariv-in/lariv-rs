//! Entity detail sidebars for task pages.

use maud::{Markup, html};

use crate::components::{SidebarMenu, SidebarMenuItem, sidebar_menu, sidebar_menu_item_pane};

use super::routes::{TaskDetailRouteTag, TaskLogsRouteTag, TaskStatusDetailRouteTag};

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

pub fn task_detail_menu(title: &str, id: i64, active: &str) -> Markup {
    detail_sidebar_menu(
        format!("Task: {title}"),
        &[
            DetailMenuNavItem {
                title: "Detail",
                url: TaskDetailRouteTag::new(id).url(),
                active: active == "detail",
            },
            DetailMenuNavItem {
                title: "Logs",
                url: TaskLogsRouteTag::new(id).url(),
                active: active == "logs",
            },
        ],
    )
}

pub fn status_detail_menu(name: &str, id: i64, active: &str) -> Markup {
    entity_detail_menu(
        format!("Status: {name}"),
        TaskStatusDetailRouteTag::new(id).url(),
        active,
    )
}
