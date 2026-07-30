//! Sidebar navigation menus (Go `SidebarMenu` / `SidebarMenuItem` ports).

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::escape_attr;
use crate::components::button::{ButtonLink, button_link};
use crate::components::swap::{MainContentKey, SwapKey};
use crate::components::text::icon;

pub struct SidebarMenuItem<'a> {
    pub title: &'a str,
    pub url: &'a str,
    pub icon_name: Option<&'a str>,
    pub active: bool,
}

impl Default for SidebarMenuItem<'_> {
    fn default() -> Self {
        Self {
            title: "",
            url: "#",
            icon_name: None,
            active: false,
        }
    }
}

pub fn sidebar_menu_item(opts: SidebarMenuItem<'_>) -> Markup {
    // Stay inside the scaffold: swap `<main id="main-content">`, not `#app-layout`.
    // "Back" links use [`button_link`] and inherit body `#app-layout` boost instead.
    let active_class = if opts.active { " menu-active" } else { "" };
    let class = active_class.trim_start();
    html! {
        li {
            (PreEscaped(format!(
                r#"<a href="{url}" hx-get="{url}" hx-target="{target}" hx-select="{target}" hx-swap="outerHTML" hx-push-url="true"{class_attr}>"#,
                url = escape_attr(opts.url),
                target = MainContentKey::SELECTOR,
                class_attr = if class.is_empty() {
                    String::new()
                } else {
                    format!(r#" class="{}""#, escape_attr(class))
                },
            )))
            @if let Some(name) = opts.icon_name {
                (icon(name, "heroicon-sm"))
            }
            (opts.title)
            (PreEscaped("</a>"))
        }
    }
}

pub struct SidebarMenuBack<'a> {
    pub title: &'a str,
    pub url: &'a str,
}

#[derive(Default)]
pub struct SidebarMenu<'a> {
    pub title: &'a str,
    pub back: Option<SidebarMenuBack<'a>>,
    pub children: Markup,
}

pub fn sidebar_menu(opts: SidebarMenu<'_>) -> Markup {
    html! {
        ul class="menu w-full wrap-anywhere" {
            @if let Some(back) = opts.back {
                li {
                    (button_link(ButtonLink {
                        label: back.title,
                        href: back.url,
                        icon_name: Some("arrow-left"),
                        classes: "btn-sm mb-2",
                        ..Default::default()
                    }))
                }
            }
            @if !opts.title.is_empty() {
                li class="menu-title font-semibold opacity-70" { (opts.title) }
            }
            (opts.children)
        }
    }
}
