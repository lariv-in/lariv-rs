//! Build an XLSX workbook from export catalog selections.

use rust_xlsxwriter::{Format, Workbook};
use sea_orm::{ConnectionTrait, Statement};

use crate::export::{ExportCatalog, ExportTable, ExpandedSelection};

pub async fn build_workbook<C: ConnectionTrait>(
    db: &C,
    catalog: &ExportCatalog,
    selection: &ExpandedSelection,
) -> Result<Vec<u8>, String> {
    if selection.tables.is_empty() {
        return Err("no tables selected for export".into());
    }

    let mut workbook = Workbook::new();
    let header_format = Format::new().set_bold();

    for (i, table) in selection.tables.iter().enumerate() {
        let entry = catalog
            .entry(table)
            .ok_or_else(|| format!("missing catalog entry for {table}"))?;
        let rows = fetch_table_rows(db, entry).await?;
        write_sheet(&mut workbook, table, entry, &rows, &header_format, i == 0)?;
    }

    workbook
        .save_to_buffer()
        .map_err(|e| format!("write workbook: {e}"))
}

async fn fetch_table_rows<C: ConnectionTrait>(
    db: &C,
    entry: &ExportTable,
) -> Result<Vec<Vec<String>>, String> {
    let _cols = if entry.columns.is_empty() {
        return Ok(Vec::new());
    };
    let col_list = entry
        .columns
        .iter()
        .map(|c| quote_ident(c))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("SELECT {} FROM {}", col_list, quote_ident(&entry.table));
    let backend = db.get_database_backend();
    let stmt = Statement::from_string(backend, sql);
    let rows = db
        .query_all(stmt)
        .await
        .map_err(|e| format!("query {}: {e}", entry.table))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let mut line = Vec::with_capacity(entry.columns.len());
        for (i, _col) in entry.columns.iter().enumerate() {
            let val: Option<String> = row.try_get_by_index(i).ok();
            line.push(val.unwrap_or_default());
        }
        out.push(line);
    }
    Ok(out)
}

fn write_sheet(
    workbook: &mut Workbook,
    sheet_name: &str,
    entry: &ExportTable,
    rows: &[Vec<String>],
    header_format: &Format,
    _first: bool,
) -> Result<(), String> {
    let name = unique_sheet_name(sheet_name);
    let worksheet = workbook.add_worksheet();
    worksheet.set_name(&name).map_err(|e| e.to_string())?;

    for (col, header) in entry.columns.iter().enumerate() {
        worksheet
            .write_string_with_format(0, col as u16, header, header_format)
            .map_err(|e| e.to_string())?;
    }
    for (row_idx, row) in rows.iter().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            worksheet
                .write_string((row_idx + 1) as u32, col_idx as u16, cell)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn unique_sheet_name(table: &str) -> String {
    let mut name: String = table
        .chars()
        .take(31)
        .map(|c| if c == '/' || c == '\\' || c == '?' || c == '*' || c == '[' || c == ']' {
            '_'
        } else {
            c
        })
        .collect();
    if name.is_empty() {
        name = "export".into();
    }
    name
}

fn quote_ident(name: &str) -> String {
    match name {
        "order" | "group" | "select" | "table" => format!("\"{name}\""),
        _ if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') => name.to_string(),
        _ => format!("\"{name}\""),
    }
}
