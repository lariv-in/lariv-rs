//! Compile-time HTMX swap region keys.
//!
//! Every swappable DOM region is a Rust type implementing [`SwapKey`]. Call sites
//! use the type for `id`, `hx-target`, and out-of-band swaps — never free-form
//! selector strings.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::{HtmlAttrs, escape_attr};
use crate::http::{AppPanePost, BoostPost, FileDownloadPost, FkSelectGet, FragmentGet, FragmentPost, RouteUrl};

/// A named DOM region that HTMX can target or swap out-of-band.
///
/// Implement via [`swap_key!`](crate::swap_key) so `ID` and `SELECTOR` stay in sync.
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

/// `id="app-layout" hx-history-elt` — HTMX 4 swaps this on back/forward navigation.
pub fn app_layout_history_attrs() -> String {
    format!(r#"id="{}" hx-history-elt"#, AppLayoutKey::ID)
}

/// In-app navigation that replaces `#app-layout` with an explicit URL.
///
/// Prefer [`hx_nav_app_layout`] for typed app-pane GET routes.
pub fn hx_nav_app_layout_for_url(url: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("hx-get", url)
        .set("hx-target", AppLayoutKey::SELECTOR)
        .set("hx-select", AppLayoutKey::SELECTOR)
        .set("hx-swap", "outerHTML")
        .set("hx-push-url", "true")
}

/// Typed in-app navigation for an app-pane GET route value.
pub fn hx_nav_app_layout(route: impl RouteUrl) -> HtmlAttrs {
    hx_nav_app_layout_for_url(&route.url())
}

/// Typed in-app navigation with a pre-built URL string (dynamic dashboard tiles, etc.).
pub fn hx_nav_app_layout_url(url: impl AsRef<str>) -> HtmlAttrs {
    hx_nav_app_layout_for_url(url.as_ref())
}

/// Row click navigation into `#app-layout`.
pub fn nav_main_attrs(url: &str) -> HtmlAttrs {
    hx_nav_app_layout_for_url(url).set(
        "class",
        "cursor-pointer hover:bg-base-200 transition-colors",
    )
}

/// Boosted POST form that replaces `#app-layout`.
pub fn form_hx_boost_post_main(action: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("method", "POST")
        .set("hx-boost", "true")
        .set("hx-target", AppLayoutKey::SELECTOR)
        .set("hx-select", AppLayoutKey::SELECTOR)
        .set("hx-swap", "outerHTML")
        .set("hx-push-url", "true")
        .set("action", action)
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

/// HTMX attrs for a form that POSTs into a typed region with an explicit URL.
///
/// Prefer [`form_hx_post_route`] for typed fragment POST routes.
/// Internal escape hatch for query-param or non-standard route kinds.
pub(crate) fn form_hx_post_for_url<K: SwapKey>(url: &str) -> HtmlAttrs {
    form_hx_post_selector(url, K::SELECTOR)
}

/// Typed POST form targeting a fragment route value.
pub fn form_hx_post_route<K: SwapKey, R: RouteUrl + FragmentPost<K>>(route: R) -> HtmlAttrs {
    form_hx_post_for_url::<K>(&route.path())
}

/// Typed POST form with query string from a [`RouteQueryBuilder`](crate::http::route_tag::RouteQueryBuilder) result URL.
pub fn form_hx_post_url<K: SwapKey>(url: &str) -> HtmlAttrs {
    form_hx_post_for_url::<K>(url)
}

/// HTMX attrs for a form that POSTs into a selector (when the key is dynamic).
pub fn form_hx_post_selector(action: &str, target: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("method", "POST")
        .set("hx-post", action)
        .set("hx-target", target)
        .set("hx-swap", "outerMorph")
        .set("hx-push-url", "false")
}

/// POST into [`AppLayoutKey`] with an explicit URL (e.g. redirect-only POST routes).
///
/// Prefer [`form_hx_post_main`] or [`form_hx_post_main_url`].
pub(crate) fn form_hx_post_main_for_url(url: &str) -> HtmlAttrs {
    form_hx_post_for_url::<AppLayoutKey>(url)
        .set("hx-select", AppLayoutKey::SELECTOR)
        .set("hx-swap", "outerHTML")
        .set("hx-push-url", "true")
}

/// Typed POST form that replaces `#app-layout` on validation/persistence errors.
///
/// Requires an [`AppPanePost`] route — create/edit handlers must re-render the form
/// pane on failure instead of returning a redirect (see [`form_hx_post_redirect`]).
pub fn form_hx_post_main(route: impl RouteUrl + AppPanePost) -> HtmlAttrs {
    form_hx_post_main_for_url(&route.path())
}

/// POST form for redirect-only handlers ([`BoostPost`] delete, logout, etc.).
pub fn form_hx_post_redirect(route: impl RouteUrl + BoostPost) -> HtmlAttrs {
    form_hx_post_main_for_url(&route.path())
}

/// POST into `#app-layout` with an explicit URL (query strings on [`AppPanePost`] routes).
pub fn form_hx_post_main_url(url: &str) -> HtmlAttrs {
    form_hx_post_main_for_url(url)
}

/// Plain POST form for file-download routes (no HTMX swap).
pub fn form_post_download(action: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("method", "POST")
        .set("action", action)
        .set("hx-boost", "false")
}

/// Typed plain POST form for a file-download route value.
pub fn form_post_download_route<R: RouteUrl + FileDownloadPost>(route: R) -> HtmlAttrs {
    form_post_download(&route.path())
}

/// HTMX attrs for a GET form (filters) with an explicit URL.
///
/// Prefer [`form_hx_get_route`] or [`form_hx_get_url`].
pub(crate) fn form_hx_get_for_url<K: SwapKey>(url: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("method", "GET")
        .set("hx-get", url)
        .set("hx-target", K::SELECTOR)
        .set("hx-swap", "outerMorph")
        .set("hx-push-url", "true")
}

/// Typed GET filter form targeting a fragment route value.
pub fn form_hx_get_route<K: SwapKey, R: RouteUrl + FragmentGet<K>>(route: R) -> HtmlAttrs {
    form_hx_get_for_url::<K>(&route.path())
}

/// GET filter form inside an FK picker modal.
///
/// Targets the modal dialog (not the inner table) so HTMX attribute inheritance from
/// `dialog[hx-target=this]` matches the full-modal response from [`respond_picker_select`].
///
/// Requires an [`FkSelectGet`] route — use this instead of [`form_hx_get_route`] for picker
/// filter forms so HTMX swaps `outerHTML` on the dialog rather than `outerMorph` on rows
/// that carry Alpine `@click` attrs (which throws `DOMException: invalid character`).
pub fn form_hx_get_picker_route<K: SwapKey, M: SwapKey, R: RouteUrl + FkSelectGet<K, M>>(
    route: R,
) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("method", "GET")
        .set("hx-get", route.path())
        .set("hx-target", M::SELECTOR)
        .set("hx-swap", "outerHTML")
        .set("hx-push-url", "false")
}

/// GET filter form with an explicit URL.
pub fn form_hx_get_url<K: SwapKey>(url: &str) -> HtmlAttrs {
    form_hx_get_for_url::<K>(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{AppPanePost, FileDownloadPost, FkSelectGet, RouteTag, RouteUrl};

    swap_key!(TestTableKey, "test-table");
    swap_key!(TestPickerTableKey, "test-picker-table");
    swap_key!(TestPickerModalKey, "test-picker-modal");

    #[derive(Clone, Copy, Default)]
    struct TestAppPanePostRoute;
    impl RouteTag for TestAppPanePostRoute {
        const PATH: &'static str = "/test/create";
    }
    impl AppPanePost for TestAppPanePostRoute {}
    impl FkSelectGet<TestPickerTableKey, TestPickerModalKey> for TestAppPanePostRoute {}
    impl RouteUrl for TestAppPanePostRoute {
        fn path(self) -> String {
            Self::PATH.to_owned()
        }
        fn url(self) -> String {
            crate::http::trailing_slash(&self.path())
        }
    }

    #[derive(Clone, Copy, Default)]
    struct TestFileDownloadRoute;
    impl RouteTag for TestFileDownloadRoute {
        const PATH: &'static str = "/test/download";
    }
    impl FileDownloadPost for TestFileDownloadRoute {}
    impl RouteUrl for TestFileDownloadRoute {
        fn path(self) -> String {
            Self::PATH.to_owned()
        }
        fn url(self) -> String {
            crate::http::trailing_slash(&self.path())
        }
    }

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
        assert!(!t.contains("hx-select"));
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
    fn app_layout_history_attrs_includes_elt() {
        let attrs = super::app_layout_history_attrs();
        assert!(attrs.contains("app-layout"));
        assert!(attrs.contains("hx-history-elt"));
    }

    #[test]
    fn form_hx_helpers() {
        let post = form_hx_post_for_url::<TestTableKey>("/users/create/").as_string();
        assert!(post.contains("hx-post=\"/users/create/\""));
        assert!(post.contains("hx-target=\"#test-table\""));
        assert!(!post.contains("hx-select"));

        let get = form_hx_get_for_url::<TestTableKey>("/users/").as_string();
        assert!(get.contains("hx-get=\"/users/\""));
        assert!(get.contains("hx-push-url=\"true\""));
        assert!(!get.contains("hx-select"));

        let picker = form_hx_get_picker_route::<TestPickerTableKey, TestPickerModalKey, _>(
            TestAppPanePostRoute,
        )
        .as_string();
        assert!(picker.contains("hx-get=\"/test/create\""));
        assert!(picker.contains("hx-target=\"#test-picker-modal\""));
        assert!(picker.contains("hx-swap=\"outerHTML\""));
        assert!(picker.contains("hx-push-url=\"false\""));

        let main = form_hx_post_main_for_url("/users/login").as_string();
        assert!(main.contains("hx-select=\"#app-layout\""));

        let nav = hx_nav_app_layout_for_url("/dashboard/").as_string();
        assert!(nav.contains("hx-get=\"/dashboard/\""));
        assert!(nav.contains("hx-target=\"#app-layout\""));

        let boost = form_hx_boost_post_main("/users/logout/").as_string();
        assert!(boost.contains("hx-boost=\"true\""));
        assert!(boost.contains("hx-target=\"#app-layout\""));
    }

    #[test]
    fn typed_route_helpers_use_path_from_route_tag() {
        let main = form_hx_post_main(TestAppPanePostRoute).as_string();
        assert!(main.contains("hx-post=\"/test/create\""));

        let download = form_post_download_route(TestFileDownloadRoute).as_string();
        assert!(download.contains("action=\"/test/download\""));
        assert!(download.contains("hx-boost=\"false\""));
    }

    #[derive(Clone, Copy, Default)]
    struct TestQueryPostRoute;
    impl RouteTag for TestQueryPostRoute {
        const PATH: &'static str = "/test/create";
    }
    impl AppPanePost for TestQueryPostRoute {}
    impl RouteUrl for TestQueryPostRoute {
        fn path(self) -> String {
            Self::PATH.to_owned()
        }
        fn url(self) -> String {
            crate::http::trailing_slash(&self.path())
        }
    }

    #[test]
    fn form_hx_post_main_with_query() {
        let url = crate::http::RouteQueryBuilder::new(TestQueryPostRoute)
            .query("ClientID", 42)
            .query_opt("return", Some("client"))
            .build_with_query();
        let attrs = form_hx_post_main_url(&url).as_string();
        assert!(attrs.contains("hx-post=\"/test/create?ClientID=42"));
        assert!(attrs.contains("return=client"));
    }
}
