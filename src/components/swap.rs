//! Compile-time HTMX swap region keys.
//!
//! Every swappable DOM region is a Rust type implementing [`SwapKey`]. Call sites
//! use the type for `id`, `hx-target`, and out-of-band swaps — never free-form
//! selector strings.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::{HtmlAttrs, escape_attr};

/// A named DOM region that HTMX can target or swap out-of-band.
///
/// Implement via [`swap_key!`] so `ID` and `SELECTOR` stay in sync.
pub trait SwapKey {
    /// Element `id` attribute value (without `#`).
    const ID: &'static str;
    /// CSS selector for HTMX (`#` + [`ID`](Self::ID)).
    const SELECTOR: &'static str;
}

/// Declare a [`SwapKey`] type with a literal id.
///
/// ```ignore
/// swap_key!(UserTableKey, "user-table");
/// ```
#[macro_export]
macro_rules! swap_key {
    ($name:ident, $id:literal) => {
        #[derive(Debug, Clone, Copy, Default)]
        pub struct $name;

        impl $crate::components::swap::SwapKey for $name {
            const ID: &'static str = $id;
            const SELECTOR: &'static str = concat!("#", $id);
        }
    };
}

// Page pane under persistent topbar (sidebar + main). Prefer this for in-app
// navigation and form re-renders instead of full-document morphs.
swap_key!(AppLayoutKey, "app-layout");

// Inner content column (`<main>`) inside the app layout scaffold.
swap_key!(MainContentKey, "main-content");

/// Row / link navigation that replaces `#app-layout` (sidebar may change).
pub fn nav_main_attrs(url: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set(
            "class",
            "cursor-pointer hover:bg-base-200 transition-colors",
        )
        .set("hx-get", url)
        .set("hx-target", AppLayoutKey::SELECTOR)
        .set("hx-select", AppLayoutKey::SELECTOR)
        .set("hx-swap", "outerHTML")
        .set("hx-push-url", "true")
}

/// Sidebar menu-style navigation into [`MainContentKey`] only.
pub fn nav_content_attrs(url: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("hx-get", url)
        .set("hx-target", MainContentKey::SELECTOR)
        .set("hx-select", MainContentKey::SELECTOR)
        .set("hx-swap", "outerHTML")
        .set("hx-push-url", "true")
}

/// Portal where modal dialogs are appended (`document.body`).
#[derive(Debug, Clone, Copy, Default)]
pub struct ModalHostKey;

impl SwapKey for ModalHostKey {
    const ID: &'static str = "";
    /// HTMX target for appending modals as children of `body`.
    const SELECTOR: &'static str = "body";
}

/// `id="…"` for a region root element.
pub fn region_attrs<K: SwapKey>() -> HtmlAttrs {
    HtmlAttrs::new().set("id", K::ID)
}

/// Declarative HTMX targeting attrs for a keyed region (`hx-target` + outerHTML).
///
/// Prefer [`hx_target_swap`] with `"outerMorph"` for same-structure fragments (tables).
pub fn hx_target<K: SwapKey>() -> HtmlAttrs {
    hx_target_swap::<K>("outerHTML")
}

/// Like [`hx_target`] with an explicit swap strategy.
pub fn hx_target_swap<K: SwapKey>(swap: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("hx-target", K::SELECTOR)
        // Fragment responses are not full pages — clear body `hx-select:#app-layout`.
        // Empty string overrides inheritance; HTMX 4 has no `unset` keyword.
        .set("hx-select", "")
        .set("hx-swap", swap)
}

/// Attrs for an out-of-band fragment rooted at `K` (`id` + `hx-swap-oob`).
pub fn oob_attrs<K: SwapKey>() -> HtmlAttrs {
    HtmlAttrs::new()
        .set("id", K::ID)
        .set("hx-swap-oob", "true")
}

/// Attrs for OOB with an explicit swap strategy (e.g. `"outerHTML"`, `"innerHTML"`).
pub fn oob_attrs_swap<K: SwapKey>(swap: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("id", K::ID)
        .set("hx-swap-oob", swap)
}

/// Markup that deletes the keyed element via HTMX OOB (`hx-swap-oob="delete"`).
pub fn oob_delete<K: SwapKey>() -> Markup {
    if K::ID.is_empty() {
        return Markup::default();
    }
    html! {
        (PreEscaped(format!(
            r#"<div id="{}" hx-swap-oob="delete"></div>"#,
            escape_attr(K::ID)
        )))
    }
}

/// Wrap `inner` as an OOB swap for `K` (default OOB style).
///
/// `inner` must be the full replacement element including matching `id`.
pub fn oob_fragment(inner: Markup) -> Markup {
    inner
}

/// Concatenate primary + OOB fragments into one HTML response body.
pub fn fragment_response(parts: impl IntoIterator<Item = Markup>) -> Markup {
    html! {
        @for part in parts {
            (part)
        }
    }
}

/// HTMX attrs for a form that POSTs into a typed region.
pub fn form_hx_post<K: SwapKey>(action: &str) -> HtmlAttrs {
    form_hx_post_selector(action, K::SELECTOR)
}

/// HTMX attrs for a form that POSTs into a selector (when the key is dynamic).
pub fn form_hx_post_selector(action: &str, target: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("method", "POST")
        .set("hx-post", action)
        .set("hx-target", target)
        // Fragment responses are not full pages — clear body `hx-select:#app-layout`.
        .set("hx-select", "")
        .set("hx-swap", "outerMorph")
        .set("hx-push-url", "false")
}

/// POST into [`AppLayoutKey`] for scaffold/auth page forms.
///
/// Uses `outerHTML` so cross-page navigations (e.g. apps → users) replace the
/// pane cleanly; `outerMorph` can frankenstein Alpine trees across layouts.
pub fn form_hx_post_main(action: &str) -> HtmlAttrs {
    form_hx_post::<AppLayoutKey>(action)
        .set("hx-select", AppLayoutKey::SELECTOR)
        .set("hx-swap", "outerHTML")
        .set("hx-push-url", "true")
}

/// HTMX attrs for a GET form (filters) targeting a typed region.
pub fn form_hx_get<K: SwapKey>(action: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("method", "GET")
        .set("hx-get", action)
        .set("hx-target", K::SELECTOR)
        // Fragment responses are not full pages — clear body `hx-select:#app-layout`.
        .set("hx-select", "")
        .set("hx-swap", "outerMorph")
        .set("hx-push-url", "true")
}

#[cfg(test)]
mod tests {
    use super::*;

    swap_key!(TestTableKey, "test-table");

    #[test]
    fn swap_key_id_and_selector() {
        assert_eq!(TestTableKey::ID, "test-table");
        assert_eq!(TestTableKey::SELECTOR, "#test-table");
        assert_eq!(MainContentKey::SELECTOR, "#main-content");
        assert_eq!(AppLayoutKey::SELECTOR, "#app-layout");
        assert_eq!(ModalHostKey::SELECTOR, "body");
    }

    #[test]
    fn region_and_target_attrs() {
        let r = region_attrs::<TestTableKey>().as_string();
        assert!(r.contains(r#"id="test-table""#));

        let t = hx_target::<TestTableKey>().as_string();
        assert!(t.contains("hx-target=\"#test-table\""));
        assert!(t.contains("hx-select=\"\""));
        assert!(t.contains("hx-swap=\"outerHTML\""));

        let o = oob_attrs::<TestTableKey>().as_string();
        assert!(o.contains("hx-swap-oob=\"true\""));
        assert!(o.contains("id=\"test-table\""));
    }

    #[test]
    fn oob_delete_emits_marker() {
        let html = oob_delete::<TestTableKey>().into_string();
        assert!(html.contains("id=\"test-table\""));
        assert!(html.contains("hx-swap-oob=\"delete\""));
    }

    #[test]
    fn form_hx_helpers() {
        let post = form_hx_post::<TestTableKey>("/users/create/").as_string();
        assert!(post.contains("hx-post=\"/users/create/\""));
        assert!(post.contains("hx-target=\"#test-table\""));
        assert!(post.contains("hx-select=\"\""));

        let get = form_hx_get::<TestTableKey>("/users/").as_string();
        assert!(get.contains("hx-get=\"/users/\""));
        assert!(get.contains("hx-push-url=\"true\""));
        assert!(get.contains("hx-select=\"\""));

        let main = form_hx_post_main("/users/login").as_string();
        assert!(main.contains("hx-select=\"#app-layout\""));
    }
}
