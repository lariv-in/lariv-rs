//! Extra filters on the purchase hub list, registered by other plugins.
//!
//! Deployments can filter on related data such as sites without the core
//! purchase plugin knowing those entities. Extra inputs are appended after the
//! core filter fields. SQL is applied to every hub tab through the draft
//! purchase id of the row.

use std::sync::{Mutex, OnceLock};

use async_trait::async_trait;
use maud::{Markup, html};
use sea_orm::sea_query::SimpleExpr;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, Select};

static ADDONS: OnceLock<Mutex<Vec<&'static dyn PurchaseHubFilterAddon>>> = OnceLock::new();

fn addon_list() -> &'static Mutex<Vec<&'static dyn PurchaseHubFilterAddon>> {
    ADDONS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Query string for one purchase hub request (`path?query` or the raw query).
#[derive(Clone, Copy, Debug)]
pub struct HubQueryParams<'a> {
    query: &'a str,
}

impl<'a> HubQueryParams<'a> {
    pub fn new(query: &'a str) -> Self {
        Self { query }
    }

    pub fn from_path_and_query(path_and_query: &'a str) -> Self {
        let query = path_and_query.split_once('?').map(|(_, q)| q).unwrap_or("");
        Self { query }
    }

    /// First non-empty value for `key`, percent-decoded and trimmed.
    pub fn get(&self, key: &str) -> Option<String> {
        form_urlencoded::parse(self.query.as_bytes())
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }
}

/// One plugin's extra filter on the purchase hub table.
#[async_trait]
pub trait PurchaseHubFilterAddon: Send + Sync {
    fn id(&self) -> &'static str;

    /// Markup appended after the core hub filter fields.
    async fn render_inputs(&self, db: &DatabaseConnection, params: &HubQueryParams<'_>) -> Markup;

    /// Predicate restricting rows, or `None` when this addon is inactive.
    ///
    /// `draft_purchase_id_sql` is a trusted SQL expression for the row's draft
    /// purchase id:
    /// - `draft_purchases.id`
    /// - `posted_purchases.draft_purchase_id`
    /// - [`draft_purchase_id_sql_via_posted`] for cancelled, paid, and partial rows
    fn sql_predicate(
        &self,
        params: &HubQueryParams<'_>,
        draft_purchase_id_sql: &str,
    ) -> Option<SimpleExpr>;
}

/// Register a hub-filter addon (idempotent by [`PurchaseHubFilterAddon::id`]).
pub fn register_purchase_hub_filter_addon(addon: &'static dyn PurchaseHubFilterAddon) {
    let mut list = addon_list().lock().unwrap_or_else(|e| e.into_inner());
    if !list.iter().any(|a| a.id() == addon.id()) {
        list.push(addon);
    }
}

fn addons() -> Vec<&'static dyn PurchaseHubFilterAddon> {
    addon_list()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Draft purchase id on a table that stores `posted_purchase_id`.
pub fn draft_purchase_id_sql_via_posted(table: &str) -> String {
    assert!(
        !table.is_empty() && table.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "posted child table name must be a sql identifier"
    );
    format!(
        "(SELECT pi.draft_purchase_id FROM posted_purchases pi WHERE pi.id = {table}.posted_purchase_id)"
    )
}

/// Markup for every registered extra filter, in registration order.
pub async fn render_extra_filter_inputs(
    db: &DatabaseConnection,
    params: &HubQueryParams<'_>,
) -> Markup {
    let mut out = Markup::default();
    for addon in addons() {
        let extra = addon.render_inputs(db, params).await;
        out = html! { (out) (extra) };
    }
    out
}

/// AND every registered addon's predicate onto `query`.
pub fn apply_extra_sql_filters<E: EntityTrait>(
    mut query: Select<E>,
    params: &HubQueryParams<'_>,
    draft_purchase_id_sql: &str,
) -> Select<E> {
    for addon in addons() {
        if let Some(expr) = addon.sql_predicate(params, draft_purchase_id_sql) {
            query = query.filter(expr);
        }
    }
    query
}
