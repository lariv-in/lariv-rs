//! Compile-time HTMX swap region keys.
//!
//! Every swappable DOM region is a [`SwapKey`] const. Call sites use the value for
//! `id`, `hx-target`, and out-of-band swaps — never free-form selector strings.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::{HtmlAttrs, escape_attr};
use crate::http::{
    AppPanePost, BoostPost, FileDownloadPost, FkSelectGet, FragmentGet, FragmentPost, RouteUrl,
};

/// HTMX target for controls that replace a whole data table (picker filters, drill-in).
///
/// Prefer this over [`SwapKey::selector`] when two tables of the same key can appear
/// on one page — `closest` resolves to the instance that contains the control.
pub const HX_TARGET_CLOSEST_TABLE: &str = "closest .data-table-container";

/// Sort and pagination live inside [`.data-table-body`](crate::components::DATA_TABLE_BODY_CLASS).
///
/// Targeting that region leaves the title toolbar in place.
pub const HX_TARGET_CLOSEST_TABLE_BODY: &str = "closest .data-table-body";

/// Filter forms sit in the title toolbar, before the row region in document order.
pub const HX_TARGET_NEXT_TABLE_BODY: &str = "next .data-table-body";

/// `hx-select` for a row-region swap. Handlers that still return the full table
/// are reduced to the body before the swap.
pub const HX_SELECT_TABLE_BODY: &str = ".data-table-body";

/// Separator between a [`SwapKey::id`] and a per-instance suffix on data tables.
pub const TABLE_INSTANCE_SEP: &str = "--";

/// A named DOM region that HTMX can target or swap out-of-band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SwapKey(pub &'static str);

impl SwapKey {
    /// Element `id` attribute value (without `#`).
    pub const fn id(self) -> &'static str {
        self.0
    }

    /// CSS selector for HTMX (`#` + [`id`](Self::id)).
    pub fn selector(self) -> String {
        format!("#{}", self.0)
    }

    /// True for this region's id or a table instance id (`{id}--…`).
    pub fn matches_id(self, id: &str) -> bool {
        id == self.0
            || id
                .strip_prefix(self.0)
                .is_some_and(|rest| rest.starts_with(TABLE_INSTANCE_SEP))
    }
}

/// Page pane under persistent topbar (sidebar + main). Prefer this for in-app
/// navigation and form re-renders instead of full-document morphs.
pub const APP_LAYOUT: SwapKey = SwapKey("app-layout");

/// Inner content column (`<main>`) inside the app layout scaffold.
pub const MAIN_CONTENT: SwapKey = SwapKey("main-content");

/// `id="app-layout" hx-history-elt` — HTMX 4 swaps this on back/forward navigation.
///
/// When the request arrived from the apps dashboard, also emit `hx-vals` so in-app
/// HTMX navigations keep `from=dashboard`.
pub fn app_layout_history_attrs() -> String {
    if crate::components::nav_origin::from_dashboard() {
        format!(
            r#"id="{}" hx-history-elt hx-vals:inherited='{{"from":"dashboard"}}'"#,
            APP_LAYOUT.id()
        )
    } else {
        format!(r#"id="{}" hx-history-elt"#, APP_LAYOUT.id())
    }
}

/// In-app navigation that replaces `#app-layout` with an explicit URL.
///
/// Prefer [`hx_nav_app_layout`] for typed app-pane GET routes.
pub fn hx_nav_app_layout_for_url(url: &str) -> HtmlAttrs {
    let url = crate::components::nav_origin::with_nav_origin(url);
    HtmlAttrs::new()
        .set("hx-get", &url)
        .set("hx-target", APP_LAYOUT.selector())
        .set("hx-select", APP_LAYOUT.selector())
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
        .set("hx-target", APP_LAYOUT.selector())
        .set("hx-select", APP_LAYOUT.selector())
        .set("hx-swap", "outerHTML")
        .set("hx-push-url", "true")
        .set("action", action)
}

/// Sidebar menu-style navigation into [`MAIN_CONTENT`] only.
pub fn nav_content_attrs(url: &str) -> HtmlAttrs {
    let url = crate::components::nav_origin::with_nav_origin(url);
    HtmlAttrs::new()
        .set("hx-get", &url)
        .set("hx-target", MAIN_CONTENT.selector())
        .set("hx-select", MAIN_CONTENT.selector())
        .set("hx-swap", "outerHTML")
        .set("hx-push-url", "true")
}

/// `id="…"` for a region root element.
pub fn region_attrs(key: SwapKey) -> HtmlAttrs {
    HtmlAttrs::new().set("id", key.id())
}

/// Declarative HTMX targeting attrs for a keyed region (`hx-target` + outerHTML).
///
/// Prefer [`hx_target_swap`] with `"outerMorph"` for same-structure fragments (tables).
pub fn hx_target(key: SwapKey) -> HtmlAttrs {
    hx_target_swap(key, "outerHTML")
}

/// Like [`hx_target`] with an explicit swap strategy.
pub fn hx_target_swap(key: SwapKey, swap: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("hx-target", key.selector())
        .set("hx-swap", swap)
}

/// Attrs for an out-of-band fragment rooted at `key` (`id` + `hx-swap-oob`).
pub fn oob_attrs(key: SwapKey) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("id", key.id())
        .set("hx-swap-oob", "true")
}

/// Attrs for OOB with an explicit swap strategy (e.g. `"outerHTML"`, `"innerHTML"`).
pub fn oob_attrs_swap(key: SwapKey, swap: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("id", key.id())
        .set("hx-swap-oob", swap)
}

/// Markup that deletes the keyed element via HTMX OOB (`hx-swap-oob="delete"`).
pub fn oob_delete(key: SwapKey) -> Markup {
    if key.id().is_empty() {
        return Markup::default();
    }
    html! {
        (PreEscaped(format!(
            r#"<div id="{}" hx-swap-oob="delete"></div>"#,
            escape_attr(key.id())
        )))
    }
}

/// Wrap `inner` as an OOB swap for a region (default OOB style).
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
pub fn form_hx_post_for_url(key: SwapKey, url: &str) -> HtmlAttrs {
    form_hx_post_selector(url, &key.selector())
}

/// Multipart HTMX POST into a typed region (file uploads).
///
/// Sets `hx-encoding` so HTMX 4 keeps `FormData` instead of converting to
/// urlencoded (which silently drops file parts).
pub fn form_hx_post_multipart_url(key: SwapKey, url: &str) -> HtmlAttrs {
    form_hx_post_for_url(key, url).set("hx-encoding", "multipart/form-data")
}

/// Typed POST form targeting a fragment route value.
pub fn form_hx_post_route<R: RouteUrl + FragmentPost>(route: R) -> HtmlAttrs {
    form_hx_post_for_url(R::TARGET, &route.path())
}

/// Typed POST form with query string from a [`RouteQueryBuilder`](crate::http::route_tag::RouteQueryBuilder) result URL.
pub fn form_hx_post_url(key: SwapKey, url: &str) -> HtmlAttrs {
    form_hx_post_for_url(key, url)
}

/// HTMX attrs for a form that POSTs into a selector (when the key is dynamic).
///
/// Uses `outerHTML`, not `outerMorph`. Error re-renders replace the form so Alpine
/// re-inits from the server `x-data` and rebuilds `x-for` rows. Morph would keep the
/// old data stack and delete those generated rows, which exist only in the live DOM.
pub fn form_hx_post_selector(action: &str, target: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("method", "POST")
        .set("hx-post", action)
        .set("hx-target", target)
        .set("hx-swap", "outerHTML")
        .set("hx-push-url", "false")
}

/// POST into [`APP_LAYOUT`] with an explicit URL (e.g. redirect-only POST routes).
///
/// Prefer [`form_hx_post_main`] or [`form_hx_post_main_url`].
pub fn form_hx_post_main_for_url(url: &str) -> HtmlAttrs {
    form_hx_post_for_url(APP_LAYOUT, url)
        .set("hx-select", APP_LAYOUT.selector())
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

/// Classic multipart POST (no HTMX). Use when file inputs must reach the server —
/// HTMX can convert the body to urlencoded and drop file parts.
pub fn form_post_multipart(action: &str) -> HtmlAttrs {
    form_post_download(action)
}

/// Typed plain POST form for a file-download route value.
pub fn form_post_download_route<R: RouteUrl + FileDownloadPost>(route: R) -> HtmlAttrs {
    form_post_download(&route.path())
}

/// HTMX attrs for a GET form (filters) with an explicit URL.
///
/// Prefer [`form_hx_get_route`] or [`form_hx_get_url`].
///
/// Targets [`HX_TARGET_NEXT_TABLE_BODY`] so the title toolbar stays put and two
/// tables of the same [`SwapKey`] do not steal each other's filter swaps.
pub fn form_hx_get_for_url(url: &str) -> HtmlAttrs {
    let url = crate::components::nav_origin::with_nav_origin(url);
    HtmlAttrs::new()
        .set("method", "GET")
        .set("hx-get", &url)
        .set("hx-target", HX_TARGET_NEXT_TABLE_BODY)
        .set("hx-select", HX_SELECT_TABLE_BODY)
        .set("hx-swap", "outerMorph")
        .set("hx-push-url", "true")
}

/// Typed GET filter form targeting a fragment route value.
pub fn form_hx_get_route<R: RouteUrl + FragmentGet>(route: R) -> HtmlAttrs {
    let _ = R::TARGET;
    form_hx_get_for_url(&route.path())
}

/// GET filter form inside an FK picker modal.
///
/// Targets [`HX_TARGET_CLOSEST_TABLE`] (overriding `dialog[hx-target=this]` inheritance)
/// so pagination and filters swap the picker table instance, not a shared modal id.
/// Uses `outerHTML` so Alpine `@click` row attrs are not morphed in place.
pub fn form_hx_get_picker_route<R: RouteUrl + FkSelectGet>(route: R) -> HtmlAttrs {
    let _ = (R::TABLE, R::MODAL);
    HtmlAttrs::new()
        .set("method", "GET")
        .set("hx-get", route.path())
        .set("hx-target", HX_TARGET_CLOSEST_TABLE)
        .set("hx-swap", "outerHTML")
        .set("hx-push-url", "false")
}

/// GET filter form with an explicit URL.
pub fn form_hx_get_url(key: SwapKey, url: &str) -> HtmlAttrs {
    let _ = key;
    form_hx_get_for_url(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{
        AppPanePost, FileDownloadPost, FkSelectGet, FragmentGet, RouteTag, RouteUrl,
    };

    const TEST_TABLE: SwapKey = SwapKey("test-table");
    const TEST_PICKER_TABLE: SwapKey = SwapKey("test-picker-table");
    const TEST_PICKER_MODAL: SwapKey = SwapKey("test-picker-modal");

    #[derive(Clone, Copy, Default)]
    struct TestAppPanePostRoute;
    impl RouteTag for TestAppPanePostRoute {
        const PATH: &'static str = "/test/create";
    }
    impl AppPanePost for TestAppPanePostRoute {}
    impl FragmentGet for TestAppPanePostRoute {
        const TARGET: SwapKey = TEST_PICKER_TABLE;
    }
    impl FkSelectGet for TestAppPanePostRoute {
        const TABLE: SwapKey = TEST_PICKER_TABLE;
        const MODAL: SwapKey = TEST_PICKER_MODAL;
    }
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
        assert_eq!(TEST_TABLE.id(), "test-table");
        assert_eq!(TEST_TABLE.selector(), "#test-table");
        assert_eq!(MAIN_CONTENT.selector(), "#main-content");
        assert_eq!(APP_LAYOUT.selector(), "#app-layout");
        assert!(TEST_TABLE.matches_id("test-table"));
        assert!(TEST_TABLE.matches_id("test-table--abc123"));
        assert!(!TEST_TABLE.matches_id("test-table-select"));
        assert!(!TEST_TABLE.matches_id("test-picker-table"));
    }

    #[test]
    fn region_and_target_attrs() {
        let r = region_attrs(TEST_TABLE).as_string();
        assert!(r.contains(r#"id="test-table""#));

        let t = hx_target(TEST_TABLE).as_string();
        assert!(t.contains("hx-target=\"#test-table\""));
        assert!(!t.contains("hx-select"));
        assert!(t.contains("hx-swap=\"outerHTML\""));

        let o = oob_attrs(TEST_TABLE).as_string();
        assert!(o.contains("hx-swap-oob=\"true\""));
        assert!(o.contains("id=\"test-table\""));
    }

    #[test]
    fn oob_delete_emits_marker() {
        let html = oob_delete(TEST_TABLE).into_string();
        assert!(html.contains("id=\"test-table\""));
        assert!(html.contains("hx-swap-oob=\"delete\""));
    }

    #[test]
    fn app_layout_history_attrs_includes_elt() {
        let attrs = super::app_layout_history_attrs();
        assert!(attrs.contains("app-layout"));
        assert!(attrs.contains("hx-history-elt"));
        assert!(!attrs.contains("hx-vals"));
    }

    #[tokio::test]
    async fn app_layout_history_attrs_include_from_dashboard_vals() {
        crate::components::nav_origin::scope_from_dashboard(true, async {
            let attrs = super::app_layout_history_attrs();
            assert!(attrs.contains(r#"hx-vals:inherited='{"from":"dashboard"}'"#));
        })
        .await;
    }

    #[test]
    fn form_hx_helpers() {
        let post = form_hx_post_for_url(TEST_TABLE, "/users/create/").as_string();
        assert!(post.contains("hx-post=\"/users/create/\""));
        assert!(post.contains("hx-target=\"#test-table\""));
        assert!(post.contains("hx-swap=\"outerHTML\""));
        assert!(!post.contains("outerMorph"));
        assert!(!post.contains("hx-select"));

        let get = form_hx_get_for_url("/users/").as_string();
        assert!(get.contains("hx-get=\"/users/\""));
        assert!(get.contains("hx-push-url=\"true\""));
        assert!(get.contains(&format!("hx-target=\"{HX_TARGET_NEXT_TABLE_BODY}\"")));
        assert!(get.contains(&format!("hx-select=\"{HX_SELECT_TABLE_BODY}\"")));

        let picker = form_hx_get_picker_route(TestAppPanePostRoute).as_string();
        assert!(picker.contains("hx-get=\"/test/create\""));
        assert!(picker.contains(&format!("hx-target=\"{HX_TARGET_CLOSEST_TABLE}\"")));
        assert!(picker.contains("hx-swap=\"outerHTML\""));
        assert!(picker.contains("hx-push-url=\"false\""));

        let main = form_hx_post_main_for_url("/users/login").as_string();
        assert!(main.contains("hx-select=\"#app-layout\""));
        assert!(main.contains("hx-swap=\"outerHTML\""));
        assert!(main.contains("hx-push-url=\"true\""));
        assert!(!main.contains("outerMorph"));

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
