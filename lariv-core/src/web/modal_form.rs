//! Shared query params and typed wiring for create-modal GET/POST handlers.

use serde::Deserialize;

use crate::components::SwapKey;
use crate::http::{ModalGet, RouteQueryBuilder, RouteUrl};

/// Query string for create-modal forms (`name` form identity + optional parent table
/// refresh and FK field target).
///
/// `refresh` is the parent [`.data-table-container`](crate::components::data_table) element id
/// (a [`SwapKey`](crate::components::SwapKey) id). When set on successful create *without*
/// [`Self::target_input`], the modal is closed and that table is asked to re-fetch via
/// `HX-Trigger` on `document` (see [`crate::web::table_refresh_event`]).
///
/// `target_input` is the FK / M2M field name to fill instead of refreshing a picker table.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModalFormQuery {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub refresh: Option<String>,
    #[serde(default)]
    pub target_input: Option<String>,
}

impl ModalFormQuery {
    /// Form identity string (empty when absent).
    pub fn form_name(&self) -> String {
        self.name.clone().unwrap_or_default()
    }

    /// Parent table id to refresh after create (empty when absent).
    pub fn refresh_table(&self) -> String {
        self.refresh.clone().unwrap_or_default()
    }

    /// FK / M2M field name to fill after create (empty when absent).
    pub fn target_input(&self) -> String {
        self.target_input.clone().unwrap_or_default()
    }

    /// True when `refresh` matches the typed table key id.
    pub fn refreshes_table(&self, table: SwapKey) -> bool {
        self.refresh.as_deref() == Some(table.id())
    }
}

/// Create-modal wiring: swap key, form identity, and typed GET/POST routes.
///
/// Table refresh is chosen at the call site via [`modal_create_get_url`] /
/// [`modal_create_post_url_for_table`], not on the route — the same create modal may refresh
/// different tables (list vs FK picker).
#[derive(Debug, Clone, Copy)]
pub struct CreateModal<G, P> {
    pub key: SwapKey,
    pub form_name: &'static str,
    pub get: G,
    pub post: P,
}

/// Append `name` and `refresh=<table.id()>` to an existing create-modal GET URL.
///
/// Use when the GET URL already has extra query params (e.g. account `ParentID`) that
/// [`modal_create_get_for`] would drop.
pub fn modal_create_href_for_table(href: &str, form_name: &str, table: SwapKey) -> String {
    let mut out = href.to_string();
    append_query_if_absent(&mut out, "name", form_name);
    append_query_if_absent(&mut out, "refresh", table.id());
    out
}

/// Append `name` and `target_input` to an existing create-modal GET URL (FK picker plus).
///
/// Use when the GET URL already has extra query params (e.g. account `ParentID`) that
/// [`modal_create_get_for_picker`] would drop.
pub fn modal_create_href_for_picker(href: &str, form_name: &str, target_input: &str) -> String {
    let mut out = href.to_string();
    append_query_if_absent(&mut out, "name", form_name);
    append_query_if_absent(&mut out, "target_input", target_input);
    out
}

fn query_has_param(url: &str, key: &str) -> bool {
    url.contains(&format!("?{key}=")) || url.contains(&format!("&{key}="))
}

fn append_query_if_absent(url: &mut String, key: &str, value: &str) {
    if value.is_empty() || query_has_param(url, key) {
        return;
    }
    let sep = if url.contains('?') { '&' } else { '?' };
    url.push(sep);
    url.push_str(key);
    url.push('=');
    url.push_str(value);
}

/// Build a create-modal GET URL with `name` and typed parent table refresh.
pub fn modal_create_get_url(route: impl RouteUrl, form_name: &str, table: SwapKey) -> String {
    // Trailing slash matches other modal openers (`RouteUrl::url`) used by Lead/Contact create.
    modal_create_url(route, form_name, table.id(), "", true)
}

/// Build a create-modal GET URL for [`CreateModal`] `m` refreshing `table`.
pub fn modal_create_get_for<G, P>(m: CreateModal<G, P>, table: SwapKey) -> String
where
    G: ModalGet + RouteUrl + Copy,
    P: Copy,
{
    modal_create_get_url(m.get, m.form_name, table)
}

/// Build a create-modal GET URL for [`CreateModal`] `m` that fills FK field `target_input`.
pub fn modal_create_get_for_picker<G, P>(m: CreateModal<G, P>, target_input: &str) -> String
where
    G: ModalGet + RouteUrl + Copy,
    P: Copy,
{
    modal_create_url(m.get, m.form_name, "", target_input, true)
}

/// Build a create-modal POST action URL with optional `name` and `refresh` query params.
pub fn modal_create_post_url(route: impl RouteUrl, form_name: &str, refresh: &str) -> String {
    modal_create_post_query(route, form_name, refresh, "")
}

/// Build a create-modal POST action URL with optional `refresh` and `target_input`.
pub fn modal_create_post_query(
    route: impl RouteUrl,
    form_name: &str,
    refresh: &str,
    target_input: &str,
) -> String {
    modal_create_url(route, form_name, refresh, target_input, false)
}

/// Build a create-modal POST action URL refreshing typed `table`.
pub fn modal_create_post_url_for_table(
    route: impl RouteUrl,
    form_name: &str,
    table: SwapKey,
) -> String {
    modal_create_post_url(route, form_name, table.id())
}

/// Build a create-modal POST action URL for [`CreateModal`] `m` refreshing `table`.
pub fn modal_create_post_for<G, P>(m: CreateModal<G, P>, table: SwapKey) -> String
where
    P: RouteUrl + Copy,
{
    modal_create_post_url_for_table(m.post, m.form_name, table)
}

/// Build an edit-modal POST action URL with optional `name` form identity (no table refresh).
pub fn modal_edit_post_url(route: impl RouteUrl, form_name: &str) -> String {
    modal_create_url(route, form_name, "", "", false)
}

fn modal_create_url(
    route: impl RouteUrl,
    form_name: &str,
    refresh: &str,
    target_input: &str,
    trailing_slash: bool,
) -> String {
    let mut builder = RouteQueryBuilder::new(route);
    if !form_name.is_empty() {
        builder = builder.query("name", form_name);
    }
    if !refresh.is_empty() {
        builder = builder.query("refresh", refresh);
    }
    if !target_input.is_empty() {
        builder = builder.query("target_input", target_input);
    }
    if trailing_slash {
        builder.build()
    } else {
        builder.build_with_query()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::swap::SwapKey;
    use crate::http::{ModalGet, RouteTag, RouteUrl};

    const TEST_CREATE_MODAL: SwapKey = SwapKey("test-create-modal");
    const TEST_TABLE: SwapKey = SwapKey("test-table");

    #[derive(Clone, Copy, Default)]
    pub struct TestCreateGetRoute;
    impl RouteTag for TestCreateGetRoute {
        const PATH: &'static str = "/test/create";
    }
    impl RouteUrl for TestCreateGetRoute {
        fn path(self) -> String {
            Self::PATH.to_owned()
        }
        fn url(self) -> String {
            Self::PATH.to_owned()
        }
    }
    impl ModalGet for TestCreateGetRoute {}

    #[derive(Clone, Copy, Default)]
    pub struct TestCreatePostRoute;
    impl RouteTag for TestCreatePostRoute {
        const PATH: &'static str = "/test/create";
    }
    impl RouteUrl for TestCreatePostRoute {
        fn path(self) -> String {
            Self::PATH.to_owned()
        }
        fn url(self) -> String {
            Self::PATH.to_owned()
        }
    }

    const TEST_CREATE: CreateModal<TestCreateGetRoute, TestCreatePostRoute> = CreateModal {
        key: TEST_CREATE_MODAL,
        form_name: "p_test.CreateForm",
        get: TestCreateGetRoute,
        post: TestCreatePostRoute,
    };

    #[test]
    fn modal_create_urls_embed_table_refresh() {
        let get = modal_create_get_for(TEST_CREATE, TEST_TABLE);
        assert!(get.contains("/test/create/?"), "{get}");
        assert!(get.contains("name=p_test.CreateForm"), "{get}");
        assert!(get.contains("refresh=test-table"), "{get}");

        let post = modal_create_post_for(TEST_CREATE, TEST_TABLE);
        assert!(post.contains("/test/create?"), "{post}");
        assert!(post.contains("refresh=test-table"), "{post}");
        assert!(!post.contains("/test/create/?"), "{post}");
    }

    #[test]
    fn modal_create_href_preserves_existing_query() {
        let href = modal_create_href_for_table(
            "/test/create/?ParentID=9",
            "p_test.CreateForm",
            TEST_TABLE,
        );
        assert!(href.contains("ParentID=9"), "{href}");
        assert!(href.contains("name=p_test.CreateForm"), "{href}");
        assert!(href.contains("refresh=test-table"), "{href}");
        assert_eq!(href.matches('?').count(), 1, "{href}");
    }

    #[test]
    fn modal_create_get_for_picker_embeds_target_input() {
        let get = modal_create_get_for_picker(TEST_CREATE, "CustomerID");
        assert!(get.contains("/test/create/?"), "{get}");
        assert!(get.contains("name=p_test.CreateForm"), "{get}");
        assert!(get.contains("target_input=CustomerID"), "{get}");
        assert!(!get.contains("refresh="), "{get}");
    }

    #[test]
    fn modal_create_href_for_picker_preserves_existing_query() {
        let href = modal_create_href_for_picker(
            "/test/create/?ParentID=9",
            "p_test.CreateForm",
            "ParentID",
        );
        assert!(href.contains("ParentID=9"), "{href}");
        assert!(href.contains("name=p_test.CreateForm"), "{href}");
        assert!(href.contains("target_input=ParentID"), "{href}");
        assert!(!href.contains("refresh="), "{href}");
    }

    #[test]
    fn modal_create_post_query_embeds_target_input() {
        let post =
            modal_create_post_query(TestCreatePostRoute, "p_test.CreateForm", "", "CustomerID");
        assert!(post.contains("target_input=CustomerID"), "{post}");
        assert!(!post.contains("refresh="), "{post}");
    }

    #[test]
    fn modal_form_query_matches_table_key() {
        let q = ModalFormQuery {
            refresh: Some(TEST_TABLE.id().to_string()),
            ..Default::default()
        };
        assert!(q.refreshes_table(TEST_TABLE));
        assert!(!q.refreshes_table(TEST_CREATE_MODAL));
    }
}
