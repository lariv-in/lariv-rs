//! Sidebar navigation menus.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::escape_attr;
use crate::components::button::{ButtonDeletePost, button_delete_post_route};
use crate::components::htmx::{HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL};
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
    let url = crate::components::nav_origin::with_nav_origin(opts.url);
    let active_class = if opts.active { " menu-active" } else { "" };
    let class = active_class.trim_start();
    html! {
        li {
            (PreEscaped(format!(
                r#"<a href="{url}" hx-get="{url}" hx-target="{target}" hx-select="{select}" hx-swap="outerHTML" hx-push-url="true" @click="closeLeft()"{class_attr}>"#,
                url = escape_attr(&url),
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
    _form_post_url: &str,
    label: &str,
    _modal_uid: &str,
) -> Markup {
    sidebar_menu_modal_form_item(SidebarMenuModalForm {
        label,
        href,
        name: "",
        classes: "text-error",
        ..Default::default()
    })
}

/// Options for a sidebar modal-form opener rendered as a DaisyUI menu item (`<li><a>`).
pub struct SidebarMenuModalForm<'a> {
    pub label: &'a str,
    pub href: &'a str,
    /// Optional `name` query param identifying the form (create modals).
    pub name: &'a str,
    pub icon_name: Option<&'a str>,
    /// Extra classes on the anchor (e.g. `text-error` for destructive actions).
    pub classes: &'a str,
}

impl Default for SidebarMenuModalForm<'_> {
    fn default() -> Self {
        Self {
            label: "",
            href: "#",
            name: "",
            icon_name: None,
            classes: "",
        }
    }
}

/// Modal opener styled as a normal sidebar menu item — not a nested `.btn`.
pub fn sidebar_menu_modal_form_item(opts: SidebarMenuModalForm<'_>) -> Markup {
    let mut href = opts.href.to_string();
    if !opts.name.is_empty() {
        let sep = if href.contains('?') { '&' } else { '?' };
        href = format!("{href}{sep}name={}", opts.name);
    }

    // Same refresh wiring as [`crate::components::button_modal_form`], without button chrome.
    let refresh_js = concat!(
        "var t=this.closest('.data-table-container');",
        "var id=t?t.id:'';",
        "if(typeof ctx!=='undefined'&&ctx.request){",
        "var u=new URL(ctx.request.action,location.href);",
        "if(id){u.searchParams.set('refresh',id)}else{u.searchParams.delete('refresh')}",
        "ctx.request.action=u.pathname+u.search+u.hash;",
        "if(ctx.request.body&&ctx.request.body.set){ctx.request.body.set('refresh',id)}",
        "}else{var p=event.detail.parameters;if(p&&p.set){p.set('refresh',id)}else if(p){p.refresh=id}}",
    );
    let class_attr = if opts.classes.is_empty() {
        String::new()
    } else {
        format!(r#" class="{}""#, escape_attr(opts.classes))
    };

    html! {
        li {
            (PreEscaped(format!(
                r#"<a href="{href}" hx-get="{href}" hx-target="{target}" hx-swap="{swap}" hx-push-url="false" hx-on:htmx:config-request="{js}" hx-on:htmx:config:request="{js}" @click="closeLeft()"{class_attr}>"#,
                href = escape_attr(&href),
                target = escape_attr(HTMX_TARGET_BODY_MODAL),
                swap = escape_attr(HTMX_SWAP_BODY_MODAL),
                js = escape_attr(refresh_js),
                class_attr = class_attr,
            )))
            @if let Some(name) = opts.icon_name {
                (icon(name, "heroicon-sm"))
            }
            (opts.label)
            (PreEscaped("</a>"))
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

    #[test]
    fn sidebar_modal_form_item_is_menu_anchor_not_button() {
        let html = sidebar_menu_modal_form_item(SidebarMenuModalForm {
            label: "Create Item",
            href: "/filesystem/create",
            name: "p_filesystem.VNodeCreateForm",
            ..Default::default()
        })
        .into_string();
        assert!(html.contains("<li>"));
        assert!(html.contains("<a "));
        assert!(html.contains("Create Item</a>"));
        assert!(html.contains("hx-get=\"/filesystem/create?name=p_filesystem.VNodeCreateForm\""));
        assert!(!html.contains("btn"));
        assert!(!html.contains("<button"));
        assert!(!html.contains("fk-modal-host"));
    }
}
