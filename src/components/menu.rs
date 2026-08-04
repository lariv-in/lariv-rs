//! Sidebar navigation menus.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::escape_attr;
use crate::components::button::{ButtonLink, ButtonModalForm, button_link, button_modal_form};
use crate::components::swap::{MainContentKey, SwapKey};
use crate::components::text::icon;
use crate::http::RouteUrl;

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
    // "Back" links use [`button_link`] with explicit `#app-layout` nav attrs.
    let active_class = if opts.active { " menu-active" } else { "" };
    let class = active_class.trim_start();
    html! {
        li {
            (PreEscaped(format!(
                r#"<a href="{url}" hx-get="{url}" hx-target="{target}" hx-swap="outerHTML" hx-push-url="true" @click="closeLeft()"{class_attr}>"#,
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

/// Sidebar menu item that opens a delete (or other) confirmation modal.
pub fn sidebar_menu_modal_form_route(
    get_route: impl RouteUrl,
    post_route: impl RouteUrl,
    label: &str,
    modal_uid: &str,
) -> Markup {
    let href = get_route.url();
    let post_url = post_route.path();
    sidebar_menu_modal_form_urls(&href, &post_url, label, modal_uid)
}

/// Sidebar menu item that opens a confirmation modal (pre-built GET href and POST path).
pub fn sidebar_menu_modal_form_urls(
    href: &str,
    form_post_url: &str,
    label: &str,
    modal_uid: &str,
) -> Markup {
    html! {
        li {
            (button_modal_form(ButtonModalForm {
                label,
                href,
                form_post_url,
                modal_uid,
                classes: "btn-ghost btn-sm w-full justify-start font-normal text-error",
                ..Default::default()
            }))
        }
    }
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
