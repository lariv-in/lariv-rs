//! Extra columns on the invoice hub list table, registered by other plugins.
//!
//! Deployments can attach related data such as sites without the core invoice
//! plugin knowing those entities.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

static ADDONS: OnceLock<Mutex<Vec<&'static dyn InvoiceHubTableAddon>>> = OnceLock::new();

fn addon_list() -> &'static Mutex<Vec<&'static dyn InvoiceHubTableAddon>> {
    ADDONS.get_or_init(|| Mutex::new(Vec::new()))
}

/// One plugin's extra column on the invoice hub table.
#[async_trait]
pub trait InvoiceHubTableAddon: Send + Sync {
    fn id(&self) -> &'static str;

    /// Stable column key for client-side column visibility.
    fn column_key(&self) -> &'static str;

    /// Header label shown in the table.
    fn column_label(&self) -> &'static str;

    /// Cell text keyed by draft invoice id. Missing keys render empty.
    async fn cell_values(
        &self,
        db: &DatabaseConnection,
        draft_invoice_ids: &[i64],
    ) -> HashMap<i64, String>;
}

/// Register a hub-table column addon (idempotent by [`InvoiceHubTableAddon::id`]).
pub fn register_invoice_hub_table_addon(addon: &'static dyn InvoiceHubTableAddon) {
    let mut list = addon_list().lock().unwrap_or_else(|e| e.into_inner());
    if !list.iter().any(|a| a.id() == addon.id()) {
        list.push(addon);
    }
}

fn addons() -> Vec<&'static dyn InvoiceHubTableAddon> {
    addon_list()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Header metadata for one registered extra column.
#[derive(Clone, Debug)]
pub struct InvoiceHubExtraColumn {
    pub key: &'static str,
    pub label: &'static str,
}

/// Fill [`super::templates::InvoiceRow::extra_cells`] for all registered addons.
///
/// Returns column headers in registration order (same order as each row's
/// `extra_cells`).
pub async fn enrich_hub_rows(
    db: &DatabaseConnection,
    rows: &mut [super::templates::InvoiceRow],
) -> Vec<InvoiceHubExtraColumn> {
    let registered = addons();
    let columns: Vec<InvoiceHubExtraColumn> = registered
        .iter()
        .map(|addon| InvoiceHubExtraColumn {
            key: addon.column_key(),
            label: addon.column_label(),
        })
        .collect();

    if columns.is_empty() {
        for row in rows.iter_mut() {
            row.extra_cells.clear();
        }
        return columns;
    }

    let mut draft_ids: Vec<i64> = rows.iter().filter_map(|r| r.draft_invoice_id).collect();
    draft_ids.sort_unstable();
    draft_ids.dedup();

    let mut maps = Vec::with_capacity(registered.len());
    for addon in &registered {
        maps.push(addon.cell_values(db, &draft_ids).await);
    }

    for row in rows.iter_mut() {
        row.extra_cells = maps
            .iter()
            .map(|m| {
                row.draft_invoice_id
                    .and_then(|id| m.get(&id).cloned())
                    .unwrap_or_default()
            })
            .collect();
    }

    columns
}
