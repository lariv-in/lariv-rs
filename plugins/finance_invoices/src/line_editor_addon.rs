//! Extra columns on the invoice line editor, registered by other plugins.

use std::sync::{Mutex, OnceLock};

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

static ADDONS: OnceLock<Mutex<Vec<&'static dyn InvoiceLineEditorAddon>>> = OnceLock::new();

fn addon_list() -> &'static Mutex<Vec<&'static dyn InvoiceLineEditorAddon>> {
    ADDONS.get_or_init(|| Mutex::new(Vec::new()))
}

/// One plugin's column on draft and quotation line editors.
#[async_trait]
pub trait InvoiceLineEditorAddon: Send + Sync {
    fn id(&self) -> &'static str;

    fn header_html(&self) -> &'static str {
        ""
    }

    fn cell_html(&self) -> &'static str {
        ""
    }

    /// Appended inside `blankLine()`, after `remarks: ''`.
    fn blank_js(&self) -> &'static str {
        ""
    }

    /// Run while hydrating each loaded line.
    fn hydrate_js(&self) -> &'static str {
        ""
    }

    /// Extra keys inside the submit object, including a trailing comma when non-empty.
    fn submit_js(&self) -> &'static str {
        ""
    }

    /// Values merged into the line JSON for an existing row. Keys match the editor fields.
    async fn fields_for_line(
        &self,
        _db: &DatabaseConnection,
        _line_table: &str,
        _line_id: i64,
    ) -> Vec<(String, String)> {
        Vec::new()
    }
}

pub fn register_invoice_line_editor_addon(addon: &'static dyn InvoiceLineEditorAddon) {
    let mut list = addon_list()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if !list.iter().any(|existing| existing.id() == addon.id()) {
        list.push(addon);
    }
}

fn addons() -> Vec<&'static dyn InvoiceLineEditorAddon> {
    addon_list()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

pub fn line_editor_header_html() -> String {
    addons().iter().map(|addon| addon.header_html()).collect()
}

pub fn line_editor_cell_html() -> String {
    addons().iter().map(|addon| addon.cell_html()).collect()
}

pub fn line_editor_blank_js() -> String {
    addons().iter().map(|addon| addon.blank_js()).collect()
}

pub fn line_editor_hydrate_js() -> String {
    addons().iter().map(|addon| addon.hydrate_js()).collect()
}

pub fn line_editor_submit_js() -> String {
    addons().iter().map(|addon| addon.submit_js()).collect()
}

pub async fn line_editor_fields(
    db: &DatabaseConnection,
    line_table: &str,
    line_id: i64,
) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    for addon in addons() {
        fields.extend(addon.fields_for_line(db, line_table, line_id).await);
    }
    fields
}
