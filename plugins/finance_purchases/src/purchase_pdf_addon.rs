//! Extra Minijinja context for purchase PDFs, registered by other plugins.
//!
//! Deployments can attach related data such as sites without the core purchase
//! plugin knowing those entities.

use std::sync::{Mutex, OnceLock};

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use serde_json::{Map, Value};

static ADDONS: OnceLock<Mutex<Vec<&'static dyn PurchasePdfContextAddon>>> = OnceLock::new();

fn addon_list() -> &'static Mutex<Vec<&'static dyn PurchasePdfContextAddon>> {
    ADDONS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Extra JSON keys merged into the purchase PDF template context.
#[async_trait]
pub trait PurchasePdfContextAddon: Send + Sync {
    fn id(&self) -> &'static str;

    async fn extra_context(
        &self,
        db: &DatabaseConnection,
        draft_purchase_id: i64,
    ) -> Result<Value, String>;

    fn sample_extra_context(&self) -> Value;
}

/// Register a PDF context addon (idempotent by [`PurchasePdfContextAddon::id`]).
pub fn register_purchase_pdf_context_addon(addon: &'static dyn PurchasePdfContextAddon) {
    let mut list = addon_list().lock().unwrap_or_else(|e| e.into_inner());
    if !list.iter().any(|a| a.id() == addon.id()) {
        list.push(addon);
    }
}

fn addons() -> Vec<&'static dyn PurchasePdfContextAddon> {
    addon_list()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn merge_objects(into: &mut Map<String, Value>, extra: Value) {
    let Value::Object(extra) = extra else {
        return;
    };
    for (key, value) in extra {
        into.entry(key).or_insert(value);
    }
}

/// Merge live extras from all registered addons for `draft_purchase_id`.
pub async fn collect_purchase_pdf_extras(
    db: &DatabaseConnection,
    draft_purchase_id: i64,
) -> Result<Value, String> {
    let mut out = Map::new();
    for addon in addons() {
        merge_objects(&mut out, addon.extra_context(db, draft_purchase_id).await?);
    }
    Ok(Value::Object(out))
}

/// Merge sample extras from all registered addons (settings preview).
pub fn collect_purchase_pdf_sample_extras() -> Value {
    let mut out = Map::new();
    for addon in addons() {
        merge_objects(&mut out, addon.sample_extra_context());
    }
    Value::Object(out)
}
