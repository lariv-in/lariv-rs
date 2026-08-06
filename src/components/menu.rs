//! Sidebar navigation menus.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::escape_attr;
use crate::components::button::{
    ButtonDeletePost, ButtonModalForm, button_delete_post_route, button_modal_form,
};
use crate::components::swap::{AppLayoutKey, MainContentKey, SwapKey};
use crate::components::text::icon;
use crate::http::{BoostPost, RouteUrl};

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
    sidebar_menu_item_target(opts, MainContentKey::SELECTOR, MainContentKey::SELECTOR)
}

/// Sidebar item that replaces `#app-layout` (detail sidebar, delete page, etc.).
pub fn sidebar_menu_item_pane(opts: SidebarMenuItem<'_>) -> Markup {
    sidebar_menu_item_target(opts, AppLayoutKey::SELECTOR, AppLayoutKey::SELECTOR)
}

fn sidebar_menu_item_target(opts: SidebarMenuItem<'_>, target: &str, select: &str) -> Markup {
    // "Back" links use [`button_link`] with explicit `#app-layout` nav attrs.
    let active_class = if opts.active { " menu-active" } else { "" };
    let class = active_class.trim_start();
    html! {
        li {
            (PreEscaped(format!(
                r#"<a href="{url}" hx-get="{url}" hx-target="{target}" hx-select="{select}" hx-swap="outerHTML" hx-push-url="true" @click="closeLeft()"{class_attr}>"#,
                url = escape_attr(opts.url),
                target = escape_attr(target),
                select = escape_attr(select),
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

#[derive(Default)]
pub struct SidebarMenu<'a> {
    pub title: &'a str,
    pub children: Markup,
}

/// Sidebar menu item that POSTs with `hx-confirm` (e.g. post draft invoice).
pub fn sidebar_menu_post_confirm_route(
    route: impl RouteUrl + BoostPost,
    label: &str,
    confirm: &str,
) -> Markup {
    html! {
        li {
            (button_delete_post_route(
                route,
                ButtonDeletePost {
                    label,
                    confirm,
                    classes: "btn-ghost btn-sm w-full justify-start font-normal",
                },
            ))
        }
    }
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
            @if !opts.title.is_empty() {
                li class="menu-title font-semibold opacity-70" { (opts.title) }
            }
            (opts.children)
        }
    }
}

/// Navigable sidebar section link with optional extra path prefixes for active matching.
#[derive(Clone, Copy, Debug)]
pub struct SidebarNavLink<'a> {
    pub key: &'a str,
    pub title: &'a str,
    pub url: &'a str,
    pub icon_name: Option<&'a str>,
    /// Extra path prefixes that mark this link active. Empty ⇒ use [`Self::url`] only.
    pub match_prefixes: &'a [&'a str],
}

/// Strip query string and trailing slash (except root) for sidebar matching.
pub fn normalize_nav_path(path_and_query: &str) -> String {
    let path = path_and_query.split('?').next().unwrap_or(path_and_query);
    if path.len() > 1 && path.ends_with('/') {
        path.trim_end_matches('/').to_owned()
    } else {
        path.to_owned()
    }
}

fn path_matches_prefix(path: &str, prefix: &str) -> bool {
    let prefix = normalize_nav_path(prefix);
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

fn consider_prefix<'a>(
    path: &str,
    prefix: &str,
    key: &'a str,
    best: &mut Option<(usize, &'a str)>,
) {
    if path_matches_prefix(path, prefix) {
        let len = normalize_nav_path(prefix).len();
        if best.map(|(best_len, _)| len > best_len).unwrap_or(true) {
            *best = Some((len, key));
        }
    }
}

/// Longest matching prefix wins; returns the active nav key.
pub fn active_nav_key<'a>(links: &[SidebarNavLink<'a>], current_path: &str) -> Option<&'a str> {
    let path = normalize_nav_path(current_path);
    let mut best: Option<(usize, &'a str)> = None;
    for link in links {
        if link.match_prefixes.is_empty() {
            consider_prefix(&path, link.url, link.key, &mut best);
        } else {
            for prefix in link.match_prefixes {
                consider_prefix(&path, prefix, link.key, &mut best);
            }
        }
    }
    best.map(|(_, key)| key)
}

/// Pane-swapping menu items with `menu-active` derived from `current_path`.
pub fn sidebar_nav_items_pane(links: &[SidebarNavLink<'_>], current_path: &str) -> Markup {
    let active = active_nav_key(links, current_path);
    let mut items = Markup::default();
    for link in links {
        let is_active = active == Some(link.key);
        items = html! {
            (items)
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: link.title,
                url: link.url,
                icon_name: link.icon_name,
                active: is_active,
            }))
        };
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_links() -> [SidebarNavLink<'static>; 4] {
        [
            SidebarNavLink {
                key: "users",
                title: "All Users",
                url: "/users/",
                icon_name: None,
                match_prefixes: &[],
            },
            SidebarNavLink {
                key: "roles",
                title: "Roles",
                url: "/users/roles/",
                icon_name: None,
                match_prefixes: &[],
            },
            SidebarNavLink {
                key: "journals",
                title: "Journals",
                url: "/finance/journals/",
                icon_name: None,
                match_prefixes: &["/finance/journals", "/finance/journal-entries"],
            },
            SidebarNavLink {
                key: "accounts",
                title: "Accounts",
                url: "/finance/",
                icon_name: None,
                match_prefixes: &["/finance", "/finance/accounts"],
            },
        ]
    }

    #[test]
    fn active_nav_longest_prefix() {
        let links = sample_links();
        assert_eq!(active_nav_key(&links, "/users"), Some("users"));
        assert_eq!(active_nav_key(&links, "/users/"), Some("users"));
        assert_eq!(active_nav_key(&links, "/users?page=2"), Some("users"));
        assert_eq!(active_nav_key(&links, "/users/roles"), Some("roles"));
        assert_eq!(active_nav_key(&links, "/users/roles/create"), Some("roles"));
        assert_eq!(active_nav_key(&links, "/finance"), Some("accounts"));
        assert_eq!(
            active_nav_key(&links, "/finance/journals?page=2"),
            Some("journals")
        );
        assert_eq!(
            active_nav_key(&links, "/finance/journal-entries/9"),
            Some("journals")
        );
    }

    #[test]
    fn sidebar_nav_items_pane_marks_active() {
        let links = sample_links();
        let html = sidebar_nav_items_pane(&links, "/users/roles").into_string();
        assert!(html.contains("hx-target=\"#app-layout\""));
        assert!(html.contains(r#"class="menu-active""#));
        // Active class sits on the Roles anchor, not All Users.
        let roles_anchor_end = html.find(">Roles</a>").expect("roles link");
        let roles_open = html[..roles_anchor_end].rfind("<a ").expect("roles <a");
        assert!(html[roles_open..roles_anchor_end].contains(r#"class="menu-active""#));
        let users_anchor_end = html.find(">All Users</a>").expect("users link");
        let users_open = html[..users_anchor_end].rfind("<a ").expect("users <a");
        assert!(!html[users_open..users_anchor_end].contains("menu-active"));
    }
}
