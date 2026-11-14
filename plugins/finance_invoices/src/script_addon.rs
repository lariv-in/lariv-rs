//! Extra script fields on invoice create, update, and get, registered by other plugins.

use std::sync::{Mutex, OnceLock};

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use serde_json::Value;

static ADDONS: OnceLock<Mutex<Vec<&'static dyn InvoiceScriptAddon>>> = OnceLock::new();

fn addon_list() -> &'static Mutex<Vec<&'static dyn InvoiceScriptAddon>> {
    ADDONS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Which invoice record a script call is reading or writing.
#[derive(Clone, Copy)]
pub enum InvoiceScriptRecord {
    Draft,
    Quotation,
    Posted,
    Cancelled,
}

/// One plugin's fields on the invoice script API.
#[async_trait]
pub trait InvoiceScriptAddon: Send + Sync {
    fn id(&self) -> &'static str;

    /// Appended to invoice script docs when this plugin is installed.
    fn doc_suffix(&self) -> &'static str {
        ""
    }

    async fn after_write(
        &self,
        _db: &DatabaseConnection,
        _record: InvoiceScriptRecord,
        _id: i64,
        _args: &Value,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn enrich(
        &self,
        _db: &DatabaseConnection,
        _record: InvoiceScriptRecord,
        _id: i64,
    ) -> Result<Vec<(String, Value)>, String> {
        Ok(Vec::new())
    }
}

pub fn register_invoice_script_addon(addon: &'static dyn InvoiceScriptAddon) {
    let mut list = addon_list()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if !list.iter().any(|existing| existing.id() == addon.id()) {
        list.push(addon);
    }
}

fn addons() -> Vec<&'static dyn InvoiceScriptAddon> {
    addon_list()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

pub fn doc_suffix() -> String {
    addons().iter().map(|addon| addon.doc_suffix()).collect()
}

pub async fn after_invoice_script_write(
    db: &DatabaseConnection,
    record: InvoiceScriptRecord,
    id: i64,
    args: &Value,
) -> Result<(), String> {
    for addon in addons() {
        addon.after_write(db, record, id, args).await?;
    }
    Ok(())
}

pub async fn enrich_invoice_script(
    db: &DatabaseConnection,
    record: InvoiceScriptRecord,
    id: i64,
) -> Result<Vec<(String, Value)>, String> {
    let mut fields = Vec::new();
    for addon in addons() {
        fields.extend(addon.enrich(db, record, id).await?);
    }
    Ok(fields)
}
