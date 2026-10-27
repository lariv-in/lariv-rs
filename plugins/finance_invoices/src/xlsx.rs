//! Invoice workbook: one sheet per table, foreign keys as internal links.

use std::collections::{BTreeSet, HashMap, HashSet};

use rust_xlsxwriter::{Format, Workbook, utility::quote_sheet_name};
use sea_orm::{ConnectionTrait, DatabaseBackend, QueryResult, Statement, Value};

const EXCEL_MAX_CHARS: usize = 32_767;
const ID_CHUNK: usize = 400;
const MAX_PASSES: usize = 64;

struct Fk {
    column: &'static str,
    target: &'static str,
}

struct SheetDef {
    table: &'static str,
    columns: &'static [&'static str],
    /// Load every row. Invoice types are roots; everything else is reached from them.
    root: bool,
    /// `(column on this table, parent table)` — load rows whose column is a loaded parent id.
    parent: Option<(&'static str, &'static str)>,
    fks: &'static [Fk],
}

#[derive(Default)]
struct SheetData {
    rows: Vec<Vec<String>>,
    ids: HashSet<String>,
    /// Ids requested for this sheet, including ones that were not found.
    attempted: HashSet<String>,
    /// Parent ids already used to query this child sheet.
    queried_parents: HashSet<String>,
}

/// Build one workbook of every invoice type and the rows those invoices reference.
pub async fn build_invoice_workbook<C: ConnectionTrait>(db: &C) -> Result<Vec<u8>, String> {
    let specs = sheet_specs();
    let backend = db.get_database_backend();
    let mut data: HashMap<&'static str, SheetData> = HashMap::new();
    for spec in specs {
        data.insert(spec.table, SheetData::default());
    }

    for spec in specs.iter().filter(|spec| spec.root) {
        let rows = fetch_all(db, backend, spec).await?;
        absorb(spec, data.get_mut(spec.table).expect("sheet"), rows);
    }

    for _ in 0..MAX_PASSES {
        if !expand_once(db, backend, specs, &mut data).await? {
            break;
        }
    }

    write_workbook(specs, &data)
}

async fn expand_once<C: ConnectionTrait>(
    db: &C,
    backend: DatabaseBackend,
    specs: &'static [SheetDef],
    data: &mut HashMap<&'static str, SheetData>,
) -> Result<bool, String> {
    let mut grew = false;

    let mut needed: HashMap<&'static str, BTreeSet<String>> = HashMap::new();
    for spec in specs {
        let sheet = &data[spec.table];
        for fk in spec.fks {
            let col = column_index(spec, fk.column)?;
            let target = &data[fk.target];
            for row in &sheet.rows {
                let id = row.get(col).map(String::as_str).unwrap_or("");
                if id.is_empty() || target.ids.contains(id) || target.attempted.contains(id) {
                    continue;
                }
                needed.entry(fk.target).or_default().insert(id.to_string());
            }
        }
    }

    for (target, ids) in needed {
        let spec = spec_by_table(specs, target)?;
        let id_list: Vec<String> = ids.into_iter().collect();
        let rows = fetch_where_in(db, backend, spec, "id", &id_list).await?;
        let sheet = data.get_mut(target).expect("sheet");
        sheet.attempted.extend(id_list);
        let before = sheet.rows.len();
        absorb(spec, sheet, rows);
        if sheet.rows.len() != before {
            grew = true;
        }
    }

    let mut child_jobs: Vec<(&'static str, Vec<String>)> = Vec::new();
    for spec in specs {
        let Some((_parent_col, parent_table)) = spec.parent else {
            continue;
        };
        let sheet = &data[spec.table];
        let parents = &data[parent_table];
        let missing: Vec<String> = parents
            .ids
            .iter()
            .filter(|id| !sheet.queried_parents.contains(*id))
            .cloned()
            .collect();
        if !missing.is_empty() {
            child_jobs.push((spec.table, missing));
        }
    }

    for (table, parent_ids) in child_jobs {
        let spec = spec_by_table(specs, table)?;
        let (parent_col, _) = spec.parent.expect("child sheet");
        let rows = fetch_where_in(db, backend, spec, parent_col, &parent_ids).await?;
        let sheet = data.get_mut(table).expect("sheet");
        sheet.queried_parents.extend(parent_ids);
        let before = sheet.rows.len();
        absorb(spec, sheet, rows);
        if sheet.rows.len() != before {
            grew = true;
        }
    }

    Ok(grew)
}

fn absorb(spec: &SheetDef, sheet: &mut SheetData, rows: Vec<Vec<String>>) {
    let id_idx = spec.columns.iter().position(|column| *column == "id");
    for row in rows {
        if let Some(idx) = id_idx {
            let id = row.get(idx).cloned().unwrap_or_default();
            if id.is_empty() || !sheet.ids.insert(id) {
                continue;
            }
        }
        sheet.rows.push(row);
    }
}

fn write_workbook(
    specs: &'static [SheetDef],
    data: &HashMap<&'static str, SheetData>,
) -> Result<Vec<u8>, String> {
    let mut prepared: Vec<Prepared> = Vec::new();

    for spec in specs {
        let sheet = &data[spec.table];
        if sheet.rows.is_empty() {
            continue;
        }
        let mut rows = sheet.rows.clone();
        let id_idx = spec.columns.iter().position(|column| *column == "id");
        sort_rows(&mut rows, id_idx);
        let fks = spec
            .fks
            .iter()
            .map(|fk| Ok((column_index(spec, fk.column)?, fk.target)))
            .collect::<Result<Vec<_>, String>>()?;
        prepared.push(Prepared {
            table: spec.table,
            columns: spec
                .columns
                .iter()
                .map(|column| (*column).to_string())
                .collect(),
            rows,
            fks,
        });
    }

    append_relation_links(&mut prepared);

    let mut row_index: HashMap<&'static str, HashMap<String, usize>> = HashMap::new();
    for sheet in &prepared {
        let Some(id_idx) = sheet.columns.iter().position(|column| *column == "id") else {
            continue;
        };
        let mut ids = HashMap::new();
        for (offset, row) in sheet.rows.iter().enumerate() {
            let id = row.get(id_idx).cloned().unwrap_or_default();
            if !id.is_empty() {
                ids.insert(id, offset);
            }
        }
        row_index.insert(sheet.table, ids);
    }

    let mut locations: HashMap<(&'static str, String), LinkTarget> = HashMap::new();
    for sheet in &prepared {
        let Some(ids) = row_index.get(sheet.table) else {
            continue;
        };
        for (id, offset) in ids {
            let excel_row = u32::try_from(offset + 2).map_err(|e| e.to_string())?;
            let label = fk_label(sheet.table, id, &prepared, &row_index);
            locations.insert((sheet.table, id.clone()), LinkTarget { excel_row, label });
        }
    }

    let mut workbook = Workbook::new();
    let header_format = Format::new().set_bold();
    for sheet in &prepared {
        if OMITTED_SHEETS.contains(&sheet.table) {
            continue;
        }
        write_sheet(&mut workbook, sheet, &locations, &header_format)?;
    }
    workbook
        .save_to_buffer()
        .map_err(|e| format!("write workbook: {e}"))
}

struct Prepared {
    table: &'static str,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    fks: Vec<(usize, &'static str)>,
}

const OMITTED_SHEETS: &[&str] = &[
    "draft_invoice_taxes",
    "posted_invoice_taxes",
    "cancelled_invoice_taxes",
    "draft_invoice_goods_line_taxes",
    "posted_invoice_goods_line_taxes",
    "cancelled_invoice_goods_line_taxes",
    "draft_payment_terms",
    "posted_payment_terms",
];

struct RelArray {
    parent: &'static str,
    /// Column on the parent whose value matches `child_key`.
    parent_key: &'static str,
    child: &'static str,
    child_key: &'static str,
    prefix: &'static str,
    /// Column on the child to link. `None` links to the child row itself.
    link_column: Option<&'static str>,
    link_table: &'static str,
}

fn append_relation_links(prepared: &mut [Prepared]) {
    for spec in [
        RelArray {
            parent: "draft_invoices",
            parent_key: "id",
            child: "draft_invoice_goods_lines",
            child_key: "draft_invoice_id",
            prefix: "line",
            link_column: None,
            link_table: "draft_invoice_goods_lines",
        },
        RelArray {
            parent: "draft_invoices",
            parent_key: "id",
            child: "draft_invoice_service_lines",
            child_key: "draft_invoice_id",
            prefix: "service_line",
            link_column: None,
            link_table: "draft_invoice_service_lines",
        },
        RelArray {
            parent: "posted_invoices",
            parent_key: "id",
            child: "posted_invoice_service_lines",
            child_key: "posted_invoice_id",
            prefix: "service_line",
            link_column: None,
            link_table: "posted_invoice_service_lines",
        },
        RelArray {
            parent: "cancelled_invoices",
            parent_key: "id",
            child: "cancelled_invoice_service_lines",
            child_key: "cancelled_invoice_id",
            prefix: "service_line",
            link_column: None,
            link_table: "cancelled_invoice_service_lines",
        },
        RelArray {
            parent: "posted_invoices",
            parent_key: "id",
            child: "posted_invoice_goods_lines",
            child_key: "posted_invoice_id",
            prefix: "line",
            link_column: None,
            link_table: "posted_invoice_goods_lines",
        },
        RelArray {
            parent: "cancelled_invoices",
            parent_key: "id",
            child: "cancelled_invoice_goods_lines",
            child_key: "cancelled_invoice_id",
            prefix: "line",
            link_column: None,
            link_table: "cancelled_invoice_goods_lines",
        },
        RelArray {
            parent: "draft_invoices",
            parent_key: "draft_payment_term_id",
            child: "draft_payment_term_lines",
            child_key: "draft_payment_term_id",
            prefix: "payment_term",
            link_column: None,
            link_table: "draft_payment_term_lines",
        },
        RelArray {
            parent: "posted_invoices",
            parent_key: "posted_payment_term_id",
            child: "posted_payment_term_lines",
            child_key: "posted_payment_term_id",
            prefix: "payment_term",
            link_column: None,
            link_table: "posted_payment_term_lines",
        },
        RelArray {
            parent: "cancelled_invoices",
            parent_key: "posted_payment_term_id",
            child: "posted_payment_term_lines",
            child_key: "posted_payment_term_id",
            prefix: "payment_term",
            link_column: None,
            link_table: "posted_payment_term_lines",
        },
        RelArray {
            parent: "draft_invoices",
            parent_key: "id",
            child: "draft_invoice_taxes",
            child_key: "draft_invoice_id",
            prefix: "header_tax",
            link_column: Some("tax_id"),
            link_table: "taxes",
        },
        RelArray {
            parent: "posted_invoices",
            parent_key: "id",
            child: "posted_invoice_taxes",
            child_key: "posted_invoice_id",
            prefix: "header_tax",
            link_column: Some("tax_id"),
            link_table: "taxes",
        },
        RelArray {
            parent: "cancelled_invoices",
            parent_key: "id",
            child: "cancelled_invoice_taxes",
            child_key: "cancelled_invoice_id",
            prefix: "header_tax",
            link_column: Some("tax_id"),
            link_table: "taxes",
        },
        RelArray {
            parent: "draft_invoice_goods_lines",
            parent_key: "id",
            child: "draft_invoice_goods_line_taxes",
            child_key: "draft_invoice_goods_line_id",
            prefix: "tax",
            link_column: Some("tax_id"),
            link_table: "taxes",
        },
        RelArray {
            parent: "posted_invoice_goods_lines",
            parent_key: "id",
            child: "posted_invoice_goods_line_taxes",
            child_key: "posted_invoice_goods_line_id",
            prefix: "tax",
            link_column: Some("tax_id"),
            link_table: "taxes",
        },
        RelArray {
            parent: "cancelled_invoice_goods_lines",
            parent_key: "id",
            child: "cancelled_invoice_goods_line_taxes",
            child_key: "cancelled_invoice_goods_line_id",
            prefix: "tax",
            link_column: Some("tax_id"),
            link_table: "taxes",
        },
    ] {
        append_link_array(prepared, &spec);
    }
    for (table, column) in [
        ("draft_invoices", "draft_payment_term_id"),
        ("posted_invoices", "posted_payment_term_id"),
        ("cancelled_invoices", "posted_payment_term_id"),
        ("draft_payment_term_lines", "draft_payment_term_id"),
        ("posted_payment_term_lines", "posted_payment_term_id"),
    ] {
        drop_column(prepared, table, column);
    }
}

fn append_link_array(prepared: &mut [Prepared], spec: &RelArray) {
    let Some(child) = prepared.iter().find(|sheet| sheet.table == spec.child) else {
        return;
    };
    let child_columns = child.columns.clone();
    let child_rows = child.rows.clone();
    let Some(parent_at) = prepared.iter().position(|sheet| sheet.table == spec.parent) else {
        return;
    };
    let parent_columns = prepared[parent_at].columns.clone();
    let grouped: Vec<Vec<String>> = prepared[parent_at]
        .rows
        .iter()
        .map(|row| {
            let key = row_value(&parent_columns, row, spec.parent_key);
            sorted_child_indexes(&child_columns, &child_rows, spec.child_key, &key)
                .iter()
                .filter_map(|idx| child_rows.get(*idx))
                .map(|child_row| match spec.link_column {
                    Some(column) => row_value(&child_columns, child_row, column),
                    None => row_value(&child_columns, child_row, "id"),
                })
                .filter(|value| !value.is_empty())
                .collect()
        })
        .collect();
    let width = grouped.iter().map(Vec::len).max().unwrap_or(0);
    if width == 0 {
        return;
    }
    let parent = &mut prepared[parent_at];
    for slot in 1..=width {
        parent.fks.push((parent.columns.len(), spec.link_table));
        parent.columns.push(format!("{}_{slot}", spec.prefix));
    }
    for (row, links) in parent.rows.iter_mut().zip(grouped) {
        for slot in 0..width {
            row.push(links.get(slot).cloned().unwrap_or_default());
        }
    }
}

fn drop_column(prepared: &mut [Prepared], table: &str, column: &str) {
    let Some(sheet) = prepared.iter_mut().find(|sheet| sheet.table == table) else {
        return;
    };
    let Some(idx) = sheet.columns.iter().position(|name| name == column) else {
        return;
    };
    sheet.columns.remove(idx);
    sheet.fks.retain_mut(|(fk_idx, _)| {
        if *fk_idx == idx {
            return false;
        }
        if *fk_idx > idx {
            *fk_idx -= 1;
        }
        true
    });
    for row in &mut sheet.rows {
        if idx < row.len() {
            row.remove(idx);
        }
    }
}

fn sorted_child_indexes(
    columns: &[String],
    rows: &[Vec<String>],
    child_key: &str,
    parent_value: &str,
) -> Vec<usize> {
    let mut indexes = parent_indexes(columns, rows, child_key, parent_value);
    indexes.sort_by_key(|idx| {
        let row = rows.get(*idx);
        let order = row
            .map(|line| row_value(columns, line, "line_order"))
            .unwrap_or_default();
        let id = row
            .map(|line| row_value(columns, line, "id"))
            .unwrap_or_default();
        (
            order.parse::<i64>().unwrap_or(0),
            id.parse::<i64>().unwrap_or(0),
        )
    });
    indexes
}

fn parent_indexes(
    columns: &[String],
    rows: &[Vec<String>],
    parent_col: &str,
    parent_id: &str,
) -> Vec<usize> {
    if parent_id.is_empty() {
        return Vec::new();
    }
    let Some(col) = columns.iter().position(|column| column == parent_col) else {
        return Vec::new();
    };
    rows.iter()
        .enumerate()
        .filter_map(|(idx, row)| {
            let value = row.get(col).map(String::as_str).unwrap_or("");
            (value == parent_id).then_some(idx)
        })
        .collect()
}

fn format_term_line(columns: &[String], row: &[String]) -> String {
    if columns.iter().any(|column| column == "date_kind") {
        let due_date = row_value(columns, row, "due_date");
        let due_duration = human_duration(&row_value(columns, row, "due_duration"));
        let date_kind = row_value(columns, row, "date_kind");
        let when = if !due_date.is_empty() {
            due_date
        } else if !due_duration.is_empty() {
            due_duration
        } else {
            date_kind
        };
        let amount = row_value(columns, row, "amount");
        let percentage = row_value(columns, row, "amount_percentage");
        let money = if !amount.is_empty() {
            amount
        } else if !percentage.is_empty() {
            format!("{percentage}%")
        } else {
            row_value(columns, row, "amount_kind")
        };
        return join_parts(&[when.as_str(), money.as_str()]);
    }
    let due_date = row_value(columns, row, "due_date");
    let amount = row_value(columns, row, "amount");
    join_parts(&[due_date.as_str(), amount.as_str()])
}

fn human_duration(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Ok(nanos) = trimmed.parse::<i64>() {
        let formatted = lariv_core::duration::format_duration(nanos);
        if !formatted.is_empty() {
            return formatted;
        }
    }
    trimmed.to_string()
}

fn row_value(columns: &[String], row: &[String], column: &str) -> String {
    columns
        .iter()
        .position(|name| name == column)
        .and_then(|idx| row.get(idx))
        .cloned()
        .unwrap_or_default()
}

struct LinkTarget {
    excel_row: u32,
    /// Visible text for links to this row. Empty means show the id.
    label: String,
}

fn write_sheet(
    workbook: &mut Workbook,
    sheet: &Prepared,
    locations: &HashMap<(&'static str, String), LinkTarget>,
    header_format: &Format,
) -> Result<(), String> {
    let name = sheet_name(sheet.table);
    let worksheet = workbook.add_worksheet();
    worksheet
        .set_name(&name)
        .map_err(|e| format!("sheet {}: {e}", sheet.table))?;

    for (col, header) in sheet.columns.iter().enumerate() {
        worksheet
            .write_string_with_format(0, col as u16, header, header_format)
            .map_err(|e| e.to_string())?;
    }

    let fk_targets: HashMap<usize, &'static str> = sheet.fks.iter().copied().collect();
    for (offset, row) in sheet.rows.iter().enumerate() {
        let excel_index = u32::try_from(offset + 1).map_err(|e| e.to_string())?;
        for (col, cell) in row.iter().enumerate() {
            if cell.is_empty() {
                continue;
            }
            let col_idx = u16::try_from(col).map_err(|e| e.to_string())?;
            if let Some(target) = fk_targets.get(&col)
                && let Some(link_target) = locations.get(&(*target, cell.clone()))
            {
                let link = format!(
                    "internal:{}!A{}",
                    quote_sheet_name(target),
                    link_target.excel_row
                );
                let text = if link_target.label.is_empty() {
                    cell.as_str()
                } else {
                    link_target.label.as_str()
                };
                worksheet
                    .write_url_with_text(excel_index, col_idx, link.as_str(), text)
                    .map_err(|e| e.to_string())?;
                continue;
            }
            worksheet
                .write_string(excel_index, col_idx, cell)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn fk_label(
    table: &str,
    id: &str,
    sheets: &[Prepared],
    index: &HashMap<&'static str, HashMap<String, usize>>,
) -> String {
    let label = match table {
        "crm_companies" => field(sheets, index, table, id, "name").to_string(),
        "crm_contacts" => combine_names(
            field(sheets, index, table, id, "name"),
            company_name(sheets, index, field(sheets, index, table, id, "company_id")),
        ),
        "crm_customers" => customer_label(sheets, index, id),
        "products" | "taxes" | "journals" | "currencies" => {
            field(sheets, index, table, id, "name").to_string()
        }
        "accounts" => account_label(
            field(sheets, index, table, id, "code"),
            field(sheets, index, table, id, "name"),
        ),
        "draft_invoices" | "posted_invoices" | "cancelled_invoices" => {
            field(sheets, index, table, id, "number").to_string()
        }
        "credit_notes" => field(sheets, index, table, id, "reason").to_string(),
        "payments" => field(sheets, index, table, id, "amount").to_string(),
        "payment_batches" => field(sheets, index, table, id, "total_amount").to_string(),
        "source_docs" => field(sheets, index, table, id, "source_doc_type").to_string(),
        "journal_entries" => join_parts(&[
            field(
                sheets,
                index,
                "journals",
                field(sheets, index, table, id, "journal_id"),
                "name",
            ),
            field(sheets, index, table, id, "datetime"),
        ]),
        "journal_entry_items" => {
            let account_id = field(sheets, index, table, id, "account_id");
            let account = account_label(
                field(sheets, index, "accounts", account_id, "code"),
                field(sheets, index, "accounts", account_id, "name"),
            );
            join_parts(&[account.as_str(), field(sheets, index, table, id, "amount")])
        }
        "draft_invoice_goods_lines"
        | "posted_invoice_goods_lines"
        | "cancelled_invoice_goods_lines" => field(
            sheets,
            index,
            "products",
            field(sheets, index, table, id, "product_id"),
            "name",
        )
        .to_string(),
        "draft_invoice_service_lines"
        | "posted_invoice_service_lines"
        | "cancelled_invoice_service_lines" => field(
            sheets,
            index,
            "services",
            field(sheets, index, table, id, "service_id"),
            "name",
        )
        .to_string(),
        "draft_payment_term_lines" | "posted_payment_term_lines" => {
            term_line_label(sheets, index, table, id)
        }
        _ => String::new(),
    };
    label.trim().to_string()
}

fn term_line_label(
    sheets: &[Prepared],
    index: &HashMap<&'static str, HashMap<String, usize>>,
    table: &str,
    id: &str,
) -> String {
    let Some(sheet) = sheets.iter().find(|sheet| sheet.table == table) else {
        return String::new();
    };
    let Some(offset) = index.get(table).and_then(|ids| ids.get(id)).copied() else {
        return String::new();
    };
    let Some(row) = sheet.rows.get(offset) else {
        return String::new();
    };
    format_term_line(&sheet.columns, row)
}

fn customer_label(
    sheets: &[Prepared],
    index: &HashMap<&'static str, HashMap<String, usize>>,
    id: &str,
) -> String {
    if is_truthy(field(sheets, index, "crm_customers", id, "is_individual")) {
        let contact_id = field(sheets, index, "crm_customers", id, "individual_id");
        combine_names(
            field(sheets, index, "crm_contacts", contact_id, "name"),
            company_name(
                sheets,
                index,
                field(sheets, index, "crm_contacts", contact_id, "company_id"),
            ),
        )
    } else {
        company_name(
            sheets,
            index,
            field(sheets, index, "crm_customers", id, "company_id"),
        )
        .to_string()
    }
}

fn company_name<'a>(
    sheets: &'a [Prepared],
    index: &HashMap<&'static str, HashMap<String, usize>>,
    company_id: &str,
) -> &'a str {
    field(sheets, index, "crm_companies", company_id, "name")
}

fn field<'a>(
    sheets: &'a [Prepared],
    index: &HashMap<&'static str, HashMap<String, usize>>,
    table: &str,
    id: &str,
    column: &str,
) -> &'a str {
    if id.is_empty() {
        return "";
    }
    let Some(sheet) = sheets.iter().find(|sheet| sheet.table == table) else {
        return "";
    };
    let Some(offset) = index.get(table).and_then(|ids| ids.get(id)).copied() else {
        return "";
    };
    let Some(col) = sheet.columns.iter().position(|name| *name == column) else {
        return "";
    };
    sheet
        .rows
        .get(offset)
        .and_then(|row| row.get(col))
        .map(String::as_str)
        .unwrap_or("")
}

fn combine_names(name: &str, company: &str) -> String {
    let name = name.trim();
    let company = company.trim();
    match (name.is_empty(), company.is_empty()) {
        (false, false) => format!("{name}, {company}"),
        (false, true) => name.to_string(),
        (true, false) => company.to_string(),
        (true, true) => String::new(),
    }
}

fn account_label(code: &str, name: &str) -> String {
    let code = code.trim();
    let name = name.trim();
    match (code.is_empty(), name.is_empty()) {
        (false, false) => format!("{code} — {name}"),
        (false, true) => code.to_string(),
        (true, false) => name.to_string(),
        (true, true) => String::new(),
    }
}

fn join_parts(parts: &[&str]) -> String {
    parts
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim(),
        "1" | "t" | "true" | "TRUE" | "True" | "yes" | "YES"
    )
}

fn sort_rows(rows: &mut [Vec<String>], id_idx: Option<usize>) {
    rows.sort_by(|left, right| match id_idx {
        Some(idx) => cmp_id(left.get(idx), right.get(idx)).then_with(|| left.cmp(right)),
        None => left.cmp(right),
    });
}

fn cmp_id(left: Option<&String>, right: Option<&String>) -> std::cmp::Ordering {
    let left = left.map(String::as_str).unwrap_or("");
    let right = right.map(String::as_str).unwrap_or("");
    match (left.parse::<i64>(), right.parse::<i64>()) {
        (Ok(a), Ok(b)) => a.cmp(&b),
        _ => left.cmp(right),
    }
}

fn sheet_name(table: &str) -> String {
    let mut name: String = table
        .chars()
        .take(31)
        .map(|c| match c {
            '/' | '\\' | '?' | '*' | '[' | ']' => '_',
            other => other,
        })
        .collect();
    if name.is_empty() {
        name = "export".into();
    }
    name
}

fn column_index(spec: &SheetDef, column: &str) -> Result<usize, String> {
    spec.columns
        .iter()
        .position(|name| *name == column)
        .ok_or_else(|| format!("{} has no column {column}", spec.table))
}

fn spec_by_table(specs: &'static [SheetDef], table: &str) -> Result<&'static SheetDef, String> {
    specs
        .iter()
        .find(|spec| spec.table == table)
        .ok_or_else(|| format!("unknown sheet {table}"))
}

async fn fetch_all<C: ConnectionTrait>(
    db: &C,
    backend: DatabaseBackend,
    spec: &SheetDef,
) -> Result<Vec<Vec<String>>, String> {
    let sql = format!(
        "SELECT {} FROM {}",
        column_list(spec),
        quote_ident(spec.table)
    );
    let stmt = Statement::from_string(backend, sql);
    query_rows(db, spec, stmt).await
}

async fn fetch_where_in<C: ConnectionTrait>(
    db: &C,
    backend: DatabaseBackend,
    spec: &SheetDef,
    column: &str,
    ids: &[String],
) -> Result<Vec<Vec<String>>, String> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for chunk in ids.chunks(ID_CHUNK) {
        let placeholders = (0..chunk.len())
            .map(|i| match backend {
                DatabaseBackend::Postgres => format!("${}", i + 1),
                _ => "?".to_string(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} IN ({placeholders})",
            column_list(spec),
            quote_ident(spec.table),
            quote_ident(column),
        );
        let values = chunk
            .iter()
            .map(|id| match id.parse::<i64>() {
                Ok(n) => Value::BigInt(Some(n)),
                Err(_) => Value::String(Some(id.clone())),
            })
            .collect::<Vec<_>>();
        let stmt = Statement::from_sql_and_values(backend, sql, values);
        out.extend(query_rows(db, spec, stmt).await?);
    }
    Ok(out)
}

fn column_list(spec: &SheetDef) -> String {
    spec.columns
        .iter()
        .map(|column| quote_ident(column))
        .collect::<Vec<_>>()
        .join(", ")
}

async fn query_rows<C: ConnectionTrait>(
    db: &C,
    spec: &SheetDef,
    stmt: Statement,
) -> Result<Vec<Vec<String>>, String> {
    let rows = db
        .query_all_raw(stmt)
        .await
        .map_err(|e| format!("query {}: {e}", spec.table))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let mut line = Vec::with_capacity(spec.columns.len());
        for idx in 0..spec.columns.len() {
            line.push(cell_value(&row, idx));
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
    } else if let Ok(v) = row.try_get_by_index::<Option<i32>>(idx) {
        v.map(|n| n.to_string()).unwrap_or_default()
    } else if let Ok(v) = row.try_get_by_index::<Option<f64>>(idx) {
        v.map(|n| n.to_string()).unwrap_or_default()
    } else if let Ok(v) = row.try_get_by_index::<Option<bool>>(idx) {
        v.map(|b| b.to_string()).unwrap_or_default()
    } else if let Ok(v) = row.try_get_by_index::<Option<rust_decimal::Decimal>>(idx) {
        v.map(|n| n.normalize().to_string()).unwrap_or_default()
    } else if let Ok(v) = row.try_get_by_index::<Option<chrono::NaiveDate>>(idx) {
        v.map(|d| d.to_string()).unwrap_or_default()
    } else if let Ok(v) = row.try_get_by_index::<Option<chrono::NaiveDateTime>>(idx) {
        v.map(|d| d.to_string()).unwrap_or_default()
    } else if let Ok(v) = row.try_get_by_index::<Option<chrono::DateTime<chrono::Utc>>>(idx) {
        v.map(|d| d.to_rfc3339()).unwrap_or_default()
    } else if let Ok(v) = row.try_get_by_index::<Option<Vec<u8>>>(idx) {
        v.map(|bytes| bytes.iter().map(|b| format!("{b:02x}")).collect())
            .unwrap_or_default()
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
    filtered.chars().take(EXCEL_MAX_CHARS).collect()
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn sheet_specs() -> &'static [SheetDef] {
    SHEETS
}

const SHEETS: &[SheetDef] = &[
    SheetDef {
        table: "draft_invoices",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "number",
            "reference",
            "eway_bill_number",
            "payment_reference",
            "bank_account",
            "remarks",
            "datetime",
            "delivery_date",
            "customer_id",
            "draft_payment_term_id",
        ],
        root: true,
        parent: None,
        fks: &[
            Fk {
                column: "customer_id",
                target: "crm_customers",
            },
            Fk {
                column: "draft_payment_term_id",
                target: "draft_payment_terms",
            },
        ],
    },
    SheetDef {
        table: "posted_invoices",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "posted_at",
            "number",
            "reference",
            "eway_bill_number",
            "payment_reference",
            "bank_account",
            "remarks",
            "journal_id",
            "datetime",
            "delivery_date",
            "customer_id",
            "journal_entry_id",
            "posted_payment_term_id",
        ],
        root: true,
        parent: None,
        fks: &[
            Fk {
                column: "journal_id",
                target: "journals",
            },
            Fk {
                column: "customer_id",
                target: "crm_customers",
            },
            Fk {
                column: "journal_entry_id",
                target: "journal_entries",
            },
            Fk {
                column: "posted_payment_term_id",
                target: "posted_payment_terms",
            },
        ],
    },
    SheetDef {
        table: "cancelled_invoices",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "posted_invoice_id",
            "posted_at",
            "cancelled_at",
            "number",
            "reference",
            "eway_bill_number",
            "payment_reference",
            "bank_account",
            "remarks",
            "journal_id",
            "datetime",
            "delivery_date",
            "customer_id",
            "credit_note_id",
            "posted_payment_term_id",
        ],
        root: true,
        parent: None,
        fks: &[
            Fk {
                column: "posted_invoice_id",
                target: "posted_invoices",
            },
            Fk {
                column: "journal_id",
                target: "journals",
            },
            Fk {
                column: "customer_id",
                target: "crm_customers",
            },
            Fk {
                column: "credit_note_id",
                target: "credit_notes",
            },
            Fk {
                column: "posted_payment_term_id",
                target: "posted_payment_terms",
            },
        ],
    },
    SheetDef {
        table: "paid_invoices",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "payment_id",
            "posted_invoice_id",
            "prior_partially_paid_invoice_id",
        ],
        root: true,
        parent: None,
        fks: &[
            Fk {
                column: "payment_id",
                target: "payments",
            },
            Fk {
                column: "posted_invoice_id",
                target: "posted_invoices",
            },
            Fk {
                column: "prior_partially_paid_invoice_id",
                target: "partially_paid_invoices",
            },
        ],
    },
    SheetDef {
        table: "partially_paid_invoices",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "payment_id",
            "posted_invoice_id",
            "prior_partially_paid_invoice_id",
        ],
        root: true,
        parent: None,
        fks: &[
            Fk {
                column: "payment_id",
                target: "payments",
            },
            Fk {
                column: "posted_invoice_id",
                target: "posted_invoices",
            },
            Fk {
                column: "prior_partially_paid_invoice_id",
                target: "partially_paid_invoices",
            },
        ],
    },
    SheetDef {
        table: "draft_invoice_goods_lines",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "draft_invoice_id",
            "product_id",
            "product_name",
            "product_reference",
            "product_remarks",
            "hsn_code",
            "variable_types",
            "base_price_formula",
            "sales_price_formula",
            "rate",
            "quantity",
            "variable_values",
            "pre_tax_amount",
            "remarks",
        ],
        root: false,
        parent: Some(("draft_invoice_id", "draft_invoices")),
        fks: &[
            Fk {
                column: "draft_invoice_id",
                target: "draft_invoices",
            },
            Fk {
                column: "product_id",
                target: "products",
            },
        ],
    },
    SheetDef {
        table: "draft_invoice_service_lines",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "draft_invoice_id",
            "service_id",
            "service_name",
            "service_reference",
            "service_remarks",
            "sac_code",
            "variable_types",
            "base_price_formula",
            "sales_price_formula",
            "rate",
            "quantity",
            "variable_values",
            "pre_tax_amount",
            "remarks",
        ],
        root: false,
        parent: Some(("draft_invoice_id", "draft_invoices")),
        fks: &[
            Fk {
                column: "draft_invoice_id",
                target: "draft_invoices",
            },
            Fk {
                column: "service_id",
                target: "services",
            },
        ],
    },
    SheetDef {
        table: "draft_invoice_taxes",
        columns: &["draft_invoice_id", "tax_id"],
        root: false,
        parent: Some(("draft_invoice_id", "draft_invoices")),
        fks: &[
            Fk {
                column: "draft_invoice_id",
                target: "draft_invoices",
            },
            Fk {
                column: "tax_id",
                target: "taxes",
            },
        ],
    },
    SheetDef {
        table: "draft_invoice_goods_line_taxes",
        columns: &["draft_invoice_goods_line_id", "tax_id"],
        root: false,
        parent: Some(("draft_invoice_goods_line_id", "draft_invoice_goods_lines")),
        fks: &[
            Fk {
                column: "draft_invoice_goods_line_id",
                target: "draft_invoice_goods_lines",
            },
            Fk {
                column: "tax_id",
                target: "taxes",
            },
        ],
    },
    SheetDef {
        table: "posted_invoice_goods_lines",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "posted_invoice_id",
            "product_id",
            "product_name",
            "product_reference",
            "product_remarks",
            "hsn_code",
            "variable_types",
            "base_price_formula",
            "sales_price_formula",
            "rate",
            "quantity",
            "variable_values",
            "pre_tax_amount",
            "remarks",
            "journal_entry_item_id",
        ],
        root: false,
        parent: Some(("posted_invoice_id", "posted_invoices")),
        fks: &[
            Fk {
                column: "posted_invoice_id",
                target: "posted_invoices",
            },
            Fk {
                column: "product_id",
                target: "products",
            },
            Fk {
                column: "journal_entry_item_id",
                target: "journal_entry_items",
            },
        ],
    },
    SheetDef {
        table: "posted_invoice_service_lines",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "posted_invoice_id",
            "service_id",
            "service_name",
            "service_reference",
            "service_remarks",
            "sac_code",
            "variable_types",
            "base_price_formula",
            "sales_price_formula",
            "rate",
            "quantity",
            "variable_values",
            "pre_tax_amount",
            "remarks",
            "journal_entry_item_id",
        ],
        root: false,
        parent: Some(("posted_invoice_id", "posted_invoices")),
        fks: &[
            Fk {
                column: "posted_invoice_id",
                target: "posted_invoices",
            },
            Fk {
                column: "service_id",
                target: "services",
            },
            Fk {
                column: "journal_entry_item_id",
                target: "journal_entry_items",
            },
        ],
    },
    SheetDef {
        table: "posted_invoice_taxes",
        columns: &["posted_invoice_id", "tax_id"],
        root: false,
        parent: Some(("posted_invoice_id", "posted_invoices")),
        fks: &[
            Fk {
                column: "posted_invoice_id",
                target: "posted_invoices",
            },
            Fk {
                column: "tax_id",
                target: "taxes",
            },
        ],
    },
    SheetDef {
        table: "posted_invoice_goods_line_taxes",
        columns: &["posted_invoice_goods_line_id", "tax_id"],
        root: false,
        parent: Some(("posted_invoice_goods_line_id", "posted_invoice_goods_lines")),
        fks: &[
            Fk {
                column: "posted_invoice_goods_line_id",
                target: "posted_invoice_goods_lines",
            },
            Fk {
                column: "tax_id",
                target: "taxes",
            },
        ],
    },
    SheetDef {
        table: "cancelled_invoice_goods_lines",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "cancelled_invoice_id",
            "product_id",
            "product_name",
            "product_reference",
            "product_remarks",
            "hsn_code",
            "variable_types",
            "base_price_formula",
            "sales_price_formula",
            "rate",
            "quantity",
            "variable_values",
            "pre_tax_amount",
            "remarks",
            "journal_entry_item_id",
        ],
        root: false,
        parent: Some(("cancelled_invoice_id", "cancelled_invoices")),
        fks: &[
            Fk {
                column: "cancelled_invoice_id",
                target: "cancelled_invoices",
            },
            Fk {
                column: "product_id",
                target: "products",
            },
            Fk {
                column: "journal_entry_item_id",
                target: "journal_entry_items",
            },
        ],
    },
    SheetDef {
        table: "cancelled_invoice_service_lines",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "cancelled_invoice_id",
            "service_id",
            "service_name",
            "service_reference",
            "service_remarks",
            "sac_code",
            "variable_types",
            "base_price_formula",
            "sales_price_formula",
            "rate",
            "quantity",
            "variable_values",
            "pre_tax_amount",
            "remarks",
            "journal_entry_item_id",
        ],
        root: false,
        parent: Some(("cancelled_invoice_id", "cancelled_invoices")),
        fks: &[
            Fk {
                column: "cancelled_invoice_id",
                target: "cancelled_invoices",
            },
            Fk {
                column: "service_id",
                target: "services",
            },
            Fk {
                column: "journal_entry_item_id",
                target: "journal_entry_items",
            },
        ],
    },
    SheetDef {
        table: "cancelled_invoice_taxes",
        columns: &["cancelled_invoice_id", "tax_id"],
        root: false,
        parent: Some(("cancelled_invoice_id", "cancelled_invoices")),
        fks: &[
            Fk {
                column: "cancelled_invoice_id",
                target: "cancelled_invoices",
            },
            Fk {
                column: "tax_id",
                target: "taxes",
            },
        ],
    },
    SheetDef {
        table: "cancelled_invoice_goods_line_taxes",
        columns: &["cancelled_invoice_goods_line_id", "tax_id"],
        root: false,
        parent: Some((
            "cancelled_invoice_goods_line_id",
            "cancelled_invoice_goods_lines",
        )),
        fks: &[
            Fk {
                column: "cancelled_invoice_goods_line_id",
                target: "cancelled_invoice_goods_lines",
            },
            Fk {
                column: "tax_id",
                target: "taxes",
            },
        ],
    },
    SheetDef {
        table: "draft_payment_terms",
        columns: &["id", "created_at", "updated_at"],
        root: false,
        parent: None,
        fks: &[],
    },
    SheetDef {
        table: "draft_payment_term_lines",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "draft_payment_term_id",
            "line_order",
            "date_kind",
            "due_date",
            "due_duration",
            "amount_kind",
            "amount",
            "amount_percentage",
        ],
        root: false,
        parent: Some(("draft_payment_term_id", "draft_payment_terms")),
        fks: &[Fk {
            column: "draft_payment_term_id",
            target: "draft_payment_terms",
        }],
    },
    SheetDef {
        table: "posted_payment_terms",
        columns: &["id", "created_at", "updated_at"],
        root: false,
        parent: None,
        fks: &[],
    },
    SheetDef {
        table: "posted_payment_term_lines",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "posted_payment_term_id",
            "line_order",
            "due_date",
            "amount",
        ],
        root: false,
        parent: Some(("posted_payment_term_id", "posted_payment_terms")),
        fks: &[Fk {
            column: "posted_payment_term_id",
            target: "posted_payment_terms",
        }],
    },
    SheetDef {
        table: "crm_customers",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "is_individual",
            "company_id",
            "individual_id",
        ],
        root: false,
        parent: None,
        fks: &[
            Fk {
                column: "company_id",
                target: "crm_companies",
            },
            Fk {
                column: "individual_id",
                target: "crm_contacts",
            },
        ],
    },
    SheetDef {
        table: "crm_companies",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "name",
            "address_line_1",
            "address_line_2",
            "city",
            "pincode",
            "state",
            "website",
            "gstin",
            "cin",
            "pan",
            "phone",
            "email",
        ],
        root: false,
        parent: None,
        fks: &[],
    },
    SheetDef {
        table: "crm_contacts",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "company_id",
            "name",
            "email",
            "phone",
            "is_primary",
        ],
        root: false,
        parent: None,
        fks: &[Fk {
            column: "company_id",
            target: "crm_companies",
        }],
    },
    SheetDef {
        table: "products",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "stock_id",
            "reference",
            "remarks",
            "name",
            "hsn_code",
            "variables",
            "base_price_formula",
            "sales_price_formula",
        ],
        root: false,
        parent: None,
        fks: &[],
    },
    SheetDef {
        table: "services",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "reference",
            "remarks",
            "name",
            "sac_code",
            "variables",
            "base_price_formula",
            "sales_price_formula",
        ],
        root: false,
        parent: None,
        fks: &[],
    },
    SheetDef {
        table: "taxes",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "name",
            "percentage",
            "tax_type",
        ],
        root: false,
        parent: None,
        fks: &[],
    },
    SheetDef {
        table: "accounts",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "name",
            "code",
            "is_group",
            "balance_type",
            "parent_id",
        ],
        root: false,
        parent: None,
        fks: &[Fk {
            column: "parent_id",
            target: "accounts",
        }],
    },
    SheetDef {
        table: "journals",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "name",
            "is_active",
            "is_mutable",
            "currency_id",
        ],
        root: false,
        parent: None,
        fks: &[Fk {
            column: "currency_id",
            target: "currencies",
        }],
    },
    SheetDef {
        table: "currencies",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "code",
            "name",
            "symbol",
            "minor_unit",
        ],
        root: false,
        parent: None,
        fks: &[],
    },
    SheetDef {
        table: "journal_entries",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "datetime",
            "source_doc_id",
            "journal_id",
        ],
        root: false,
        parent: None,
        fks: &[
            Fk {
                column: "source_doc_id",
                target: "source_docs",
            },
            Fk {
                column: "journal_id",
                target: "journals",
            },
        ],
    },
    SheetDef {
        table: "journal_entry_items",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "datetime",
            "account_id",
            "amount",
            "journal_entry_id",
        ],
        root: false,
        parent: None,
        fks: &[
            Fk {
                column: "account_id",
                target: "accounts",
            },
            Fk {
                column: "journal_entry_id",
                target: "journal_entries",
            },
        ],
    },
    SheetDef {
        table: "credit_notes",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "datetime",
            "reason",
            "journal_entry_id",
            "reversed_journal_entry_id",
        ],
        root: false,
        parent: None,
        fks: &[
            Fk {
                column: "journal_entry_id",
                target: "journal_entries",
            },
            Fk {
                column: "reversed_journal_entry_id",
                target: "journal_entries",
            },
        ],
    },
    SheetDef {
        table: "payments",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "posted_invoice_id",
            "amount",
            "datetime",
            "journal_entry_id",
            "payment_batch_id",
        ],
        root: false,
        parent: None,
        fks: &[
            Fk {
                column: "posted_invoice_id",
                target: "posted_invoices",
            },
            Fk {
                column: "journal_entry_id",
                target: "journal_entries",
            },
            Fk {
                column: "payment_batch_id",
                target: "payment_batches",
            },
        ],
    },
    SheetDef {
        table: "payment_batches",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "datetime",
            "journal_entry_id",
            "total_amount",
        ],
        root: false,
        parent: None,
        fks: &[Fk {
            column: "journal_entry_id",
            target: "journal_entries",
        }],
    },
    SheetDef {
        table: "source_docs",
        columns: &[
            "id",
            "created_at",
            "updated_at",
            "source_doc_type",
            "source_doc_id",
        ],
        root: false,
        parent: None,
        fks: &[],
    },
];

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read};

    use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
    use zip::ZipArchive;

    use super::build_invoice_workbook;

    #[tokio::test]
    async fn posted_invoice_customer_links_to_customer_sheet() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("sqlite memory");
        for spec in super::SHEETS {
            let cols = spec
                .columns
                .iter()
                .map(|column| format!("{column} TEXT"))
                .collect::<Vec<_>>()
                .join(", ");
            db.execute_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                format!("CREATE TABLE {} ({cols})", spec.table),
            ))
            .await
            .expect("create table");
        }
        db.execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO crm_companies (id, name) VALUES ('3', 'Acme')",
        ))
        .await
        .expect("insert company");
        db.execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO crm_contacts (id, name, company_id) VALUES ('9', 'Ada', '3')",
        ))
        .await
        .expect("insert contact");
        db.execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO crm_customers (id, is_individual, individual_id) VALUES ('7', '1', '9')",
        ))
        .await
        .expect("insert individual customer");
        db.execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO crm_customers (id, is_individual, company_id) VALUES ('8', '0', '3')",
        ))
        .await
        .expect("insert company customer");
        db.execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO products (id, name) VALUES ('4', 'Widget')",
        ))
        .await
        .expect("insert product");
        db.execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO posted_invoices (id, customer_id, number) VALUES ('1', '7', 'INV-1')",
        ))
        .await
        .expect("insert invoice");
        db.execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO draft_payment_terms (id) VALUES ('5')",
        ))
        .await
        .expect("insert payment term");
        db.execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO draft_payment_term_lines \
             (id, draft_payment_term_id, line_order, date_kind, due_duration, amount_kind, amount_percentage) \
             VALUES ('6', '5', '1', 'relative', '1296000000000000', 'relative', '100')",
        ))
        .await
        .expect("insert payment term line");
        db.execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO draft_invoices (id, customer_id, number, draft_payment_term_id) \
             VALUES ('2', '8', 'DRAFT-2', '5')",
        ))
        .await
        .expect("insert draft");
        db.execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO posted_invoice_goods_lines (id, posted_invoice_id, product_id, quantity) \
             VALUES ('10', '1', '4', '2')",
        ))
        .await
        .expect("insert line");

        let bytes = build_invoice_workbook(&db).await.expect("workbook");
        let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("zip");
        let mut xml = String::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).expect("zip entry");
            let mut buf = String::new();
            file.read_to_string(&mut buf).ok();
            xml.push_str(&buf);
        }
        assert!(
            xml.contains("name=\"posted_invoices\""),
            "posted invoices sheet missing"
        );
        assert!(
            xml.contains("name=\"crm_customers\""),
            "customer sheet missing"
        );
        assert!(
            xml.contains("name=\"posted_invoice_goods_lines\""),
            "invoice lines stay on their own sheet"
        );
        assert!(
            xml.contains("name=\"draft_payment_term_lines\""),
            "payment term lines stay on their own sheet"
        );
        assert!(
            xml.contains("location=\"draft_payment_term_lines!A2\"")
                && xml.contains("display=\"2 weeks 1 day · 100%\""),
            "payment term should be a workbook link with a readable duration"
        );
        assert!(
            xml.contains("location=\"posted_invoice_goods_lines!A2\"")
                && xml.contains("display=\"Widget\""),
            "invoice line should be a workbook link on the single invoice row"
        );
        assert!(
            xml.contains("location=\"crm_customers!A2\"") && xml.contains("display=\"Ada, Acme\""),
            "individual customer link should show the contact and company names"
        );
        assert!(
            xml.contains("location=\"crm_customers!A3\"") && xml.contains("display=\"Acme\""),
            "company customer link should show the company name"
        );
        assert!(
            xml.contains("location=\"crm_companies!A2\"") && xml.contains("display=\"Acme\""),
            "company link should show the company name"
        );
        assert!(
            xml.contains("location=\"crm_contacts!A2\"") && xml.contains("display=\"Ada, Acme\""),
            "contact link should show the contact and company names"
        );
        assert!(
            xml.contains("location=\"products!A2\"") && xml.contains("display=\"Widget\""),
            "inlined product link should show the product name"
        );
    }
}
