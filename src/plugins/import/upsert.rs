//! Catalog-driven INSERT ON CONFLICT import.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, QueryResult, Statement, TransactionTrait, Value};

use crate::export::{ExportCatalog, ExportTable};

use super::xlsx::ParsedWorkbook;

/// Per-table upsert counts.
#[derive(Clone, Debug)]
pub struct TableImportResult {
    pub table: String,
    pub inserted: u64,
    pub updated: u64,
}

/// Result shown after a successful import.
#[derive(Clone, Debug)]
pub struct ImportReport {
    pub tables: Vec<TableImportResult>,
    pub skipped_sheets: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ColKind {
    Text,
    Int,
    Float,
    Bool,
    Bytes,
    Date,
    Timestamp,
    /// Postgres enum (udt name, e.g. `client_status`).
    Enum(String),
    Json,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ColMeta {
    kind: ColKind,
    nullable: bool,
}

const DEFAULT_COL_META: ColMeta = ColMeta {
    kind: ColKind::Text,
    nullable: true,
};

/// Upsert all parsed sheets inside one transaction.
pub async fn import_workbook<C>(
    db: &C,
    catalog: &ExportCatalog,
    workbook: &ParsedWorkbook,
) -> Result<ImportReport, String>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db
        .begin()
        .await
        .map_err(|e| format!("begin transaction: {e}"))?;
    let mut tables = Vec::new();
    for sheet in &workbook.sheets {
        let entry = catalog
            .entry(&sheet.table)
            .ok_or_else(|| format!("missing catalog entry for {}", sheet.table))?;
        let result = import_sheet(&txn, entry, &sheet.columns, &sheet.rows).await;
        match result {
            Ok(result) => tables.push(result),
            Err(err) => {
                txn.rollback()
                    .await
                    .map_err(|e| format!("rollback after {err}: {e}"))?;
                return Err(err);
            }
        }
    }
    txn.commit()
        .await
        .map_err(|e| format!("commit transaction: {e}"))?;
    Ok(ImportReport {
        tables,
        skipped_sheets: workbook.skipped_sheets.clone(),
    })
}

async fn import_sheet<C: ConnectionTrait>(
    db: &C,
    entry: &ExportTable,
    columns: &[String],
    rows: &[Vec<String>],
) -> Result<TableImportResult, String> {
    let types = load_column_types(db, &entry.table).await?;
    let pks = &entry.primary_keys;
    for pk in pks {
        if !columns.iter().any(|c| c == pk) {
            return Err(format!(
                "sheet {} is missing primary key column {pk}",
                entry.table
            ));
        }
    }

    let non_pk: Vec<String> = columns
        .iter()
        .filter(|c| !pks.iter().any(|pk| pk == *c))
        .cloned()
        .collect();

    let mut inserted = 0u64;
    let mut updated = 0u64;
    for (row_idx, row) in rows.iter().enumerate() {
        let values = coerce_row(entry, columns, row, &types, row_idx)?;
        let existed = row_exists(db, entry, columns, &values).await?;
        upsert_row(db, entry, columns, &non_pk, &values, &types).await?;
        if existed {
            updated = updated.saturating_add(1);
        } else {
            inserted = inserted.saturating_add(1);
        }
    }

    reset_sequence(db, entry, &types).await?;

    Ok(TableImportResult {
        table: entry.table.clone(),
        inserted,
        updated,
    })
}

fn coerce_row(
    entry: &ExportTable,
    columns: &[String],
    row: &[String],
    types: &HashMap<String, ColMeta>,
    row_idx: usize,
) -> Result<Vec<Value>, String> {
    let mut out = Vec::with_capacity(columns.len());
    for (i, col) in columns.iter().enumerate() {
        let raw = row.get(i).map(String::as_str).unwrap_or("");
        if entry.primary_keys.iter().any(|pk| pk == col) && raw.trim().is_empty() {
            return Err(format!(
                "{} row {} missing primary key {col}",
                entry.table,
                row_idx.saturating_add(1)
            ));
        }
        let meta = types.get(col).cloned().unwrap_or(DEFAULT_COL_META);
        out.push(coerce_value(&meta, raw).map_err(|e| {
            format!(
                "{} row {} column {col}: {e}",
                entry.table,
                row_idx.saturating_add(1)
            )
        })?);
    }
    Ok(out)
}

fn coerce_value(meta: &ColMeta, raw: &str) -> Result<Value, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(empty_of(meta));
    }
    match &meta.kind {
        ColKind::Text | ColKind::Enum(_) | ColKind::Json => {
            Ok(Value::String(Some(Box::new(trimmed.to_string()))))
        }
        ColKind::Int => {
            if let Ok(n) = trimmed.parse::<i64>() {
                Ok(Value::BigInt(Some(n)))
            } else if let Some(b) = parse_bool(trimmed) {
                Ok(Value::BigInt(Some(if b { 1 } else { 0 })))
            } else {
                Err(format!("invalid integer {trimmed:?}"))
            }
        }
        ColKind::Float => trimmed
            .parse::<f64>()
            .map(|n| Value::Double(Some(n)))
            .map_err(|_| format!("invalid number {trimmed:?}")),
        ColKind::Bool => parse_bool(trimmed)
            .map(|b| Value::Bool(Some(b)))
            .ok_or_else(|| format!("invalid boolean {trimmed:?}")),
        ColKind::Bytes => decode_hex(trimmed).map(|b| Value::Bytes(Some(Box::new(b)))),
        ColKind::Date => parse_date(trimmed)
            .map(|d| Value::ChronoDate(Some(Box::new(d))))
            .ok_or_else(|| format!("invalid date {trimmed:?}")),
        ColKind::Timestamp => parse_timestamp(trimmed)
            .map(|dt| Value::ChronoDateTimeUtc(Some(Box::new(dt))))
            .ok_or_else(|| format!("invalid timestamp {trimmed:?}")),
    }
}

fn empty_of(meta: &ColMeta) -> Value {
    if meta.nullable {
        return null_of(&meta.kind);
    }
    match &meta.kind {
        ColKind::Text | ColKind::Json => Value::String(Some(Box::new(String::new()))),
        other => null_of(other),
    }
}

fn null_of(kind: &ColKind) -> Value {
    match kind {
        ColKind::Text | ColKind::Enum(_) | ColKind::Json => Value::String(None),
        ColKind::Int => Value::BigInt(None),
        ColKind::Float => Value::Double(None),
        ColKind::Bool => Value::Bool(None),
        ColKind::Bytes => Value::Bytes(None),
        ColKind::Date => Value::ChronoDate(None),
        ColKind::Timestamp => Value::ChronoDateTimeUtc(None),
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "true" | "t" | "1" | "yes" | "on" => Some(true),
        "false" | "f" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(date);
    }
    parse_timestamp(s).map(|dt| dt.date_naive())
}

fn parse_timestamp(s: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    const FMT: [&str; 4] = [
        "%Y-%m-%d %H:%M:%S%.f%z",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
    ];
    for fmt in FMT {
        if let Ok(dt) = DateTime::parse_from_str(s, fmt) {
            return Some(dt.with_timezone(&Utc));
        }
        if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(naive.and_utc());
        }
    }
    None
}

fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err(format!("hex length must be even, got {}", s.len()));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let mut idx = 0;
    while idx < s.len() {
        let pair = s
            .get(idx..idx.saturating_add(2))
            .ok_or_else(|| format!("invalid hex at {idx}"))?;
        let byte = u8::from_str_radix(pair, 16).map_err(|_| format!("invalid hex {pair:?}"))?;
        out.push(byte);
        idx = idx.saturating_add(2);
    }
    Ok(out)
}

async fn load_column_types<C: ConnectionTrait>(
    db: &C,
    table: &str,
) -> Result<HashMap<String, ColMeta>, String> {
    match db.get_database_backend() {
        DatabaseBackend::Postgres => load_postgres_types(db, table).await,
        DatabaseBackend::Sqlite => load_sqlite_types(db, table).await,
        other => Err(format!("unsupported database backend {other:?}")),
    }
}

async fn load_postgres_types<C: ConnectionTrait>(
    db: &C,
    table: &str,
) -> Result<HashMap<String, ColMeta>, String> {
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT column_name, data_type, udt_name, is_nullable FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = $1",
        [Value::String(Some(Box::new(table.to_string())))],
    );
    let rows = db
        .query_all(stmt)
        .await
        .map_err(|e| format!("column types for {table}: {e}"))?;
    let mut out = HashMap::new();
    for row in rows {
        let name: String = row
            .try_get("", "column_name")
            .map_err(|e| format!("column_name: {e}"))?;
        let data_type: String = row.try_get("", "data_type").unwrap_or_default();
        let udt: String = row.try_get("", "udt_name").unwrap_or_default();
        let is_nullable: String = row
            .try_get("", "is_nullable")
            .unwrap_or_else(|_| "YES".into());
        out.insert(
            name,
            ColMeta {
                kind: classify_type(&data_type, &udt),
                nullable: !is_nullable.eq_ignore_ascii_case("NO"),
            },
        );
    }
    Ok(out)
}

async fn load_sqlite_types<C: ConnectionTrait>(
    db: &C,
    table: &str,
) -> Result<HashMap<String, ColMeta>, String> {
    let sql = format!("PRAGMA table_info({})", quote_ident(table));
    let stmt = Statement::from_string(DatabaseBackend::Sqlite, sql);
    let rows = db
        .query_all(stmt)
        .await
        .map_err(|e| format!("column types for {table}: {e}"))?;
    let mut out = HashMap::new();
    for row in rows {
        let name: String = pragma_string(&row, "name")?;
        let decl: String = pragma_string(&row, "type").unwrap_or_default();
        let notnull: i64 = row.try_get("", "notnull").unwrap_or(0);
        out.insert(
            name,
            ColMeta {
                kind: classify_type(&decl, &decl),
                nullable: notnull == 0,
            },
        );
    }
    Ok(out)
}

fn pragma_string(row: &QueryResult, col: &str) -> Result<String, String> {
    row.try_get::<String>("", col)
        .or_else(|_| {
            row.try_get::<Option<String>>("", col)
                .map(|v| v.unwrap_or_default())
        })
        .map_err(|e| format!("pragma {col}: {e}"))
}

fn classify_type(data_type: &str, udt: &str) -> ColKind {
    let data_type = data_type.to_ascii_lowercase();
    let udt = udt.to_ascii_lowercase();
    let t = format!("{data_type} {udt}");
    if t.contains("bool") {
        ColKind::Bool
    } else if t.contains("bytea") || t.contains("blob") {
        ColKind::Bytes
    } else if data_type == "date" || udt == "date" {
        ColKind::Date
    } else if data_type == "user-defined" {
        ColKind::Enum(udt)
    } else if data_type == "json" || data_type == "jsonb" || udt == "json" || udt == "jsonb" {
        ColKind::Json
    } else if t.contains("timestamp") || t.contains("datetime") {
        ColKind::Timestamp
    } else if t.contains("double")
        || t.contains("real")
        || t.contains("float")
        || t.contains("numeric")
        || t.contains("decimal")
    {
        ColKind::Float
    } else if t.contains("int") || t.contains("serial") {
        ColKind::Int
    } else {
        ColKind::Text
    }
}

async fn row_exists<C: ConnectionTrait>(
    db: &C,
    entry: &ExportTable,
    columns: &[String],
    values: &[Value],
) -> Result<bool, String> {
    let backend = db.get_database_backend();
    let mut clauses = Vec::new();
    let mut binds = Vec::new();
    for (i, pk) in entry.primary_keys.iter().enumerate() {
        let Some(col_idx) = columns.iter().position(|c| c == pk) else {
            return Err(format!("missing pk {pk}"));
        };
        let Some(value) = values.get(col_idx).cloned() else {
            return Err(format!("missing pk value {pk}"));
        };
        clauses.push(format!(
            "{} = {}",
            quote_ident(pk),
            placeholder(backend, i.saturating_add(1))
        ));
        binds.push(value);
    }
    let sql = format!(
        "SELECT 1 FROM {} WHERE {} LIMIT 1",
        quote_ident(&entry.table),
        clauses.join(" AND ")
    );
    let stmt = Statement::from_sql_and_values(backend, sql, binds);
    let row = db
        .query_one(stmt)
        .await
        .map_err(|e| format!("exists {}: {e}", entry.table))?;
    Ok(row.is_some())
}

async fn upsert_row<C: ConnectionTrait>(
    db: &C,
    entry: &ExportTable,
    columns: &[String],
    non_pk: &[String],
    values: &[Value],
    types: &HashMap<String, ColMeta>,
) -> Result<(), String> {
    let backend = db.get_database_backend();
    let col_sql = columns
        .iter()
        .map(|c| quote_ident(c))
        .collect::<Vec<_>>()
        .join(", ");
    let placeholders = columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            typed_placeholder(
                backend,
                i.saturating_add(1),
                &types.get(col).unwrap_or(&DEFAULT_COL_META).kind,
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let conflict = entry
        .primary_keys
        .iter()
        .map(|c| quote_ident(c))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = if non_pk.is_empty() {
        format!(
            "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO NOTHING",
            quote_ident(&entry.table),
            col_sql,
            placeholders,
            conflict
        )
    } else {
        let updates = non_pk
            .iter()
            .map(|c| format!("{} = EXCLUDED.{}", quote_ident(c), quote_ident(c)))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO UPDATE SET {}",
            quote_ident(&entry.table),
            col_sql,
            placeholders,
            conflict,
            updates
        )
    };
    let stmt = Statement::from_sql_and_values(backend, sql, values.iter().cloned());
    db.execute(stmt)
        .await
        .map_err(|e| format!("upsert {}: {e}", entry.table))?;
    Ok(())
}

async fn reset_sequence<C: ConnectionTrait>(
    db: &C,
    entry: &ExportTable,
    types: &HashMap<String, ColMeta>,
) -> Result<(), String> {
    let Some(pk) = entry.primary_keys.first() else {
        return Ok(());
    };
    if entry.primary_keys.len() != 1
        || !matches!(types.get(pk), Some(meta) if meta.kind == ColKind::Int)
    {
        return Ok(());
    }
    match db.get_database_backend() {
        DatabaseBackend::Postgres => {
            let sql = format!(
                "SELECT setval(pg_get_serial_sequence({}, {}), \
                 (SELECT COALESCE(MAX({}), 1) FROM {}), true)",
                sql_string_literal(&entry.table),
                sql_string_literal(pk),
                quote_ident(pk),
                quote_ident(&entry.table)
            );
            let stmt = Statement::from_string(DatabaseBackend::Postgres, sql);
            if let Err(err) = db.execute(stmt).await {
                tracing::warn!(table = %entry.table, error = %err, "skip sequence reset");
            }
        }
        DatabaseBackend::Sqlite => {
            let sql = format!(
                "UPDATE sqlite_sequence SET seq = (SELECT COALESCE(MAX({}), 0) FROM {}) WHERE name = {}",
                quote_ident(pk),
                quote_ident(&entry.table),
                sql_string_literal(&entry.table)
            );
            let stmt = Statement::from_string(DatabaseBackend::Sqlite, sql);
            if let Err(err) = db.execute(stmt).await {
                tracing::warn!(table = %entry.table, error = %err, "skip sqlite_sequence reset");
            }
        }
        _ => {}
    }
    Ok(())
}

fn placeholder(backend: DatabaseBackend, idx: usize) -> String {
    match backend {
        DatabaseBackend::Postgres => format!("${idx}"),
        _ => "?".into(),
    }
}

fn typed_placeholder(backend: DatabaseBackend, idx: usize, kind: &ColKind) -> String {
    let base = placeholder(backend, idx);
    match (backend, kind) {
        (DatabaseBackend::Postgres, ColKind::Enum(udt)) => format!("{base}::{}", quote_ident(udt)),
        (DatabaseBackend::Postgres, ColKind::Json) => format!("{base}::jsonb"),
        _ => base,
    }
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn sql_string_literal(name: &str) -> String {
    format!("'{}'", name.replace('\'', "''"))
}

#[cfg(test)]
mod coerce_tests {
    use sea_orm::Value;

    use super::{ColKind, ColMeta, coerce_value};

    #[test]
    fn empty_not_null_text_becomes_empty_string() {
        let meta = ColMeta {
            kind: ColKind::Text,
            nullable: false,
        };
        assert_eq!(
            coerce_value(&meta, "  ").unwrap(),
            Value::String(Some(Box::new(String::new())))
        );
    }

    #[test]
    fn empty_nullable_text_becomes_null() {
        let meta = ColMeta {
            kind: ColKind::Text,
            nullable: true,
        };
        assert_eq!(coerce_value(&meta, "").unwrap(), Value::String(None));
    }
}

#[cfg(all(test, feature = "plugin-export"))]
mod tests {
    use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};

    use super::import_workbook;
    use crate::export::{ExpandedSelection, ExportCapability, ExportTable};
    use crate::plugins::export::xlsx::build_workbook;
    use crate::plugins::import::xlsx::parse_workbook;

    async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
        db.execute(Statement::from_string(DatabaseBackend::Sqlite, sql))
            .await
            .expect("sql");
    }

    async fn setup_schema(db: &sea_orm::DatabaseConnection) {
        exec(
            db,
            "CREATE TABLE roles (
                id INTEGER PRIMARY KEY,
                name TEXT
            )",
        )
        .await;
        exec(
            db,
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                name TEXT,
                role_id INTEGER,
                is_superuser INTEGER,
                password BLOB
            )",
        )
        .await;
    }

    #[tokio::test]
    async fn roundtrip_preserves_ids_and_omitted_password() {
        let source = Database::connect("sqlite::memory:")
            .await
            .expect("source db");
        setup_schema(&source).await;
        exec(&source, "INSERT INTO roles (id, name) VALUES (1, 'admin')").await;
        exec(
            &source,
            "INSERT INTO users (id, name, role_id, is_superuser, password) \
             VALUES (1, 'Ada', 1, 1, x'00ff')",
        )
        .await;
        exec(
            &source,
            "INSERT INTO users (id, name, role_id, is_superuser, password) \
             VALUES (2, 'Bob', 1, 0, x'aabb')",
        )
        .await;

        let catalog = ExportCapability::new()
            .register(ExportTable::new(
                "roles",
                "Role",
                vec!["id".into(), "name".into()],
            ))
            .register(
                ExportTable::new(
                    "users",
                    "User",
                    vec![
                        "id".into(),
                        "name".into(),
                        "role_id".into(),
                        "is_superuser".into(),
                    ],
                )
                .with_deps(vec!["roles".into()]),
            )
            .catalog();
        let bytes = build_workbook(
            &source,
            &catalog,
            &ExpandedSelection {
                roots: vec!["users".into()],
                tables: vec!["roles".into(), "users".into()],
            },
        )
        .await
        .expect("export");

        let target = Database::connect("sqlite::memory:")
            .await
            .expect("target db");
        setup_schema(&target).await;
        exec(&target, "INSERT INTO roles (id, name) VALUES (1, 'old')").await;
        exec(
            &target,
            "INSERT INTO users (id, name, role_id, is_superuser, password) \
             VALUES (1, 'Old', 1, 0, x'deadbeef')",
        )
        .await;

        let parsed = parse_workbook(&bytes, &catalog).expect("parse");
        let report = import_workbook(&target, &catalog, &parsed)
            .await
            .expect("import");
        assert_eq!(report.tables.len(), 2);

        let role_name: String = target
            .query_one(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT name FROM roles WHERE id = 1",
            ))
            .await
            .expect("role query")
            .expect("role row")
            .try_get("", "name")
            .expect("role name");
        assert_eq!(role_name, "admin");

        let user_row = target
            .query_one(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT name, role_id, is_superuser, password FROM users WHERE id = 1",
            ))
            .await
            .expect("user query")
            .expect("user row");
        let name: String = user_row.try_get("", "name").expect("name");
        let role_id: i64 = user_row.try_get("", "role_id").expect("role_id");
        let is_superuser: i64 = user_row.try_get("", "is_superuser").expect("flag");
        let password: Vec<u8> = user_row.try_get("", "password").expect("password");
        assert_eq!(name, "Ada");
        assert_eq!(role_id, 1);
        assert_eq!(is_superuser, 1);
        assert_eq!(password, vec![0xde, 0xad, 0xbe, 0xef]);

        let bob: String = target
            .query_one(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT name FROM users WHERE id = 2",
            ))
            .await
            .expect("bob query")
            .expect("bob row")
            .try_get("", "name")
            .expect("bob name");
        assert_eq!(bob, "Bob");
    }

    #[tokio::test]
    async fn empty_not_null_text_imports_as_empty_string() {
        let db = Database::connect("sqlite::memory:").await.expect("db");
        exec(
            &db,
            "CREATE TABLE otp_preferences (
                id INTEGER PRIMARY KEY,
                otp_template_id TEXT NOT NULL DEFAULT '',
                smtp_host TEXT
            )",
        )
        .await;

        let catalog = ExportCapability::new()
            .register(ExportTable::new(
                "otp_preferences",
                "OtpPreferences",
                vec!["id".into(), "otp_template_id".into(), "smtp_host".into()],
            ))
            .catalog();
        let parsed = crate::plugins::import::xlsx::ParsedWorkbook {
            sheets: vec![crate::plugins::import::xlsx::ParsedSheet {
                table: "otp_preferences".into(),
                columns: vec!["id".into(), "otp_template_id".into(), "smtp_host".into()],
                rows: vec![vec!["1".into(), "".into(), "".into()]],
            }],
            skipped_sheets: Vec::new(),
        };

        import_workbook(&db, &catalog, &parsed)
            .await
            .expect("import");

        let row = db
            .query_one(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT otp_template_id, smtp_host FROM otp_preferences WHERE id = 1",
            ))
            .await
            .expect("query")
            .expect("row");
        let template_id: String = row.try_get("", "otp_template_id").expect("template");
        let smtp_host: Option<String> = row.try_get("", "smtp_host").expect("smtp");
        assert_eq!(template_id, "");
        assert_eq!(smtp_host, None);
    }
}
