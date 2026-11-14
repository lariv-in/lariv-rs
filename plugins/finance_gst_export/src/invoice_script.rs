//! GST fields on the invoice script API.

use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde_json::{Value, json};

use lariv_core::rune_env::{RuneEnvCapability, RuneEnvRegistrar};
use lariv_plugin_finance_invoices::script_addon::{
    InvoiceScriptAddon, InvoiceScriptRecord, register_invoice_script_addon,
};

use crate::details::{
    GstFormValues, GstTable, gst_from_form, load_gst, name_is_gst_component, supply_for_line,
    upsert_gst,
};

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
        " GST return fields: gst_section, gst_inv_typ, reverse_charge, etin, supplier_gstin, exp_typ, shipping_bill_number, shipping_bill_date, port_code, amendment_of_number, amendment_of_date. Each line may set gst_supply (taxable, nil, exempt, nongst)."
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
        let Some(obj) = args.as_object() else {
            return Ok(());
        };
        let values = GstFormValues {
            gst_section: json_string(obj.get("gst_section")),
            gst_inv_typ: json_string(obj.get("gst_inv_typ")),
            reverse_charge: match obj.get("reverse_charge") {
                Some(Value::Bool(true)) => "true".to_string(),
                Some(Value::String(value)) => value.clone(),
                _ => String::new(),
            },
            etin: json_string(obj.get("etin")),
            supplier_gstin: json_string(obj.get("supplier_gstin")),
            exp_typ: json_string(obj.get("exp_typ")),
            shipping_bill_number: json_string(obj.get("shipping_bill_number")),
            shipping_bill_date: json_string(obj.get("shipping_bill_date")),
            port_code: json_string(obj.get("port_code")),
            amendment_of_number: json_string(obj.get("amendment_of_number")),
            amendment_of_date: json_string(obj.get("amendment_of_date")),
        };
        let details = gst_from_form(&values)?;
        upsert_gst(db, table, id, &details).await?;
        if let Some(lines) = obj.get("lines").and_then(Value::as_array) {
            sync_line_supply(db, table, id, lines).await?;
        }
        Ok(())
    }

    async fn enrich(
        &self,
        db: &DatabaseConnection,
        record: InvoiceScriptRecord,
        id: i64,
    ) -> Result<Vec<(String, Value)>, String> {
        let Some(table) = gst_table(record) else {
            return Ok(Vec::new());
        };
        let details = load_gst(db, table, id).await?;
        Ok(vec![
            ("gst_section".into(), json!(details.gst_section)),
            ("gst_inv_typ".into(), json!(details.gst_inv_typ)),
            ("reverse_charge".into(), json!(details.reverse_charge)),
            ("etin".into(), json_opt(details.etin)),
            ("supplier_gstin".into(), json_opt(details.supplier_gstin)),
            ("exp_typ".into(), json_opt(details.exp_typ)),
            (
                "shipping_bill_number".into(),
                json_opt(details.shipping_bill_number),
            ),
            (
                "shipping_bill_date".into(),
                json_opt(details.shipping_bill_date.map(|date| date.to_string())),
            ),
            ("port_code".into(), json_opt(details.port_code)),
            (
                "amendment_of_number".into(),
                json_opt(details.amendment_of_number),
            ),
            (
                "amendment_of_date".into(),
                json_opt(details.amendment_of_date.map(|date| date.to_string())),
            ),
        ])
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

fn json_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn json_opt(value: Option<String>) -> Value {
    value.map_or(Value::Null, Value::String)
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
