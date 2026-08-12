//! Shared query params and typed wiring for create-modal GET/POST handlers.

use serde::Deserialize;

use crate::components::SwapKey;
use crate::http::{ModalGet, RouteQueryBuilder, RouteUrl};

/// Query string for create-modal forms (`name` form identity + optional parent table refresh).
///
/// `refresh` is the parent [`.data-table-container`](crate::components::data_table) element id
/// (a [`SwapKey`](crate::components::SwapKey) id). When set on successful create, the modal is
/// closed and that table is asked to re-fetch via `HX-Trigger` targeted at `#refresh`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModalFormQuery {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub refresh: Option<String>,
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

    /// True when `refresh` matches the typed table key id.
    pub fn refreshes_table<T: SwapKey>(&self) -> bool {
        self.refresh.as_deref() == Some(T::ID)
    }
}

/// Create-modal swap key with typed GET/POST routes and form identity.
///
/// Table refresh is chosen at the call site via [`modal_create_get_url`] /
/// [`modal_create_post_url_for_table`], not on the route — the same create modal may refresh
/// different tables (list vs FK picker).
pub trait CreateModal: SwapKey {
    type Get: ModalGet + RouteUrl + Copy + Default;
    type Post: RouteUrl + Copy + Default;
    const FORM_NAME: &'static str;
}

/// Build a create-modal GET URL with `name` and typed parent table refresh.
pub fn modal_create_get_url<T: SwapKey>(route: impl RouteUrl, form_name: &str) -> String {
    modal_create_url(route, form_name, T::ID)
}

/// Build a create-modal GET URL for [`CreateModal`] `M` refreshing table `T`.
pub fn modal_create_get_for<M: CreateModal, T: SwapKey>() -> String {
    modal_create_get_url::<T>(M::Get::default(), M::FORM_NAME)
}

/// Build a create-modal POST action URL with optional `name` and `refresh` query params.
pub fn modal_create_post_url(route: impl RouteUrl, form_name: &str, refresh: &str) -> String {
    modal_create_url(route, form_name, refresh)
}

/// Build a create-modal POST action URL refreshing typed table `T`.
pub fn modal_create_post_url_for_table<T: SwapKey>(
    route: impl RouteUrl,
    form_name: &str,
) -> String {
    modal_create_post_url(route, form_name, T::ID)
}

/// Build a create-modal POST action URL for [`CreateModal`] `M` refreshing table `T`.
pub fn modal_create_post_for<M: CreateModal, T: SwapKey>() -> String {
    modal_create_post_url_for_table::<T>(M::Post::default(), M::FORM_NAME)
}

fn modal_create_url(route: impl RouteUrl, form_name: &str, refresh: &str) -> String {
    let mut builder = RouteQueryBuilder::new(route);
    if !form_name.is_empty() {
        builder = builder.query("name", form_name);
    }
    if !refresh.is_empty() {
        builder = builder.query("refresh", refresh);
    }
    builder.build_with_query()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::swap::SwapKey;
    use crate::http::{ModalGet, RouteTag, RouteUrl};
    use crate::swap_key;

    swap_key!(TestCreateModalKey, "test-create-modal");
    swap_key!(TestTableKey, "test-table");

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

    impl CreateModal for TestCreateModalKey {
        type Get = TestCreateGetRoute;
        type Post = TestCreatePostRoute;
        const FORM_NAME: &'static str = "p_test.CreateForm";
    }

    #[test]
    fn modal_create_urls_embed_table_refresh() {
        let get = modal_create_get_for::<TestCreateModalKey, TestTableKey>();
        assert!(get.contains("name=p_test.CreateForm"), "{get}");
        assert!(get.contains("refresh=test-table"), "{get}");

        let post = modal_create_post_for::<TestCreateModalKey, TestTableKey>();
        assert!(post.contains("refresh=test-table"), "{post}");
    }

    #[test]
    fn modal_form_query_matches_table_key() {
        let q = ModalFormQuery {
            refresh: Some(TestTableKey::ID.to_string()),
            ..Default::default()
        };
        assert!(q.refreshes_table::<TestTableKey>());
        assert!(!q.refreshes_table::<TestCreateModalKey>());
    }
}
