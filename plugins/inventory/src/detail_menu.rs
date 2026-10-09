//! Entity detail sidebars.

use maud::{Markup, html};

use lariv_core::components::{SidebarMenu, SidebarMenuItem, sidebar_menu, sidebar_menu_item_pane};

use super::routes::{MovementDetailRouteTag, StockDetailRouteTag};

pub fn stock_detail_menu(name: &str, id: i64) -> Markup {
    let title = format!("Stock: {name}");
    sidebar_menu(SidebarMenu {
        title: title.as_str(),
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Detail",
                url: &StockDetailRouteTag::new(id).url(),
                active: true,
                ..Default::default()
            }))
        },
    })
}

pub fn movement_detail_menu(label: &str, id: i64) -> Markup {
    let title = format!("Movement: {label}");
    sidebar_menu(SidebarMenu {
        title: title.as_str(),
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Detail",
                url: &MovementDetailRouteTag::new(id).url(),
                active: true,
                ..Default::default()
            }))
        },
    })
}
