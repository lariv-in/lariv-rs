//! Shared entity detail sidebar menus for accounting plugins.

use maud::{Markup, html};

use crate::components::{SidebarMenu, SidebarMenuItem, sidebar_menu, sidebar_menu_item_pane};

/// One navigational item in an entity detail sidebar.
pub struct DetailMenuNavItem {
    pub title: &'static str,
    pub url: String,
    pub active: bool,
}

/// Full-page delete navigation.
pub struct DetailMenuDeleteLink {
    pub title: &'static str,
    pub url: String,
    pub active: bool,
}

/// Build a detail/edit sidebar for an accounting entity.
pub fn detail_sidebar_menu(
    menu_title: String,
    nav_items: &[DetailMenuNavItem],
    delete_link: Option<DetailMenuDeleteLink>,
    extra: Markup,
) -> Markup {
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
            if let Some(d) = delete_link {
                children = html! {
                    (children)
                    (sidebar_menu_item_pane(SidebarMenuItem {
                        title: d.title,
                        url: &d.url,
                        active: d.active,
                        ..Default::default()
                    }))
                };
            }
            html! {
                (children)
                (extra)
            }
        },
    })
}
