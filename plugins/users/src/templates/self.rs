use maud::{Markup, html};

use lariv_core::components::{SidebarMenu, SidebarMenuItem, sidebar_menu, sidebar_menu_item_pane};

use crate::routes::{UsersSelfChangePasswordGetRouteTag, UsersSelfRouteTag};

pub(crate) fn user_self_menu(user_name: &str, active: &str) -> Markup {
    let title = format!("My account: {user_name}");
    let detail_url = UsersSelfRouteTag.url();
    let pw_url = UsersSelfChangePasswordGetRouteTag.url();
    sidebar_menu(SidebarMenu {
        title: &title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "My Profile",
                url: &detail_url,
                active: active == "detail",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Change Password",
                url: &pw_url,
                active: active == "password",
                ..Default::default()
            }))
        },
    })
}
