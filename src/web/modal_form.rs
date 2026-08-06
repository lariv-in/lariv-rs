//! Shared query params for create-modal GET/POST handlers.

use serde::Deserialize;

use crate::http::{RouteQueryBuilder, RouteUrl};

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
}

/// Build a create-modal POST action URL with optional `name` and `refresh` query params.
pub fn modal_create_post_url(route: impl RouteUrl, form_name: &str, refresh: &str) -> String {
    let mut builder = RouteQueryBuilder::new(route);
    if !form_name.is_empty() {
        builder = builder.query("name", form_name);
    }
    if !refresh.is_empty() {
        builder = builder.query("refresh", refresh);
    }
    builder.build_with_query()
}
