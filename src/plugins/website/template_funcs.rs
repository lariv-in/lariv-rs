//! minijinja globals/filters porting Go `html/template` FuncMap from `pages.go`.

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use minijinja::{Environment, Error, ErrorKind, Value};
use pulldown_cmark::{Options, Parser, html};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde_json::{Map as JsonMap, Value as JsonValue, json};

fn format_time_val(val: &Value, fmt_layout: &str) -> String {
    if val.is_undefined() || val.is_none() {
        return String::new();
    }
    if let Some(s) = val.as_str() {
        let s = s.trim();
        if s.is_empty() {
            return String::new();
        }
        if let Ok(t) = DateTime::parse_from_rfc3339(s) {
            return t.with_timezone(&Utc).format(fmt_layout).to_string();
        }
        if let Ok(t) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
            return t.format(fmt_layout).to_string();
        }
        if let Ok(t) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            return t.format(fmt_layout).to_string();
        }
        return s.to_string();
    }
    val.to_string()
}

fn markdown_to_html(s: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(s, opts);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

fn block_on_db<F, T>(fut: F) -> Result<T, Error>
where
    F: std::future::Future<Output = Result<T, sea_orm::DbErr>>,
{
    let handle = tokio::runtime::Handle::try_current().map_err(|_| {
        Error::new(
            ErrorKind::InvalidOperation,
            "no tokio runtime for template query",
        )
    })?;
    tokio::task::block_in_place(|| handle.block_on(fut))
        .map_err(|e| Error::new(ErrorKind::InvalidOperation, e.to_string()))
}

fn rows_to_maps(
    backend: DatabaseBackend,
    db: &DatabaseConnection,
    table: &str,
    rows: Vec<sea_orm::QueryResult>,
) -> Result<Value, Error> {
    if rows.is_empty() {
        return Ok(Value::from_serialize(Vec::<JsonValue>::new()));
    }
    let cols = column_names(backend, db, table)?;
    let mut out = Vec::new();
    for row in rows {
        let mut map = JsonMap::new();
        for col in &cols {
            if let Ok(v) = row.try_get::<Option<String>>("", col.as_str()) {
                map.insert(col.clone(), json!(v));
            } else if let Ok(v) = row.try_get::<Option<i64>>("", col.as_str()) {
                map.insert(col.clone(), json!(v));
            } else if let Ok(v) = row.try_get::<Option<bool>>("", col.as_str()) {
                map.insert(col.clone(), json!(v));
            } else {
                map.insert(col.clone(), JsonValue::Null);
            }
        }
        out.push(JsonValue::Object(map));
    }
    Ok(Value::from_serialize(out))
}

fn column_names(
    backend: DatabaseBackend,
    db: &DatabaseConnection,
    table: &str,
) -> Result<Vec<String>, Error> {
    let sql = match backend {
        DatabaseBackend::Postgres => format!(
            "SELECT column_name FROM information_schema.columns WHERE table_name = '{table}' ORDER BY ordinal_position"
        ),
        DatabaseBackend::Sqlite => format!("PRAGMA table_info('{table}')"),
        _ => return Ok(vec![]),
    };
    let db2 = db.clone();
    let rows = block_on_db(async move {
        db2.query_all(Statement::from_string(backend, sql)).await
    })?;
    let mut cols = Vec::new();
    for row in rows {
        let name = match backend {
            DatabaseBackend::Postgres => row
                .try_get::<String>("", "column_name")
                .map_err(|e| Error::new(ErrorKind::InvalidOperation, e.to_string()))?,
            DatabaseBackend::Sqlite => row
                .try_get::<String>("", "name")
                .map_err(|e| Error::new(ErrorKind::InvalidOperation, e.to_string()))?,
            _ => continue,
        };
        cols.push(name);
    }
    Ok(cols)
}

/// Register request-scoped globals and DB helpers on a minijinja environment.
pub fn register_funcs(
    env: &mut Environment<'static>,
    db: DatabaseConnection,
    path: String,
    query: Vec<(String, String)>,
) {
    let path_fn = path.clone();
    env.add_function("path", move || path_fn.clone());

    let slug_path = path;
    env.add_function("slug", move || {
        let trimmed = slug_path.trim_end_matches('/');
        trimmed.rsplit('/').next().unwrap_or("").to_string()
    });

    env.add_function("param", move |name: String| {
        query
            .iter()
            .find(|(k, _)| k == &name)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    });

    env.add_function("first", |slice: Value| {
        slice
            .get_item_by_index(0)
            .unwrap_or_else(|_| Value::from(()))
    });

    env.add_filter("format_datetime", |val: Value, _layout: Option<String>| {
        format_time_val(&val, "%a, %d %b %Y %H:%M:%S")
    });

    env.add_filter("format_date", |val: Value, _layout: Option<String>| {
        format_time_val(&val, "%d %b %Y")
    });

    env.add_filter("markdown", |val: Value| {
        let s = val
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| val.to_string());
        Value::from_safe_string(markdown_to_html(&s))
    });

    env.add_filter("markdown_unsafe", |val: Value| {
        let s = val
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| val.to_string());
        Value::from_safe_string(markdown_to_html(&s))
    });

    let db_q = db.clone();
    env.add_function(
        "query",
        move |table: String, limit: Option<i64>, offset: Option<i64>| -> Result<Value, Error> {
            let limit = limit.filter(|l| *l > 0).unwrap_or(10);
            let offset = offset.unwrap_or(0);
            let backend = db_q.get_database_backend();
            let sql = format!("SELECT * FROM \"{table}\" LIMIT {limit} OFFSET {offset}");
            let db = db_q.clone();
            let rows = block_on_db(async move {
                db.query_all(Statement::from_string(backend, sql)).await
            })?;
            rows_to_maps(backend, &db_q, &table, rows)
        },
    );

    let db_qw = db.clone();
    env.add_function(
        "query_where",
        move |table: String, where_clause: String| -> Result<Value, Error> {
            let backend = db_qw.get_database_backend();
            let sql = format!("SELECT * FROM \"{table}\" WHERE {where_clause}");
            let db = db_qw.clone();
            let rows = block_on_db(async move {
                db.query_all(Statement::from_string(backend, sql)).await
            })?;
            rows_to_maps(backend, &db_qw, &table, rows)
        },
    );

    let db_get = db;
    env.add_function(
        "get",
        move |table: String, id: Value| -> Result<Value, Error> {
            let backend = db_get.get_database_backend();
            let id_sql = id.to_string();
            let sql = format!("SELECT * FROM \"{table}\" WHERE id = {id_sql} LIMIT 1");
            let db = db_get.clone();
            let rows = block_on_db(async move {
                db.query_all(Statement::from_string(backend, sql)).await
            })?;
            let vals = rows_to_maps(backend, &db_get, &table, rows)?;
            Ok(vals
                .get_item_by_index(0)
                .unwrap_or_else(|_| Value::from(())))
        },
    );
}
