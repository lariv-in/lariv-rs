//! GST supply on invoice script lines.

use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde_json::Value;

use lariv_core::rune_env::{RuneEnvCapability, RuneEnvRegistrar};
use lariv_plugin_finance_invoices::script_addon::{
    InvoiceScriptAddon, InvoiceScriptRecord, register_invoice_script_addon,
};

use crate::details::{GstTable, name_is_gst_component, supply_for_line};

pub fn register() {
    register_invoice_script_addon(&GstInvoiceScript);
}

#[derive(Clone, Copy, Default)]
pub struct RuneHook;

impl RuneEnvRegistrar for RuneHook {
    fn register_rune_env(self, rune_env: &mut RuneEnvCapability) {
        register();
        lariv_plugin_finance_invoices::rune_env::mount(rune_env);
    }
}

struct GstInvoiceScript;

#[async_trait]
impl InvoiceScriptAddon for GstInvoiceScript {
    fn id(&self) -> &'static str {
        "finance_gst_export"
    }

    fn doc_suffix(&self) -> &'static str {
        " Each line may set gst_supply (taxable, nil, exempt, nongst)."
    }

    async fn after_write(
        &self,
        db: &DatabaseConnection,
        record: InvoiceScriptRecord,
        id: i64,
        args: &Value,
    ) -> Result<(), String> {
        let Some(table) = gst_table(record) else {
            return Ok(());
        };
        let Some(lines) = args.get("lines").and_then(Value::as_array) else {
            return Ok(());
        };
        sync_line_supply(db, table, id, lines).await
    }
}

fn gst_table(record: InvoiceScriptRecord) -> Option<GstTable> {
    match record {
        InvoiceScriptRecord::Draft => Some(GstTable::Draft),
        InvoiceScriptRecord::Quotation => Some(GstTable::Quotation),
        InvoiceScriptRecord::Posted => Some(GstTable::Posted),
        InvoiceScriptRecord::Cancelled => Some(GstTable::Cancelled),
    }
}

async fn sync_line_supply(
    db: &DatabaseConnection,
    table: GstTable,
    parent: i64,
    lines: &[Value],
) -> Result<(), String> {
    let (line_table, parent_col) = crate::details::line_table(table);
    let mut supplies = Vec::new();
    for line in lines {
        let product_id = line.get("product_id").and_then(Value::as_i64).unwrap_or(0);
        if product_id <= 0 {
            continue;
        }
        let raw = line
            .get("gst_supply")
            .and_then(Value::as_str)
            .unwrap_or("taxable");
        let tax_ids = line
            .get("tax_ids")
            .and_then(Value::as_array)
            .map(|ids| {
                ids.iter()
                    .filter_map(Value::as_i64)
                    .filter(|id| *id > 0)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let has_gst = line_has_gst_tax(db, &tax_ids).await?;
        supplies.push(supply_for_line(raw, has_gst));
    }
    if supplies.is_empty() {
        return Ok(());
    }
    let ids = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("SELECT id FROM {line_table} WHERE {parent_col} = $1 ORDER BY id"),
            [parent.into()],
        ))
        .await
        .map_err(|error| error.to_string())?;
    if ids.len() != supplies.len() {
        return Ok(());
    }
    for (row, supply) in ids.iter().zip(supplies) {
        let id: i64 = row.try_get("", "id").map_err(|error| error.to_string())?;
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("UPDATE {line_table} SET gst_supply = $1 WHERE id = $2"),
            [supply.into(), id.into()],
        ))
        .await
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn line_has_gst_tax(db: &DatabaseConnection, tax_ids: &[i64]) -> Result<bool, String> {
    if tax_ids.is_empty() {
        return Ok(false);
    }
    let list = tax_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("SELECT name, tax_type::text AS tax_type FROM taxes WHERE id IN ({list})"),
        ))
        .await
        .map_err(|error| error.to_string())?;
    Ok(rows.iter().any(|row| {
        let levied = row.try_get::<String>("", "tax_type").unwrap_or_default() == "levied";
        let name = row.try_get::<String>("", "name").unwrap_or_default();
        levied && name_is_gst_component(&name)
    }))
}
