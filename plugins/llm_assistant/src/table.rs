//! Read tabular VNode bytes with Polars, and spreadsheets with calamine.
//!
//! Polars reads csv, tsv, json, ndjson, parquet, Arrow IPC, and Avro.
//! Calamine reads xls, xlsx, xlsm, xlsb, xla, xlam, and ods. Every path
//! returns a Polars [`DataFrame`] so SQL and row JSON share one shape.

use std::collections::HashSet;
use std::io::Cursor;

use calamine::{
    Data, DataType, Ods, Reader, Sheets, Xls, Xlsb, Xlsx, open_workbook_auto_from_rs,
    open_workbook_from_rs,
};
use polars::io::avro::AvroReader;
use polars::prelude::{
    Column, CsvReadOptions, DataFrame, IntoLazy, IpcReader, IpcStreamReader, JsonFormat,
    JsonReader, JsonWriter, ParquetReader, SerReader, SerWriter,
};
use polars::sql::SQLContext;
use serde_json::{Map, Value, json};

const DEFAULT_N_ROWS: usize = 500;
const MAX_N_ROWS: usize = 10_000;
const FORMATS: &str =
    "csv, tsv, json, ndjson, parquet, ipc, ipc_stream, avro, xls, xlsx, xlsm, xlsb, xla, xlam, ods";

/// Which pre-read check to apply when the VNode path is already known.
#[derive(Clone, Copy)]
pub(crate) enum FileCheck {
    Table,
    Sheets,
}

#[derive(Clone, Copy)]
enum SheetKind {
    Xls,
    Xlsx,
    Xlsb,
    Ods,
    Auto,
}

#[derive(Clone, Copy)]
enum TableKind {
    Csv { separator: u8 },
    Json { sniff: bool },
    Ndjson,
    Parquet,
    Ipc,
    IpcStream,
    Avro,
    Spreadsheet(SheetKind),
}

struct Resolved<'a> {
    kind: TableKind,
    label: &'a str,
}

enum SheetSelect {
    Name(String),
    Index(usize),
}

/// Parsed `read_table` / `query_table` / `list_sheets` arguments, minus the VNode ref.
pub(crate) struct TableRequest {
    format: Option<String>,
    sheet: Option<SheetSelect>,
    n_rows: usize,
    columns: Option<Vec<String>>,
    separator: Option<u8>,
    has_header: Option<bool>,
    skip_rows: usize,
    try_parse_dates: Option<bool>,
    sql: Option<String>,
}

impl TableRequest {
    pub(crate) fn parse(value: &Value, fn_name: &str, sql_required: bool) -> Result<Self, String> {
        let obj = value
            .as_object()
            .ok_or_else(|| format!("{fn_name} requires an object argument"))?;
        let sql = optional_string(obj, "sql")?;
        let sql = match sql {
            Some(sql) if sql_required => Some(sql),
            Some(_) => return Err("sql is only valid for query_table".into()),
            None if sql_required => return Err("sql is required".into()),
            None => None,
        };
        Ok(Self {
            format: optional_string(obj, "format")?,
            sheet: optional_sheet(obj)?,
            n_rows: optional_n_rows(obj)?,
            columns: optional_columns(obj)?,
            separator: optional_separator(obj)?,
            has_header: optional_bool(obj, "has_header")?,
            skip_rows: optional_usize(obj, "skip_rows")?.unwrap_or(0),
            try_parse_dates: optional_bool(obj, "try_parse_dates")?,
            sql,
        })
    }

    /// Reject a file name that these tools cannot read, before opening the VNode.
    pub(crate) fn check(&self, file_name: &str, check: FileCheck) -> Result<(), String> {
        let resolved = self.resolve(file_name)?;
        if matches!(check, FileCheck::Sheets) && !resolved.kind.is_spreadsheet() {
            return Err(
                "list_sheets reads spreadsheet files only (xls, xlsx, xlsm, xlsb, xla, xlam, ods)"
                    .into(),
            );
        }
        if matches!(check, FileCheck::Sheets) && self.sheet.is_some() {
            return Err("list_sheets does not take sheet; it returns every sheet name".into());
        }
        Ok(())
    }

    fn resolve(&self, file_name: &str) -> Result<Resolved<'static>, String> {
        let mut resolved = match &self.format {
            Some(format) => parse_format(format)?,
            None => infer_format(file_name)?,
        };
        self.apply_options(&mut resolved)?;
        Ok(resolved)
    }

    fn apply_options(&self, resolved: &mut Resolved<'static>) -> Result<(), String> {
        if let Some(separator) = self.separator {
            match &mut resolved.kind {
                TableKind::Csv { separator: slot } => *slot = separator,
                _ => return Err("separator is only valid for csv or tsv files".into()),
            }
        }
        if self.has_header.is_some() && !resolved.kind.uses_header() {
            return Err("has_header is only valid for csv, tsv, and spreadsheet files".into());
        }
        if self.try_parse_dates.is_some() && !matches!(resolved.kind, TableKind::Csv { .. }) {
            return Err("try_parse_dates is only valid for csv or tsv files".into());
        }
        if self.sheet.is_some() && !resolved.kind.is_spreadsheet() {
            return Err(
                "sheet is only valid for spreadsheet files (xls, xlsx, xlsm, xlsb, xla, xlam, ods)"
                    .into(),
            );
        }
        Ok(())
    }

    fn has_header(&self) -> bool {
        self.has_header.unwrap_or(true)
    }

    fn try_parse_dates(&self) -> bool {
        self.try_parse_dates.unwrap_or(true)
    }
}

impl TableKind {
    fn is_spreadsheet(&self) -> bool {
        matches!(self, Self::Spreadsheet(_))
    }

    fn uses_header(&self) -> bool {
        matches!(self, Self::Csv { .. } | Self::Spreadsheet(_))
    }
}

pub(crate) fn read_table_bytes(
    file_name: &str,
    bytes: &[u8],
    request: &TableRequest,
) -> Result<Value, String> {
    request.check(file_name, FileCheck::Table)?;
    let resolved = finish_resolve(request.resolve(file_name)?, bytes);
    let (frame, sheet) = load_frame(bytes, &resolved, request, Some(request.n_rows))?;
    let (frame, truncated) = take_rows(frame, request.n_rows);
    frame_json(resolved.label, sheet.as_deref(), frame, truncated)
}

pub(crate) fn query_table_bytes(
    file_name: &str,
    bytes: &[u8],
    request: &TableRequest,
) -> Result<Value, String> {
    request.check(file_name, FileCheck::Table)?;
    let resolved = finish_resolve(request.resolve(file_name)?, bytes);
    let sql = request
        .sql
        .as_deref()
        .filter(|sql| !sql.is_empty())
        .ok_or_else(|| "sql is required".to_string())?;
    let (frame, sheet) = load_frame(bytes, &resolved, request, None)?;
    let frame = run_sql(frame, sql)?;
    let (frame, truncated) = take_rows(frame, request.n_rows);
    frame_json(resolved.label, sheet.as_deref(), frame, truncated)
}

pub(crate) fn list_sheets_bytes(
    file_name: &str,
    bytes: &[u8],
    request: &TableRequest,
) -> Result<Value, String> {
    request.check(file_name, FileCheck::Sheets)?;
    let resolved = request.resolve(file_name)?;
    let TableKind::Spreadsheet(kind) = resolved.kind else {
        return Err(
            "list_sheets reads spreadsheet files only (xls, xlsx, xlsm, xlsb, xla, xlam, ods)"
                .into(),
        );
    };
    let book = open_book(bytes, kind)?;
    let sheets = book.sheet_names();
    Ok(json!({
        "format": resolved.label,
        "sheets": sheets,
    }))
}

fn finish_resolve(mut resolved: Resolved<'static>, bytes: &[u8]) -> Resolved<'static> {
    if matches!(resolved.kind, TableKind::Json { sniff: true }) && !looks_like_json_array(bytes) {
        resolved.kind = TableKind::Ndjson;
        resolved.label = "ndjson";
    }
    resolved
}

fn load_frame(
    bytes: &[u8],
    resolved: &Resolved<'_>,
    request: &TableRequest,
    row_cap: Option<usize>,
) -> Result<(DataFrame, Option<String>), String> {
    let (frame, sheet) = match &resolved.kind {
        TableKind::Csv { separator } => (
            read_csv(bytes, *separator, request, row_cap, resolved.label)?,
            None,
        ),
        TableKind::Json { .. } => (read_json(bytes, JsonFormat::Json, request, row_cap)?, None),
        TableKind::Ndjson => (
            read_json(bytes, JsonFormat::JsonLines, request, row_cap)?,
            None,
        ),
        TableKind::Parquet => (read_parquet(bytes, request, row_cap)?, None),
        TableKind::Ipc => (read_ipc(bytes, request, row_cap)?, None),
        TableKind::IpcStream => (read_ipc_stream(bytes, request, row_cap)?, None),
        TableKind::Avro => (read_avro(bytes, request, row_cap)?, None),
        TableKind::Spreadsheet(kind) => read_spreadsheet(bytes, *kind, request, row_cap)?,
    };
    Ok((project(frame, request.columns.as_deref())?, sheet))
}

fn read_csv(
    bytes: &[u8],
    separator: u8,
    request: &TableRequest,
    row_cap: Option<usize>,
    label: &str,
) -> Result<DataFrame, String> {
    let mut options = CsvReadOptions::default()
        .with_has_header(request.has_header())
        .with_skip_rows(request.skip_rows)
        .map_parse_options(|options| {
            options
                .with_separator(separator)
                .with_try_parse_dates(request.try_parse_dates())
        });
    if let Some(n_rows) = row_cap {
        options = options.with_n_rows(Some(n_rows.saturating_add(1)));
    }
    options
        .into_reader_with_file_handle(Cursor::new(bytes))
        .finish()
        .map_err(|e| format!("read {label}: {e}"))
}

fn read_json(
    bytes: &[u8],
    format: JsonFormat,
    request: &TableRequest,
    row_cap: Option<usize>,
) -> Result<DataFrame, String> {
    let frame = JsonReader::new(Cursor::new(bytes))
        .with_json_format(format)
        .finish()
        .map_err(|e| format!("read json: {e}"))?;
    window(frame, request.skip_rows, row_cap)
}

fn read_parquet(
    bytes: &[u8],
    request: &TableRequest,
    row_cap: Option<usize>,
) -> Result<DataFrame, String> {
    let mut reader = ParquetReader::new(Cursor::new(bytes));
    if let Some(n_rows) = row_cap {
        reader = reader.with_slice(Some((request.skip_rows, n_rows.saturating_add(1))));
        return reader.finish().map_err(|e| format!("read parquet: {e}"));
    }
    let frame = reader.finish().map_err(|e| format!("read parquet: {e}"))?;
    window(frame, request.skip_rows, None)
}

fn read_ipc(
    bytes: &[u8],
    request: &TableRequest,
    row_cap: Option<usize>,
) -> Result<DataFrame, String> {
    let mut reader = IpcReader::new(Cursor::new(bytes));
    if let Some(fetch) = fetch_len(request.skip_rows, row_cap)? {
        reader = reader.with_n_rows(Some(fetch));
    }
    let frame = reader.finish().map_err(|e| format!("read ipc: {e}"))?;
    window(frame, request.skip_rows, None)
}

fn read_ipc_stream(
    bytes: &[u8],
    request: &TableRequest,
    row_cap: Option<usize>,
) -> Result<DataFrame, String> {
    let mut reader = IpcStreamReader::new(Cursor::new(bytes));
    if let Some(fetch) = fetch_len(request.skip_rows, row_cap)? {
        reader = reader.with_n_rows(Some(fetch));
    }
    let frame = reader
        .finish()
        .map_err(|e| format!("read ipc_stream: {e}"))?;
    window(frame, request.skip_rows, None)
}

fn read_avro(
    bytes: &[u8],
    request: &TableRequest,
    row_cap: Option<usize>,
) -> Result<DataFrame, String> {
    let mut reader = AvroReader::new(Cursor::new(bytes));
    if let Some(fetch) = fetch_len(request.skip_rows, row_cap)? {
        reader = reader.with_n_rows(Some(fetch));
    }
    let frame = reader.finish().map_err(|e| format!("read avro: {e}"))?;
    window(frame, request.skip_rows, None)
}

fn read_spreadsheet(
    bytes: &[u8],
    kind: SheetKind,
    request: &TableRequest,
    row_cap: Option<usize>,
) -> Result<(DataFrame, Option<String>), String> {
    let mut book = open_book(bytes, kind)?;
    let names = book.sheet_names();
    let sheet = select_sheet(&names, request.sheet.as_ref())?;
    let range = book
        .worksheet_range(&sheet)
        .map_err(|e| format!("read sheet {sheet}: {e}"))?;
    let frame = frame_from_range(&range, request.has_header(), request.skip_rows, row_cap)?;
    Ok((frame, Some(sheet)))
}

fn open_book(bytes: &[u8], kind: SheetKind) -> Result<Sheets<Cursor<Vec<u8>>>, String> {
    let cursor = Cursor::new(bytes.to_vec());
    match kind {
        SheetKind::Xls => open_workbook_from_rs::<Xls<_>, _>(cursor)
            .map(Sheets::Xls)
            .map_err(|e| format!("open spreadsheet: {e}")),
        SheetKind::Xlsx => open_workbook_from_rs::<Xlsx<_>, _>(cursor)
            .map(Sheets::Xlsx)
            .map_err(|e| format!("open spreadsheet: {e}")),
        SheetKind::Xlsb => open_workbook_from_rs::<Xlsb<_>, _>(cursor)
            .map(Sheets::Xlsb)
            .map_err(|e| format!("open spreadsheet: {e}")),
        SheetKind::Ods => open_workbook_from_rs::<Ods<_>, _>(cursor)
            .map(Sheets::Ods)
            .map_err(|e| format!("open spreadsheet: {e}")),
        SheetKind::Auto => {
            open_workbook_auto_from_rs(cursor).map_err(|e| format!("open spreadsheet: {e}"))
        }
    }
}

fn select_sheet(names: &[String], sheet: Option<&SheetSelect>) -> Result<String, String> {
    match sheet {
        None => names
            .first()
            .cloned()
            .ok_or_else(|| "workbook has no sheets".to_string()),
        Some(SheetSelect::Name(name)) => {
            if names.iter().any(|existing| existing == name) {
                Ok(name.clone())
            } else {
                Err(format!(
                    "sheet \"{name}\" not found; sheets: {}",
                    names.join(", ")
                ))
            }
        }
        Some(SheetSelect::Index(idx)) => names.get(*idx).cloned().ok_or_else(|| {
            format!(
                "sheet index {idx} is out of range; workbook has {} sheets",
                names.len()
            )
        }),
    }
}

fn frame_from_range(
    range: &calamine::Range<Data>,
    has_header: bool,
    skip_rows: usize,
    row_cap: Option<usize>,
) -> Result<DataFrame, String> {
    let all = range.rows().collect::<Vec<_>>();
    let Some(rows) = all.get(skip_rows..) else {
        return Err(format!(
            "skip_rows {skip_rows} is past the end of the sheet"
        ));
    };
    let (headers, data) = if has_header {
        let Some((header, rest)) = rows.split_first() else {
            return Err("sheet has no header row".into());
        };
        (header_names(header), rest)
    } else {
        let width = rows.iter().map(|row| row.len()).max().unwrap_or(0);
        let names = (0..width).map(|idx| format!("column_{idx}")).collect();
        (names, rows)
    };
    let data = match row_cap {
        Some(n_rows) => data.get(..n_rows.saturating_add(1)).unwrap_or(data),
        None => data,
    };
    cells_to_frame(headers, data)
}

fn header_names(row: &[Data]) -> Vec<String> {
    row.iter()
        .enumerate()
        .map(|(idx, cell)| match cell_string(cell) {
            Some(name) if !name.trim().is_empty() => name.trim().to_string(),
            _ => format!("column_{idx}"),
        })
        .collect()
}

fn cells_to_frame(mut headers: Vec<String>, rows: &[&[Data]]) -> Result<DataFrame, String> {
    let width = headers
        .len()
        .max(rows.iter().map(|row| row.len()).max().unwrap_or(0));
    while headers.len() < width {
        let idx = headers.len();
        headers.push(format!("column_{idx}"));
    }
    if headers.is_empty() {
        return Ok(DataFrame::empty());
    }
    let headers = unique_headers(headers);
    let mut columns = Vec::with_capacity(headers.len());
    for (idx, name) in headers.iter().enumerate() {
        let kind = rows.iter().fold(ColKind::Empty, |kind, row| {
            merge_kind(kind, cell_kind(cell_at(row, idx)))
        });
        columns.push(column_from_cells(name, kind, idx, rows)?);
    }
    DataFrame::new_infer_height(columns).map_err(|e| format!("build sheet: {e}"))
}

fn unique_headers(names: Vec<String>) -> Vec<String> {
    let mut used = HashSet::new();
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let mut candidate = name.clone();
        let mut suffix = 2usize;
        while !used.insert(candidate.clone()) {
            candidate = format!("{name}_{suffix}");
            suffix = suffix.saturating_add(1);
        }
        out.push(candidate);
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ColKind {
    Empty,
    Int,
    Float,
    Bool,
    Str,
}

fn merge_kind(left: ColKind, right: ColKind) -> ColKind {
    use ColKind::{Bool, Empty, Float, Int, Str};
    match (left, right) {
        (Empty, other) | (other, Empty) => other,
        (Int, Int) => Int,
        (Int, Float) | (Float, Int) | (Float, Float) => Float,
        (Bool, Bool) => Bool,
        _ => Str,
    }
}

fn cell_kind(cell: &Data) -> ColKind {
    match cell {
        Data::Empty => ColKind::Empty,
        Data::Int(_) => ColKind::Int,
        Data::Float(_) => ColKind::Float,
        Data::Bool(_) => ColKind::Bool,
        _ => ColKind::Str,
    }
}

fn cell_at(row: &[Data], idx: usize) -> &Data {
    row.get(idx).unwrap_or(&Data::Empty)
}

fn column_from_cells(
    name: &str,
    kind: ColKind,
    idx: usize,
    rows: &[&[Data]],
) -> Result<Column, String> {
    match kind {
        ColKind::Int => {
            let mut values = Vec::with_capacity(rows.len());
            for row in rows {
                match cell_at(row, idx) {
                    Data::Empty => values.push(None),
                    Data::Int(value) => values.push(Some(*value)),
                    _ => return Err(format!("column \"{name}\" is not an integer column")),
                }
            }
            Ok(Column::new(name.into(), values))
        }
        ColKind::Float => {
            let mut values = Vec::with_capacity(rows.len());
            for row in rows {
                match cell_at(row, idx) {
                    Data::Empty => values.push(None),
                    Data::Float(value) => values.push(Some(*value)),
                    Data::Int(value) => values.push(Some(*value as f64)),
                    _ => return Err(format!("column \"{name}\" is not a float column")),
                }
            }
            Ok(Column::new(name.into(), values))
        }
        ColKind::Bool => {
            let mut values = Vec::with_capacity(rows.len());
            for row in rows {
                match cell_at(row, idx) {
                    Data::Empty => values.push(None),
                    Data::Bool(value) => values.push(Some(*value)),
                    _ => return Err(format!("column \"{name}\" is not a boolean column")),
                }
            }
            Ok(Column::new(name.into(), values))
        }
        ColKind::Empty | ColKind::Str => {
            let values = rows
                .iter()
                .map(|row| cell_string(cell_at(row, idx)))
                .collect::<Vec<_>>();
            Ok(Column::new(name.into(), values))
        }
    }
}

fn cell_string(cell: &Data) -> Option<String> {
    match cell {
        Data::Empty => None,
        Data::String(value) => Some(value.clone()),
        Data::Int(value) => Some(value.to_string()),
        Data::Float(value) => Some(value.to_string()),
        Data::Bool(value) => Some(value.to_string()),
        Data::DateTime(_) | Data::DateTimeIso(_) => Some(
            cell.as_datetime()
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
                .unwrap_or_else(|| cell.to_string()),
        ),
        Data::DurationIso(value) => Some(value.clone()),
        Data::Error(err) => Some(format!("{err:?}")),
    }
}

fn project(frame: DataFrame, columns: Option<&[String]>) -> Result<DataFrame, String> {
    let Some(columns) = columns else {
        return Ok(frame);
    };
    frame
        .select(columns.iter().map(String::as_str))
        .map_err(|e| format!("columns: {e}"))
}

fn window(frame: DataFrame, skip_rows: usize, row_cap: Option<usize>) -> Result<DataFrame, String> {
    let frame = skip_frame(frame, skip_rows)?;
    Ok(match row_cap {
        Some(n_rows) => frame.head(Some(n_rows.saturating_add(1))),
        None => frame,
    })
}

fn skip_frame(frame: DataFrame, skip_rows: usize) -> Result<DataFrame, String> {
    if skip_rows == 0 {
        return Ok(frame);
    }
    let offset = i64::try_from(skip_rows).map_err(|_err| "skip_rows is too large".to_string())?;
    Ok(frame.slice(offset, usize::MAX))
}

fn fetch_len(skip_rows: usize, row_cap: Option<usize>) -> Result<Option<usize>, String> {
    let Some(n_rows) = row_cap else {
        return Ok(None);
    };
    skip_rows
        .checked_add(n_rows)
        .and_then(|len| len.checked_add(1))
        .map(Some)
        .ok_or_else(|| "n_rows is too large".to_string())
}

fn take_rows(frame: DataFrame, n_rows: usize) -> (DataFrame, bool) {
    if frame.height() > n_rows {
        (frame.head(Some(n_rows)), true)
    } else {
        (frame, false)
    }
}

fn run_sql(frame: DataFrame, sql: &str) -> Result<DataFrame, String> {
    let mut ctx = SQLContext::new();
    ctx.register("data", frame.lazy());
    ctx.execute(sql)
        .and_then(|frame| frame.collect())
        .map_err(|e| format!("sql: {e}"))
}

fn frame_json(
    format: &str,
    sheet: Option<&str>,
    mut frame: DataFrame,
    truncated: bool,
) -> Result<Value, String> {
    let columns = frame
        .columns()
        .iter()
        .map(|column| {
            json!({
                "name": column.name().as_str(),
                "dtype": column.dtype().to_string(),
            })
        })
        .collect::<Vec<_>>();
    let height = frame.height();
    let width = frame.width();
    let rows = if height == 0 {
        Vec::new()
    } else {
        let mut buf = Vec::new();
        JsonWriter::new(&mut buf)
            .with_json_format(JsonFormat::Json)
            .finish(&mut frame)
            .map_err(|e| format!("serialize table: {e}"))?;
        match serde_json::from_slice::<Value>(&buf).map_err(|e| format!("serialize table: {e}"))? {
            Value::Array(rows) => rows,
            other => {
                return Err(format!(
                    "serialize table: expected a JSON array, got {other}"
                ));
            }
        }
    };
    let sheet = match sheet {
        Some(name) => Value::String(name.to_string()),
        None => Value::Null,
    };
    Ok(json!({
        "format": format,
        "sheet": sheet,
        "columns": columns,
        "height": height,
        "width": width,
        "truncated": truncated,
        "rows": rows,
    }))
}

fn infer_format(file_name: &str) -> Result<Resolved<'static>, String> {
    match file_extension(file_name).as_str() {
        "csv" => Ok(resolved(TableKind::Csv { separator: b',' }, "csv")),
        "tsv" | "tab" => Ok(resolved(TableKind::Csv { separator: b'\t' }, "tsv")),
        "json" => Ok(resolved(TableKind::Json { sniff: true }, "json")),
        "jsonl" | "ndjson" => Ok(resolved(TableKind::Ndjson, "ndjson")),
        "parquet" | "pq" => Ok(resolved(TableKind::Parquet, "parquet")),
        "ipc" | "arrow" | "feather" => Ok(resolved(TableKind::Ipc, "ipc")),
        "arrows" | "ipcstream" => Ok(resolved(TableKind::IpcStream, "ipc_stream")),
        "avro" => Ok(resolved(TableKind::Avro, "avro")),
        "xls" => Ok(sheet_format(SheetKind::Xls, "xls")),
        "xla" => Ok(sheet_format(SheetKind::Xls, "xla")),
        "xlsx" => Ok(sheet_format(SheetKind::Xlsx, "xlsx")),
        "xlsm" => Ok(sheet_format(SheetKind::Xlsx, "xlsm")),
        "xlam" => Ok(sheet_format(SheetKind::Xlsx, "xlam")),
        "xlsb" => Ok(sheet_format(SheetKind::Xlsb, "xlsb")),
        "ods" => Ok(sheet_format(SheetKind::Ods, "ods")),
        _ => Err(format!(
            "cannot infer a table format from \"{file_name}\"; set format to one of {FORMATS}"
        )),
    }
}

fn parse_format(format: &str) -> Result<Resolved<'static>, String> {
    match format {
        "csv" => Ok(resolved(TableKind::Csv { separator: b',' }, "csv")),
        "tsv" | "tab" => Ok(resolved(TableKind::Csv { separator: b'\t' }, "tsv")),
        "json" => Ok(resolved(TableKind::Json { sniff: false }, "json")),
        "ndjson" | "jsonl" => Ok(resolved(TableKind::Ndjson, "ndjson")),
        "parquet" | "pq" => Ok(resolved(TableKind::Parquet, "parquet")),
        "ipc" | "arrow" | "feather" => Ok(resolved(TableKind::Ipc, "ipc")),
        "ipc_stream" | "ipc-stream" | "arrows" => Ok(resolved(TableKind::IpcStream, "ipc_stream")),
        "avro" => Ok(resolved(TableKind::Avro, "avro")),
        "xls" => Ok(sheet_format(SheetKind::Xls, "xls")),
        "xla" => Ok(sheet_format(SheetKind::Xls, "xla")),
        "xlsx" => Ok(sheet_format(SheetKind::Xlsx, "xlsx")),
        "xlsm" => Ok(sheet_format(SheetKind::Xlsx, "xlsm")),
        "xlam" => Ok(sheet_format(SheetKind::Xlsx, "xlam")),
        "xlsb" => Ok(sheet_format(SheetKind::Xlsb, "xlsb")),
        "ods" => Ok(sheet_format(SheetKind::Ods, "ods")),
        "excel" => Ok(sheet_format(SheetKind::Auto, "excel")),
        other => Err(format!(
            "unknown format \"{other}\"; expected one of {FORMATS}"
        )),
    }
}

fn resolved(kind: TableKind, label: &'static str) -> Resolved<'static> {
    Resolved { kind, label }
}

fn sheet_format(kind: SheetKind, label: &'static str) -> Resolved<'static> {
    resolved(TableKind::Spreadsheet(kind), label)
}

fn file_extension(file_name: &str) -> String {
    let base = file_name
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or(file_name);
    let lower = base.to_ascii_lowercase();
    let stripped = strip_compression(&lower);
    match stripped.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => ext.to_string(),
        _ => String::new(),
    }
}

fn strip_compression(lower: &str) -> &str {
    for suffix in [".gz", ".gzip", ".zst", ".zstd", ".zlib"] {
        if let Some(stripped) = lower.strip_suffix(suffix) {
            return stripped;
        }
    }
    lower
}

fn looks_like_json_array(bytes: &[u8]) -> bool {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    bytes
        .iter()
        .find(|byte| !byte.is_ascii_whitespace())
        .copied()
        == Some(b'[')
}

fn optional_string(obj: &Map<String, Value>, field: &str) -> Result<Option<String>, String> {
    match obj.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => {
            let value = value.trim();
            if value.is_empty() {
                Err(format!("{field} must not be empty"))
            } else if field == "format" {
                Ok(Some(value.to_ascii_lowercase()))
            } else {
                Ok(Some(value.to_string()))
            }
        }
        Some(_) => Err(format!("{field} must be a string")),
    }
}

fn optional_bool(obj: &Map<String, Value>, field: &str) -> Result<Option<bool>, String> {
    match obj.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(format!("{field} must be a bool")),
    }
}

fn optional_usize(obj: &Map<String, Value>, field: &str) -> Result<Option<usize>, String> {
    match obj.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => Ok(Some(parse_usize(value, field)?)),
    }
}

fn optional_n_rows(obj: &Map<String, Value>) -> Result<usize, String> {
    match optional_usize(obj, "n_rows")? {
        None => Ok(DEFAULT_N_ROWS),
        Some(n_rows) if n_rows > MAX_N_ROWS => Err(format!("n_rows cannot exceed {MAX_N_ROWS}")),
        Some(n_rows) => Ok(n_rows),
    }
}

fn optional_separator(obj: &Map<String, Value>) -> Result<Option<u8>, String> {
    match optional_string(obj, "separator")? {
        None => Ok(None),
        Some(value) => parse_separator(&value).map(Some),
    }
}

fn parse_separator(value: &str) -> Result<u8, String> {
    match value {
        "\\t" | "tab" => Ok(b'\t'),
        other => {
            let mut chars = other.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => u8::try_from(u32::from(ch))
                    .map_err(|_err| "separator must be a single ASCII character".to_string()),
                _ => Err("separator must be a single character".into()),
            }
        }
    }
}

fn optional_columns(obj: &Map<String, Value>) -> Result<Option<Vec<String>>, String> {
    match obj.get("columns") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) if items.is_empty() => Err("columns must not be empty".into()),
        Some(Value::Array(items)) => {
            let mut columns = Vec::with_capacity(items.len());
            for item in items {
                let Some(name) = item.as_str().map(str::trim).filter(|name| !name.is_empty())
                else {
                    return Err("columns must be non-empty strings".into());
                };
                columns.push(name.to_string());
            }
            Ok(Some(columns))
        }
        Some(_) => Err("columns must be a list of strings".into()),
    }
}

fn optional_sheet(obj: &Map<String, Value>) -> Result<Option<SheetSelect>, String> {
    match obj.get("sheet") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(name)) => {
            let name = name.trim();
            if name.is_empty() {
                Err("sheet name must not be empty".into())
            } else {
                Ok(Some(SheetSelect::Name(name.to_string())))
            }
        }
        Some(Value::Number(index)) => {
            let index = parse_usize(&Value::Number(index.clone()), "sheet")?;
            Ok(Some(SheetSelect::Index(index)))
        }
        Some(_) => Err("sheet must be a name or a 0-based index".into()),
    }
}

fn parse_usize(value: &Value, field: &str) -> Result<usize, String> {
    let Some(number) = value.as_u64() else {
        if value.as_i64().is_some() {
            return Err(format!("{field} must be a non-negative integer"));
        }
        return Err(format!("{field} must be an integer"));
    };
    usize::try_from(number).map_err(|_err| format!("{field} is too large"))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;
    use std::sync::Arc;

    use polars::prelude::{IpcWriter, ParquetWriter};
    use sea_orm::{ConnectionTrait, Database, Schema};
    use serde_json::json;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use crate::rune_engine;
    use crate::rune_env::Hook;
    use lariv_core::rune_env::{RuneEnvCapability, RuneEnvCtx, RuneEnvRegistrar};
    use lariv_plugin_filesystem::entities::filesystem_node;
    use lariv_plugin_filesystem::node::{self, NodeFile};
    use lariv_plugin_filesystem::storage::{DynFilestore, LocalFilestore};

    fn request(value: Value, sql_required: bool) -> TableRequest {
        TableRequest::parse(&value, "table", sql_required).expect("request")
    }

    fn read(name: &str, bytes: &[u8], value: Value) -> Value {
        read_table_bytes(name, bytes, &request(value, false)).expect("read")
    }

    fn cell<'a>(out: &'a Value, row: usize, column: &str) -> &'a Value {
        out.get("rows")
            .and_then(Value::as_array)
            .and_then(|rows| rows.get(row))
            .and_then(|row| row.get(column))
            .expect("cell")
    }

    fn column_name(out: &Value, idx: usize) -> &str {
        out.get("columns")
            .and_then(Value::as_array)
            .and_then(|columns| columns.get(idx))
            .and_then(|column| column.get("name"))
            .and_then(Value::as_str)
            .expect("column")
    }

    #[test]
    fn reads_csv_and_limits_rows() {
        let bytes = b"name,qty\nbolt,3\nnut,5\n";
        let out = read("parts.csv", bytes, json!({}));
        assert_eq!(out.get("format").and_then(Value::as_str), Some("csv"));
        assert_eq!(out.get("height").and_then(Value::as_u64), Some(2));
        assert_eq!(cell(&out, 0, "name").as_str(), Some("bolt"));
        assert_eq!(cell(&out, 1, "qty").as_i64(), Some(5));

        let limited = read("parts.csv", bytes, json!({"n_rows": 1}));
        assert_eq!(limited.get("height").and_then(Value::as_u64), Some(1));
        assert_eq!(
            limited.get("truncated").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(cell(&limited, 0, "qty").as_i64(), Some(3));
    }

    #[test]
    fn queries_csv_with_polars_sql() {
        let bytes = b"name,qty\nbolt,3\nnut,5\n";
        let out = query_table_bytes(
            "parts.csv",
            bytes,
            &request(json!({"sql": "SELECT sum(qty) AS total FROM data"}), true),
        )
        .expect("query");
        assert_eq!(cell(&out, 0, "total").as_i64(), Some(8));
    }

    #[test]
    fn reads_tsv_json_and_ndjson() {
        let tsv = read("parts.tsv", b"name\tqty\nbolt\t3\n", json!({}));
        assert_eq!(tsv.get("format").and_then(Value::as_str), Some("tsv"));
        assert_eq!(cell(&tsv, 0, "qty").as_i64(), Some(3));

        let json_rows = read("parts.json", br#"[{"name":"bolt","qty":3}]"#, json!({}));
        assert_eq!(
            json_rows.get("format").and_then(Value::as_str),
            Some("json")
        );
        assert_eq!(cell(&json_rows, 0, "name").as_str(), Some("bolt"));

        let lines = read(
            "parts.json",
            b"{\"name\":\"bolt\",\"qty\":3}\n{\"name\":\"nut\",\"qty\":5}\n",
            json!({}),
        );
        assert_eq!(lines.get("format").and_then(Value::as_str), Some("ndjson"));
        assert_eq!(lines.get("height").and_then(Value::as_u64), Some(2));
    }

    #[test]
    fn roundtrips_parquet_ipc_and_avro() {
        let mut frame = polars::df!("name" => ["bolt"], "qty" => [3i64]).expect("frame");
        let mut parquet = Vec::new();
        ParquetWriter::new(&mut parquet)
            .finish(&mut frame)
            .expect("parquet");
        let out = read("parts.parquet", &parquet, json!({}));
        assert_eq!(out.get("format").and_then(Value::as_str), Some("parquet"));
        assert_eq!(cell(&out, 0, "name").as_str(), Some("bolt"));

        let mut frame = polars::df!("name" => ["bolt"], "qty" => [3i64]).expect("frame");
        let mut ipc = Vec::new();
        IpcWriter::new(&mut ipc).finish(&mut frame).expect("ipc");
        let out = read("parts.arrow", &ipc, json!({}));
        assert_eq!(out.get("format").and_then(Value::as_str), Some("ipc"));
        assert_eq!(cell(&out, 0, "qty").as_i64(), Some(3));

        let mut frame = polars::df!("name" => ["bolt"], "qty" => [3i64]).expect("frame");
        let mut avro = Vec::new();
        polars::io::avro::AvroWriter::new(&mut avro)
            .finish(&mut frame)
            .expect("avro");
        let out = read("parts.avro", &avro, json!({}));
        assert_eq!(out.get("format").and_then(Value::as_str), Some("avro"));
        assert_eq!(cell(&out, 0, "name").as_str(), Some("bolt"));
    }

    #[test]
    fn reads_xlsx_xlam_and_ods_sheets() {
        let bytes = xlsx_bytes();
        let sheets =
            list_sheets_bytes("parts.xlsx", &bytes, &request(json!({}), false)).expect("sheets");
        let names = sheets
            .get("sheets")
            .and_then(Value::as_array)
            .expect("sheet list");
        assert_eq!(names.len(), 2);
        assert_eq!(names.first().and_then(Value::as_str), Some("Parts"));

        let out = read("parts.xlsx", &bytes, json!({"sheet": "Parts"}));
        assert_eq!(out.get("format").and_then(Value::as_str), Some("xlsx"));
        assert_eq!(out.get("sheet").and_then(Value::as_str), Some("Parts"));
        assert_eq!(cell(&out, 0, "name").as_str(), Some("bolt"));
        let qty = cell(&out, 0, "qty").as_f64().expect("qty");
        assert!(qty > 2.9 && qty < 3.1, "{qty}");

        let second = read("addin.xlam", &bytes, json!({"sheet": 1}));
        assert_eq!(second.get("format").and_then(Value::as_str), Some("xlam"));
        assert_eq!(column_name(&second, 0), "only");

        let ods = read("parts.ods", &ods_bytes(), json!({}));
        assert_eq!(ods.get("format").and_then(Value::as_str), Some("ods"));
        assert_eq!(cell(&ods, 0, "name").as_str(), Some("bolt"));
        let qty = cell(&ods, 0, "qty").as_f64().expect("ods qty");
        assert!(qty > 2.9 && qty < 3.1, "{qty}");
    }

    #[test]
    fn rejects_an_unknown_extension() {
        let err = read_table_bytes("notes.bin", b"name\n", &request(json!({}), false))
            .expect_err("format");
        assert!(err.contains("cannot infer"), "{err}");
    }

    fn xlsx_bytes() -> Vec<u8> {
        use rust_xlsxwriter::Workbook;

        let mut workbook = Workbook::new();
        let parts = workbook.add_worksheet();
        parts.set_name("Parts").expect("sheet name");
        parts.write_string(0, 0, "name").expect("header");
        parts.write_string(0, 1, "qty").expect("header");
        parts.write_string(1, 0, "bolt").expect("name");
        parts.write_number(1, 1, 3).expect("qty");
        let other = workbook.add_worksheet();
        other.set_name("Other").expect("sheet name");
        other.write_string(0, 0, "only").expect("header");
        other.write_string(1, 0, "x").expect("value");
        workbook.save_to_buffer().expect("xlsx")
    }

    fn ods_bytes() -> Vec<u8> {
        let content = concat!(
            r#"<?xml version="1.0" encoding="UTF-8"?>"#,
            r#"<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" "#,
            r#"xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" "#,
            r#"xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">"#,
            "<office:body><office:spreadsheet>",
            r#"<table:table table:name="Sheet1">"#,
            "<table:table-row>",
            r#"<table:table-cell office:value-type="string"><text:p>name</text:p></table:table-cell>"#,
            r#"<table:table-cell office:value-type="string"><text:p>qty</text:p></table:table-cell>"#,
            "</table:table-row>",
            "<table:table-row>",
            r#"<table:table-cell office:value-type="string"><text:p>bolt</text:p></table:table-cell>"#,
            r#"<table:table-cell office:value-type="float" office:value="3"></table:table-cell>"#,
            "</table:table-row>",
            "</table:table></office:spreadsheet></office:body></office:document-content>"
        );
        let mut buf = Vec::new();
        {
            let mut writer = ZipWriter::new(Cursor::new(&mut buf));
            let stored =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            writer.start_file("mimetype", stored).expect("mimetype");
            writer
                .write_all(b"application/vnd.oasis.opendocument.spreadsheet")
                .expect("mimetype bytes");
            writer.start_file("content.xml", stored).expect("content");
            writer.write_all(content.as_bytes()).expect("content bytes");
            let manifest = concat!(
                r#"<?xml version="1.0" encoding="UTF-8"?>"#,
                r#"<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0">"#,
                r#"<manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.spreadsheet"/>"#,
                r#"<manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>"#,
                "</manifest:manifest>"
            );
            writer
                .start_file("META-INF/manifest.xml", stored)
                .expect("manifest");
            writer
                .write_all(manifest.as_bytes())
                .expect("manifest bytes");
            writer.finish().expect("ods");
        }
        buf
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn read_table_reads_a_vnode() {
        let db = Database::connect("sqlite::memory:").await.expect("sqlite");
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        db.execute(&schema.create_table_from_entity(filesystem_node::Entity))
            .await
            .expect("table");
        let dir = std::env::temp_dir().join(format!("lariv-table-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let store: Arc<DynFilestore> = Arc::new(LocalFilestore::new(dir.to_string_lossy()));
        node::create(
            &db,
            store.as_ref(),
            "parts.csv".into(),
            false,
            Some(NodeFile::Bytes {
                filename: "parts.csv".into(),
                data: b"name,qty\nbolt,3\n".to_vec(),
            }),
            None,
            None,
            None,
        )
        .await
        .expect("vnode");

        let mut cap = RuneEnvCapability::new();
        Hook.register_rune_env(&mut cap);
        let env_ctx = RuneEnvCtx {
            db: &db,
            store: lariv_plugin_filesystem::storage::erase_store(Arc::clone(&store)),
            session_id: None,
        };
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"read_table(#{ path: "/parts.csv", n_rows: 10 })"#,
            &[],
        )
        .await;
        let result = out.get("result").expect("result");
        assert_eq!(cell(result, 0, "name").as_str(), Some("bolt"));
        assert_eq!(cell(result, 0, "qty").as_i64(), Some(3));
    }
}
