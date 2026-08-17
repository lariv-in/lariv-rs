//! Build an XLSX workbook from export catalog selections.

use rust_xlsxwriter::{Format, Workbook};
use sea_orm::{ConnectionTrait, QueryResult, Statement};

use crate::export::{ExpandedSelection, ExportCatalog, ExportTable};

const EXCEL_MAX_CHARS: usize = 32_767;

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
    if entry.columns.is_empty() {
        return Ok(Vec::new());
    }
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
        for i in 0..entry.columns.len() {
            line.push(cell_value(&row, i));
        }
        out.push(line);
    }
    Ok(out)
}

fn cell_value(row: &QueryResult, idx: usize) -> String {
    let raw = if let Ok(v) = row.try_get_by_index::<Option<String>>(idx) {
        v.unwrap_or_default()
    } else if let Ok(v) = row.try_get_by_index::<Option<i64>>(idx) {
        v.map(|n| n.to_string()).unwrap_or_default()
    } else if let Ok(v) = row.try_get_by_index::<Option<f64>>(idx) {
        v.map(|n| n.to_string()).unwrap_or_default()
    } else if let Ok(v) = row.try_get_by_index::<Option<bool>>(idx) {
        v.map(|b| b.to_string()).unwrap_or_default()
    } else if let Ok(v) = row.try_get_by_index::<Option<Vec<u8>>>(idx) {
        v.map(|bytes| bytes.iter().map(|b| format!("{b:02x}")).collect())
            .unwrap_or_default()
    } else if let Ok(v) = row.try_get_by_index::<Option<chrono::DateTime<chrono::Utc>>>(idx) {
        v.map(|d| d.to_rfc3339()).unwrap_or_default()
    } else {
        String::new()
    };
    sanitize_xlsx_string(&raw)
}

fn sanitize_xlsx_string(s: &str) -> String {
    let filtered: String = s
        .chars()
        .filter(|c| {
            let u = *c as u32;
            u == 0x9 || u == 0xA || u == 0xD || (0x20..0xFFFE).contains(&u)
        })
        .collect();
    match filtered.char_indices().nth(EXCEL_MAX_CHARS) {
        Some((idx, _)) => filtered[..idx].to_string(),
        None => filtered,
    }
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
        .map(|c| {
            if c == '/' || c == '\\' || c == '?' || c == '*' || c == '[' || c == ']' {
                '_'
            } else {
                c
            }
        })
        .collect();
    if name.is_empty() {
        name = "export".into();
    }
    name
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

    use super::build_workbook;
    use crate::export::{ExpandedSelection, ExportCapability, ExportTable};

    #[tokio::test]
    async fn exports_blob_and_bool_columns() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("sqlite memory");
        db.execute(Statement::from_string(
            DbBackend::Sqlite,
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                name TEXT,
                is_superuser INTEGER,
                password BLOB
            )",
        ))
        .await
        .expect("create users");
        db.execute(Statement::from_string(
            DbBackend::Sqlite,
            "INSERT INTO users (id, name, is_superuser, password) VALUES (1, 'Ada', 1, x'00ff')",
        ))
        .await
        .expect("insert user");

        let catalog = ExportCapability::new()
            .register(ExportTable::new(
                "users",
                "User",
                vec![
                    "id".into(),
                    "name".into(),
                    "is_superuser".into(),
                    "password".into(),
                ],
            ))
            .catalog();
        let selection = ExpandedSelection {
            roots: vec!["users".into()],
            tables: vec!["users".into()],
        };
        let bytes = build_workbook(&db, &catalog, &selection)
            .await
            .expect("workbook");
        assert!(!bytes.is_empty());
    }
}
