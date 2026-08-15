//! Extra Minijinja context for invoice PDFs, registered by other plugins.
//!
//! Deployments can attach related data such as sites without the core invoice
//! plugin knowing those entities.

use std::sync::{Mutex, OnceLock};

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use serde_json::{Map, Value};

static ADDONS: OnceLock<Mutex<Vec<&'static dyn InvoicePdfContextAddon>>> = OnceLock::new();

fn addon_list() -> &'static Mutex<Vec<&'static dyn InvoicePdfContextAddon>> {
    ADDONS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Extra JSON keys merged into the invoice PDF template context.
#[async_trait]
pub trait InvoicePdfContextAddon: Send + Sync {
    fn id(&self) -> &'static str;

    async fn extra_context(
        &self,
        db: &DatabaseConnection,
        draft_invoice_id: i64,
    ) -> Result<Value, String>;

    fn sample_extra_context(&self) -> Value;
}

/// Register a PDF context addon (idempotent by [`InvoicePdfContextAddon::id`]).
pub fn register_invoice_pdf_context_addon(addon: &'static dyn InvoicePdfContextAddon) {
    let mut list = addon_list().lock().unwrap_or_else(|e| e.into_inner());
    if !list.iter().any(|a| a.id() == addon.id()) {
        list.push(addon);
    }
}

fn addons() -> Vec<&'static dyn InvoicePdfContextAddon> {
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

/// Merge live extras from all registered addons for `draft_invoice_id`.
pub async fn collect_invoice_pdf_extras(
    db: &DatabaseConnection,
    draft_invoice_id: i64,
) -> Result<Value, String> {
    let mut out = Map::new();
    for addon in addons() {
        merge_objects(&mut out, addon.extra_context(db, draft_invoice_id).await?);
    }
    Ok(Value::Object(out))
}

/// Merge sample extras from all registered addons (settings preview).
pub fn collect_invoice_pdf_sample_extras() -> Value {
    let mut out = Map::new();
    for addon in addons() {
        merge_objects(&mut out, addon.sample_extra_context());
    }
    Value::Object(out)
}
