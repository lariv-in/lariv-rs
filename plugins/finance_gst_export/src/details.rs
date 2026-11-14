//! GST supply flag on each invoice line.

use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

#[derive(Clone, Copy)]
pub enum GstTable {
    Draft,
    Posted,
    Cancelled,
    Quotation,
}

pub fn normalize_supply(raw: &str) -> String {
    match raw.trim() {
        "nil" | "exempt" | "nongst" => raw.trim().to_string(),
        _ => "taxable".to_string(),
    }
}

pub fn name_is_gst_component(name: &str) -> bool {
    let upper = name.trim().to_ascii_uppercase();
    upper.starts_with("IGST")
        || upper.starts_with("CGST")
        || upper.starts_with("SGST")
        || upper.starts_with("UTGST")
        || upper.starts_with("CESS")
}

pub fn supply_for_line(raw: &str, has_gst_tax: bool) -> String {
    if has_gst_tax {
        "taxable".to_string()
    } else {
        normalize_supply(raw)
    }
}

pub async fn copy_line_supply<C: ConnectionTrait>(
    db: &C,
    from_lines: &str,
    from_parent_col: &str,
    from_parent: i64,
    to_lines: &str,
    to_parent_col: &str,
    to_parent: i64,
) -> Result<(), String> {
    let from_rows = line_supplies(db, from_lines, from_parent_col, from_parent).await?;
    let to_ids = line_ids(db, to_lines, to_parent_col, to_parent).await?;
    if from_rows.len() != to_ids.len() {
        return Err("GST line supply copy found a different number of lines".into());
    }
    for (supply, id) in from_rows.into_iter().zip(to_ids) {
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("UPDATE {to_lines} SET gst_supply = $1 WHERE id = $2"),
            [supply.into(), id.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn line_supplies<C: ConnectionTrait>(
    db: &C,
    table: &str,
    parent_col: &str,
    parent: i64,
) -> Result<Vec<String>, String> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("SELECT gst_supply FROM {table} WHERE {parent_col} = $1 ORDER BY id"),
            [parent.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    rows.iter()
        .map(|row| {
            row.try_get::<String>("", "gst_supply")
                .map_err(|e| e.to_string())
        })
        .collect()
}

async fn line_ids<C: ConnectionTrait>(
    db: &C,
    table: &str,
    parent_col: &str,
    parent: i64,
) -> Result<Vec<i64>, String> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("SELECT id FROM {table} WHERE {parent_col} = $1 ORDER BY id"),
            [parent.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    rows.iter()
        .map(|row| row.try_get::<i64>("", "id").map_err(|e| e.to_string()))
        .collect()
}

pub fn line_table(table: GstTable) -> (&'static str, &'static str) {
    match table {
        GstTable::Draft => ("draft_invoice_lines", "draft_invoice_id"),
        GstTable::Posted => ("posted_invoice_lines", "posted_invoice_id"),
        GstTable::Cancelled => ("cancelled_invoice_lines", "cancelled_invoice_id"),
        GstTable::Quotation => ("quotation_invoice_lines", "quotation_invoice_id"),
    }
}
