//! Parse export-plugin XLSX workbooks into catalog-aligned sheets.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::Cursor;

use calamine::{Data, Reader, Xlsx, open_workbook_from_rs};

use crate::export::ExportCatalog;

/// One catalog table parsed from a workbook sheet.
#[derive(Clone, Debug)]
pub struct ParsedSheet {
    pub table: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Workbook contents after matching sheets to the export catalog.
#[derive(Clone, Debug)]
pub struct ParsedWorkbook {
    pub sheets: Vec<ParsedSheet>,
    pub skipped_sheets: Vec<String>,
}

/// Read an XLSX buffer and keep only sheets registered in `catalog`.
pub fn parse_workbook(bytes: &[u8], catalog: &ExportCatalog) -> Result<ParsedWorkbook, String> {
    if bytes.is_empty() {
        return Err("empty file".into());
    }
    let mut workbook: Xlsx<_> =
        open_workbook_from_rs(Cursor::new(bytes)).map_err(|e| format!("open xlsx: {e}"))?;
    let names = workbook.sheet_names().to_vec();
    if names.is_empty() {
        return Err("workbook has no sheets".into());
    }

    let mut sheets = Vec::new();
    let mut skipped_sheets = Vec::new();
    for name in names {
        if catalog.entry(&name).is_none() {
            skipped_sheets.push(name);
            continue;
        }
        let range = workbook
            .worksheet_range(&name)
            .map_err(|e| format!("read sheet {name}: {e}"))?;
        let mut row_iter = range.rows();
        let Some(header_row) = row_iter.next() else {
            return Err(format!("sheet {name} has no header row"));
        };
        let headers: Vec<String> = header_row.iter().map(cell_to_string).collect();
        let entry = catalog
            .entry(&name)
            .ok_or_else(|| format!("missing catalog entry for {name}"))?;
        let mut columns = Vec::new();
        let mut col_indexes = Vec::new();
        for (idx, header) in headers.iter().enumerate() {
            if entry.columns.iter().any(|c| c == header) {
                columns.push(header.clone());
                col_indexes.push(idx);
            }
        }
        if columns.is_empty() {
            return Err(format!("sheet {name} has no catalog columns"));
        }
        let mut rows = Vec::new();
        for row in row_iter {
            let values: Vec<String> = col_indexes
                .iter()
                .map(|&idx| row.get(idx).map(cell_to_string).unwrap_or_default())
                .collect();
            if values.iter().all(|v| v.is_empty()) {
                continue;
            }
            rows.push(values);
        }
        sheets.push(ParsedSheet {
            table: name,
            columns,
            rows,
        });
    }

    if sheets.is_empty() {
        return Err("no registered tables in workbook".into());
    }

    let order = import_order(
        catalog,
        &sheets.iter().map(|s| s.table.clone()).collect::<Vec<_>>(),
    )?;
    let mut by_name: BTreeMap<String, ParsedSheet> =
        sheets.into_iter().map(|s| (s.table.clone(), s)).collect();
    let mut ordered = Vec::with_capacity(order.len());
    for table in order {
        if let Some(sheet) = by_name.remove(&table) {
            ordered.push(sheet);
        }
    }

    Ok(ParsedWorkbook {
        sheets: ordered,
        skipped_sheets,
    })
}

/// Tables in FK-target-first order (deps before dependents).
pub fn import_order(catalog: &ExportCatalog, tables: &[String]) -> Result<Vec<String>, String> {
    let set: BTreeSet<String> = tables.iter().cloned().collect();
    let mut incoming: BTreeMap<String, usize> =
        set.iter().cloned().map(|table| (table, 0)).collect();
    let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for table in &set {
        let Some(entry) = catalog.entry(table) else {
            continue;
        };
        for dep in &entry.immediate_deps {
            if !set.contains(dep) {
                continue;
            }
            let Some(count) = incoming.get_mut(table) else {
                continue;
            };
            *count = count.saturating_add(1);
            outgoing.entry(dep.clone()).or_default().push(table.clone());
        }
    }

    let mut ready: VecDeque<String> = incoming
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(table, _)| table.clone())
        .collect();
    let mut ordered = Vec::new();
    while let Some(table) = ready.pop_front() {
        ordered.push(table.clone());
        let Some(children) = outgoing.get(&table).cloned() else {
            continue;
        };
        for child in children {
            let Some(count) = incoming.get_mut(&child) else {
                continue;
            };
            *count = count.saturating_sub(1);
            if *count == 0 {
                ready.push_back(child);
            }
        }
    }

    if ordered.len() != set.len() {
        return Err("cyclic import dependencies".into());
    }
    Ok(ordered)
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(err) => format!("{err:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::import_order;
    use crate::export::{ExportCapability, ExportTable};

    #[test]
    fn import_order_puts_deps_first() {
        let catalog = ExportCapability::new()
            .register(ExportTable::new("roles", "Role", vec!["id".into()]))
            .register(
                ExportTable::new("users", "User", vec!["id".into(), "role_id".into()])
                    .with_deps(vec!["roles".into()]),
            )
            .register(
                ExportTable::new(
                    "clients",
                    "Client",
                    vec!["id".into(), "created_by_id".into()],
                )
                .with_deps(vec!["users".into()]),
            )
            .catalog();
        let order = import_order(
            &catalog,
            &["clients".into(), "users".into(), "roles".into()],
        )
        .expect("order");
        assert_eq!(order, vec!["roles", "users", "clients"]);
    }

    #[test]
    fn parse_workbook_rejects_empty_file() {
        let catalog = ExportCapability::new().catalog();
        let err = super::parse_workbook(&[], &catalog).expect_err("empty");
        assert!(err.contains("empty"), "{err}");
    }

    #[cfg(feature = "plugin-export")]
    #[test]
    fn parse_workbook_skips_unknown_sheets() {
        use rust_xlsxwriter::Workbook;

        let mut workbook = Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.set_name("nope").expect("name");
        sheet.write_string(0, 0, "id").expect("header");
        let roles = workbook.add_worksheet();
        roles.set_name("roles").expect("roles name");
        roles.write_string(0, 0, "id").expect("id");
        roles.write_string(0, 1, "name").expect("name");
        roles.write_string(1, 0, "1").expect("id val");
        roles.write_string(1, 1, "admin").expect("name val");
        let bytes = workbook.save_to_buffer().expect("xlsx");

        let catalog = ExportCapability::new()
            .register(ExportTable::new(
                "roles",
                "Role",
                vec!["id".into(), "name".into()],
            ))
            .catalog();
        let parsed = super::parse_workbook(&bytes, &catalog).expect("parse");
        assert_eq!(parsed.skipped_sheets, vec!["nope"]);
        assert_eq!(parsed.sheets.len(), 1);
        assert_eq!(parsed.sheets[0].table, "roles");
        assert_eq!(parsed.sheets[0].rows.len(), 1);
    }
}
