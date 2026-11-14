//! Canonical SeaORM 2.x helpers for plugin migrations.
//!
//! Prefer these patterns in new migrations:
//! - **SeaQuery DDL/DML:** `manager.create_table(...)`, `conn.execute(&query)`, etc.
//! - **Raw SQL string:** `exec_sql(manager, sql).await?`
//! - **Runtime raw SQL:** `exec_sql_conn(conn, backend, sql).await?`
//! - **Another plugin's table:** create the column, then [`ensure_foreign_key`]. The
//!   constraint is skipped until that table exists. Plugins list those keys on
//!   `define_register_migrations!`; the mounted app applies them again after every
//!   plugin migrator has run.

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};
use sea_orm_migration::prelude::*;

/// Whether the migration connection targets PostgreSQL.
pub fn is_postgres(manager: &SchemaManager<'_>) -> bool {
    manager.get_database_backend() == DatabaseBackend::Postgres
}

/// Execute a raw SQL string during a migration (`execute_raw` wrapper).
/// Finish table rebuilds that stopped on `RENAME` because the statement cache still
/// held the dropped name. A new connection can rename the copy back.
pub async fn repair_sqlite_rebuild_copies(db: &DatabaseConnection) -> Result<(), DbErr> {
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name LIKE '%\\_\\_d' ESCAPE '\\'"
                .to_string(),
        ))
        .await?;
    for row in rows {
        let name: String = row.try_get("", "name")?;
        let Some(table) = name.strip_suffix("__d") else {
            continue;
        };
        if sqlite_table_exists(db, DatabaseBackend::Sqlite, table).await? {
            db.execute_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                format!("DROP TABLE \"{name}\""),
            ))
            .await?;
        } else {
            db.execute_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                format!("ALTER TABLE \"{name}\" RENAME TO \"{table}\""),
            ))
            .await?;
        }
    }
    Ok(())
}

pub async fn exec_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    exec_sql_conn(
        manager.get_connection(),
        manager.get_database_backend(),
        sql,
    )
    .await
}

/// Execute a raw SQL string on any connection (tests, conditional DDL wrappers).
pub async fn exec_sql_conn<C: ConnectionTrait>(
    conn: &C,
    backend: DatabaseBackend,
    sql: &str,
) -> Result<(), DbErr> {
    if backend == DatabaseBackend::Sqlite {
        for statement in sqlite_statements(sql) {
            match conn
                .execute_raw(Statement::from_string(backend, &statement))
                .await
            {
                Ok(_) => {}
                Err(err) if drop_column_blocked(&err) => {
                    let Some((table, column)) = drop_column_target(&statement) else {
                        return Err(err);
                    };
                    rebuild_table_without_column(conn, backend, table, column).await?;
                }
                Err(err) if sqlite_error_is_benign(&err) => {
                    tracing::debug!(error = %err, statement = %statement, "sqlite migration statement skipped");
                }
                Err(err) => return Err(err),
            }
        }
        return Ok(());
    }
    // A prepared statement accepts one command. Semicolons outside quotes split
    // the script, and a `DO` block stays one command.
    for statement in split_sql(sql) {
        conn.execute_raw(Statement::from_string(backend, statement))
            .await?;
    }
    Ok(())
}

/// Postgres `DO` blocks and column `IF EXISTS` clauses, split into statements
/// SQLite can run. Schema-only pieces of a `DO` block are kept; plpgsql control
/// flow is dropped.
fn sqlite_statements(sql: &str) -> Vec<String> {
    if procedural_do(sql) {
        return Vec::new();
    }
    let source = unwrap_do_block(sql).unwrap_or_else(|| sql.to_string());
    let mut out = Vec::new();
    for statement in split_sql(&source) {
        let trimmed = statement.trim();
        if trimmed.len() >= 2 && trimmed[..2].eq_ignore_ascii_case("DO") {
            if !procedural_do(trimmed) {
                out.extend(sqlite_statements(trimmed));
            }
            continue;
        }
        if let Some((drop_sql, create_sql)) = rewrite_create_table_like(trimmed) {
            out.push(drop_sql);
            out.push(create_sql);
            continue;
        }
        if let Some(adapted) = adapt_sqlite_statement(trimmed) {
            out.extend(split_multi_alter(&adapted));
        }
    }
    out
}

/// `CREATE TABLE new (LIKE old INCLUDING DEFAULTS)` is Postgres.
/// SQLite gets an empty copy of the columns. Constraints are added later, or skipped.
fn rewrite_create_table_like(sql: &str) -> Option<(String, String)> {
    let trimmed = sql.trim().trim_matches(';').trim();
    let upper = trimmed.to_ascii_uppercase();
    if !upper.starts_with("CREATE TABLE") || !upper.contains("(LIKE ") {
        return None;
    }
    let after = trimmed["CREATE TABLE".len()..].trim_start();
    let (name, rest) = take_table_name(after);
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('(')?.trim_start();
    if !rest.to_ascii_uppercase().starts_with("LIKE ") {
        return None;
    }
    let after_like = rest["LIKE".len()..].trim_start();
    let (other, _) = take_table_name(after_like);
    if name.is_empty() || other.is_empty() {
        return None;
    }
    Some((
        format!("DROP TABLE IF EXISTS {name}"),
        format!("CREATE TABLE {name} AS SELECT * FROM {other} WHERE 0"),
    ))
}

fn procedural_do(sql: &str) -> bool {
    let Some(body) = unwrap_do_block(sql) else {
        return false;
    };
    let upper = body.to_ascii_uppercase();
    // Assignments, casts, and catalog lookups are plpgsql. A fresh SQLite database
    // has no rows for those backfills.
    upper.contains(":=")
        || upper.contains(" LATERAL")
        || upper.contains("::")
        || upper.contains("PERFORM")
        || upper.contains("EXECUTE ")
        || upper.contains("TO_REGCLASS")
        || upper.contains("NEXTVAL(")
        || upper.contains("ON COMMIT")
}

fn unwrap_do_block(sql: &str) -> Option<String> {
    let trimmed = sql.trim();
    let head = trimmed.get(..2)?.to_ascii_uppercase();
    if head != "DO" {
        return None;
    }
    let start = trimmed.find('$')?;
    let tag_end = trimmed[start + 1..].find('$')? + start + 1;
    let tag = &trimmed[start..=tag_end];
    let rest = &trimmed[tag_end + 1..];
    let close = rest.rfind(tag)?;
    let mut body = rest[..close].trim().to_string();
    if let Some(begin) = body.to_ascii_uppercase().rfind("BEGIN") {
        // Keep the last BEGIN so a DECLARE section is not executed.
        body = body[begin + "BEGIN".len()..].trim().to_string();
    }
    Some(body)
}

fn split_sql(sql: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut chars = sql.chars().peekable();
    let mut in_single = false;
    let mut dollar: Option<String> = None;
    while let Some(ch) = chars.next() {
        if let Some(tag) = &dollar {
            current.push(ch);
            if ch == '$' && current.ends_with(tag) {
                dollar = None;
            }
            continue;
        }
        if in_single {
            current.push(ch);
            if ch == '\'' {
                if chars.peek() == Some(&'\'') {
                    if let Some(escaped) = chars.next() {
                        current.push(escaped);
                    }
                } else {
                    in_single = false;
                }
            }
            continue;
        }
        match ch {
            '\'' => {
                in_single = true;
                current.push(ch);
            }
            '$' => {
                let mut tag = String::from("$");
                while let Some(next) = chars.peek().copied() {
                    if next == '$' || next.is_ascii_alphanumeric() || next == '_' {
                        tag.push(next);
                        chars.next();
                        if next == '$' {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                if tag.ends_with('$') && tag.len() >= 2 {
                    dollar = Some(tag.clone());
                }
                current.push_str(&tag);
            }
            ';' => {
                let statement = current.trim().to_string();
                if !statement.is_empty() {
                    out.push(statement);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let statement = current.trim().to_string();
    if !statement.is_empty() {
        out.push(statement);
    }
    out
}

/// Text after the `THEN` that closes an `IF` / `ELSIF` condition.
fn text_after_then(sql: &str) -> Option<&str> {
    let mut chars = sql.char_indices().peekable();
    let mut depth = 0_i32;
    let mut in_single = false;
    let mut dollar: Option<String> = None;
    while let Some((idx, ch)) = chars.next() {
        if let Some(tag) = &dollar {
            if ch == '$' && sql[idx..].starts_with(tag) {
                for _ in 1..tag.chars().count() {
                    chars.next();
                }
                dollar = None;
            }
            continue;
        }
        if in_single {
            if ch == '\'' {
                if chars.peek().is_some_and(|(_, next)| *next == '\'') {
                    chars.next();
                } else {
                    in_single = false;
                }
            }
            continue;
        }
        match ch {
            '\'' => in_single = true,
            '(' => depth += 1,
            ')' => depth -= 1,
            '$' => {
                let mut tag = String::from("$");
                while let Some((_, next)) = chars.peek().copied() {
                    if next == '$' || next.is_ascii_alphanumeric() || next == '_' {
                        tag.push(next);
                        chars.next();
                        if next == '$' {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                if tag.ends_with('$') && tag.len() >= 2 {
                    dollar = Some(tag);
                }
            }
            _ if depth == 0 && (ch == 'T' || ch == 't') => {
                let rest = &sql[idx..];
                let word = rest
                    .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .next()
                    .unwrap_or("");
                if word.eq_ignore_ascii_case("THEN") {
                    return Some(rest[word.len()..].trim());
                }
            }
            _ => {}
        }
    }
    None
}

fn adapt_sqlite_statement(statement: &str) -> Option<String> {
    let trimmed = statement.trim().trim_matches(';').trim();
    if trimmed.is_empty() {
        return None;
    }
    let upper = trimmed.to_ascii_uppercase();
    if upper.starts_with("IF ") || upper.starts_with("ELSIF ") || upper == "IF" {
        // `IF ... THEN <sql>` is one statement up to the next semicolon.
        // Keep the SQL that follows THEN.
        let after = text_after_then(trimmed)?;
        return adapt_sqlite_statement(after);
    }
    if upper.starts_with("END")
        || upper.starts_with("ELSE")
        || upper.starts_with("DECLARE")
        || upper.starts_with("BEGIN")
        || upper.starts_with("RETURN")
        || upper.starts_with("FOR ")
        || upper.starts_with("LOOP")
        || upper.starts_with("PERFORM")
        || upper.starts_with("EXECUTE")
        || upper.starts_with("EXCEPTION")
        || upper.starts_with("WHEN ")
        || upper.starts_with("RAISE")
        || upper.starts_with("SELECT ")
        || upper.starts_with("CREATE TYPE")
        || upper.starts_with("DROP TYPE")
        || upper.starts_with("CREATE OR REPLACE FUNCTION")
        || upper.starts_with("CREATE FUNCTION")
        || upper.starts_with("CREATE OR REPLACE PROCEDURE")
        || upper.starts_with("CREATE TRIGGER")
        || upper.starts_with("CREATE OR REPLACE TRIGGER")
        || upper.starts_with("DROP FUNCTION")
        || upper.starts_with("CREATE EXTENSION")
        || upper.starts_with("CREATE SEQUENCE")
        || upper.starts_with("ALTER SEQUENCE")
        || upper.starts_with("DROP SEQUENCE")
        || upper.starts_with("ALTER TYPE")
        || upper.starts_with("COMMENT ")
        || upper.starts_with("SET ")
        || upper.contains("EXECUTE PROCEDURE")
        || upper.contains("EXECUTE FUNCTION")
    {
        return None;
    }
    if upper.contains(" USING GIN") || upper.contains("GIN_TRGM_OPS") {
        return None;
    }
    let verb = upper.split_whitespace().next().unwrap_or("");
    if !matches!(
        verb,
        "ALTER"
            | "UPDATE"
            | "INSERT"
            | "DELETE"
            | "DROP"
            | "CREATE"
            | "PRAGMA"
            | "WITH"
            | "REPLACE"
            | "REINDEX"
            | "ANALYZE"
            | "VACUUM"
    ) {
        // Leftover plpgsql (`extras jsonb`, assignments) is not SQL.
        return None;
    }
    // Postgres casts and `trim(trailing … from …)` have no SQLite spelling.
    // These show up in data backfills; a fresh SQLite database has no rows to convert.
    if upper.contains("::")
        || upper.contains("TRIM(TRAILING")
        || upper.contains(" ~ ")
        || upper.contains("DISTINCT ON")
    {
        return None;
    }
    let mut sql = trimmed.to_string();
    if upper.starts_with("ALTER TABLE") {
        let alter = upper.strip_prefix("ALTER TABLE").unwrap_or("");
        if alter.contains(" ADD CONSTRAINT")
            || alter.contains(" ADD PRIMARY KEY")
            || alter.contains(" DROP CONSTRAINT")
            || alter.contains(" ALTER COLUMN")
            || alter.contains(" ALTER ")
        {
            // `ALTER COLUMN` and adding a constraint to an existing table are Postgres-only.
            // `RENAME` still contains neither of those markers.
            if !alter.contains(" RENAME ") {
                return None;
            }
        }
        sql = sql
            .replace(" ADD COLUMN IF NOT EXISTS ", " ADD COLUMN ")
            .replace(" ADD COLUMN IF NOT EXISTS", " ADD COLUMN")
            .replace(" DROP COLUMN IF EXISTS ", " DROP COLUMN ")
            .replace(" DROP COLUMN IF EXISTS", " DROP COLUMN");
    }
    if upper.starts_with("DROP TABLE")
        || upper.starts_with("DROP INDEX")
        || upper.contains("DROP COLUMN")
    {
        sql = sql.replace(" CASCADE", "");
    }
    if upper.starts_with("CREATE") {
        sql = sql
            .replace(" ON COMMIT DROP", "")
            .replace(" on commit drop", "");
    }
    if upper.starts_with("DROP TRIGGER") {
        if let Some(on) = upper.rfind(" ON ") {
            sql = sql[..on].trim().to_string();
        }
    }
    sql = replace_outside_quotes(&sql, "ILIKE", "LIKE");
    Some(rewrite_sqlite_functions(&rewrite_update_target_alias(&sql)))
}

/// `UPDATE table alias SET ... alias.col` is Postgres. SQLite wants the real table name.
fn rewrite_update_target_alias(sql: &str) -> String {
    let trimmed = sql.trim_start();
    if !trimmed.to_ascii_uppercase().starts_with("UPDATE ") {
        return sql.to_string();
    }
    let after = trimmed["UPDATE".len()..].trim_start();
    let (table, after_table) = take_table_name(after);
    let after_table = after_table.trim_start();
    let (alias, after_alias) = take_table_name(after_table);
    if table.is_empty()
        || alias.is_empty()
        || !after_alias
            .trim_start()
            .to_ascii_uppercase()
            .starts_with("SET")
    {
        return sql.to_string();
    }
    let set_sql = replace_alias_qualifier(after_alias.trim_start(), alias, table);
    format!("UPDATE {table} {set_sql}")
}

fn replace_alias_qualifier(sql: &str, alias: &str, table: &str) -> String {
    let needle = format!("{alias}.");
    let mut out = String::new();
    let mut rest = sql;
    while let Some(idx) = rest.find(&needle) {
        let boundary_ok = rest[..idx]
            .chars()
            .next_back()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_');
        if boundary_ok {
            out.push_str(&rest[..idx]);
            out.push_str(table);
            out.push('.');
            rest = &rest[idx + needle.len()..];
        } else {
            let next = idx + 1;
            out.push_str(&rest[..next]);
            rest = &rest[next..];
        }
    }
    out.push_str(rest);
    out
}

/// Postgres functions SQLite does not provide, swapped for equivalents.
fn ident_boundary(rest: &str) -> bool {
    rest.chars()
        .next()
        .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
}

/// `ALTER TABLE t ADD a, ADD b` is one Postgres statement and several SQLite statements.
fn split_multi_alter(sql: &str) -> Vec<String> {
    let upper = sql.to_ascii_uppercase();
    if !upper.starts_with("ALTER TABLE") {
        return vec![sql.to_string()];
    }
    let after_alter = sql["ALTER TABLE".len()..].trim_start();
    let (name, after_name) = take_table_name(after_alter);
    let actions = split_top_level_commas(after_name.trim_start());
    if name.is_empty() || actions.len() <= 1 {
        return vec![sql.to_string()];
    }
    actions
        .into_iter()
        .filter(|action| !action.is_empty())
        .map(|action| format!("ALTER TABLE {name} {action}"))
        .collect()
}

fn take_table_name(sql: &str) -> (&str, &str) {
    let sql = sql.trim_start();
    if let Some(rest) = sql.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            let name_end = end + 2;
            return (&sql[..name_end], &sql[name_end..]);
        }
    }
    let end = sql.find(|ch: char| ch.is_whitespace()).unwrap_or(sql.len());
    (&sql[..end], &sql[end..])
}

fn split_top_level_commas(sql: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let bytes = sql.as_bytes();
    let mut start = 0;
    let mut depth = 0_i32;
    let mut in_single = false;
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i];
        if in_single {
            if ch == b'\'' {
                if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                }
                in_single = false;
            }
            i += 1;
            continue;
        }
        match ch {
            b'\'' => in_single = true,
            b'(' => depth += 1,
            b')' => depth -= 1,
            b',' if depth == 0 => {
                out.push(sql[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    let last = sql[start..].trim();
    if !last.is_empty() {
        out.push(last);
    }
    out
}

fn replace_outside_quotes(sql: &str, from: &str, to: &str) -> String {
    let mut out = String::new();
    let mut rest = sql;
    let mut in_single = false;
    while !rest.is_empty() {
        if in_single {
            if let Some(idx) = rest.find('\'') {
                out.push_str(&rest[..=idx]);
                if rest[idx + 1..].starts_with('\'') {
                    out.push('\'');
                    rest = &rest[idx + 2..];
                } else {
                    in_single = false;
                    rest = &rest[idx + 1..];
                }
            } else {
                out.push_str(rest);
                break;
            }
            continue;
        }
        if rest.starts_with('\'') {
            in_single = true;
            out.push('\'');
            rest = &rest[1..];
            continue;
        }
        if rest.len() >= from.len() && rest[..from.len()].eq_ignore_ascii_case(from) {
            let before_ok = out
                .chars()
                .next_back()
                .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_');
            let after_ok = rest[from.len()..]
                .chars()
                .next()
                .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_');
            if before_ok && after_ok {
                out.push_str(to);
                rest = &rest[from.len()..];
                continue;
            }
        }
        let ch = rest.chars().next().expect("rest is not empty");
        out.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out
}

fn rewrite_sqlite_functions(sql: &str) -> String {
    let mut out = String::new();
    let mut rest = sql;
    let mut in_single = false;
    while !rest.is_empty() {
        if in_single {
            let ch = rest.chars().next().unwrap_or('\0');
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
            if ch == '\'' {
                if rest.starts_with('\'') {
                    out.push('\'');
                    rest = &rest[1..];
                } else {
                    in_single = false;
                }
            }
            continue;
        }
        if rest.starts_with('\'') {
            in_single = true;
            out.push('\'');
            rest = &rest[1..];
            continue;
        }
        let boundary = out
            .chars()
            .next_back()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_');
        if boundary {
            if let Some(after) = take_empty_call(rest, "now") {
                out.push_str("CURRENT_TIMESTAMP");
                rest = after;
                continue;
            }
            if let Some(after) = take_name(rest, "btrim") {
                if ident_boundary(after) {
                    out.push_str("trim");
                    rest = after;
                    continue;
                }
            }
            if let Some(after) = take_name(rest, "false") {
                if ident_boundary(after) {
                    out.push('0');
                    rest = after;
                    continue;
                }
            }
            if let Some(after) = take_name(rest, "true") {
                if ident_boundary(after) {
                    out.push('1');
                    rest = after;
                    continue;
                }
            }
        }
        let ch = rest.chars().next().unwrap_or('\0');
        out.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out
}

/// `name()` with nothing between the parentheses. Returns the text after the call.
fn take_empty_call<'a>(rest: &'a str, name: &str) -> Option<&'a str> {
    let after_name = take_name(rest, name)?;
    let after_open = after_name.trim_start().strip_prefix('(')?;
    let after_close = after_open.trim_start().strip_prefix(')')?;
    Some(after_close)
}

/// Text after `name` when `rest` starts with that identifier.
fn take_name<'a>(rest: &'a str, name: &str) -> Option<&'a str> {
    let head = rest.get(..name.len())?;
    if !head.eq_ignore_ascii_case(name) {
        return None;
    }
    rest.get(name.len()..)
}

fn sqlite_error_is_benign(err: &DbErr) -> bool {
    let message = err.to_string().to_ascii_lowercase();
    message.contains("duplicate column name")
        || message.contains("no such column")
        || message.contains("no such table")
        || message.contains("no such index")
        || message.contains("already exists")
        || message.contains("duplicate column")
}

fn drop_column_blocked(err: &DbErr) -> bool {
    let message = err.to_string().to_ascii_lowercase();
    message.contains("cannot drop unique column")
        || message.contains("cannot drop primary key")
        || message.contains("after drop column")
}

fn drop_column_target(statement: &str) -> Option<(&str, &str)> {
    let upper = statement.to_ascii_uppercase();
    if !upper.starts_with("ALTER TABLE") {
        return None;
    }
    let idx = upper.find("DROP COLUMN")?;
    let after_alter = statement["ALTER TABLE".len()..].trim_start();
    let (table, _) = take_table_name(after_alter);
    let after_drop = statement[idx + "DROP COLUMN".len()..].trim_start();
    let (column, _) = take_table_name(after_drop);
    if table.is_empty() || column.is_empty() {
        return None;
    }
    Some((table, column.trim_matches('"')))
}

/// SQLite refuses `DROP COLUMN` when the column is UNIQUE or named by a foreign key.
/// Rebuild the table without that column. A rename onto a name dropped in this same
/// connection can fail the statement cache; the next process finishes the rename.
async fn rebuild_table_without_column<C: ConnectionTrait>(
    conn: &C,
    backend: DatabaseBackend,
    table: &str,
    column: &str,
) -> Result<(), DbErr> {
    let table = table.trim_matches('"');
    let column = column.trim_matches('"');
    let temp = format!("{table}__d");
    if repair_rebuilt_table(conn, backend, table, &temp).await? {
        return Ok(());
    }
    let Some(create_sql) = table_create_sql(conn, backend, table).await? else {
        return Ok(());
    };
    let Some(rewritten) = schema_without_column(&create_sql, column) else {
        return Err(DbErr::Custom(format!(
            "could not drop column {column} from {table}"
        )));
    };
    let temp_sql = rename_created_table(&rewritten, table, &temp);
    let columns = column_names_in_create(&temp_sql);
    conn.execute_raw(Statement::from_string(
        backend,
        "PRAGMA foreign_keys=OFF".to_string(),
    ))
    .await?;
    conn.execute_raw(Statement::from_string(backend, temp_sql))
        .await?;
    if !columns.is_empty() {
        let cols = columns
            .iter()
            .map(|name| format!("\"{name}\""))
            .collect::<Vec<_>>()
            .join(", ");
        conn.execute_raw(Statement::from_string(
            backend,
            format!("INSERT INTO \"{temp}\" ({cols}) SELECT {cols} FROM \"{table}\""),
        ))
        .await?;
    }
    conn.execute_raw(Statement::from_string(
        backend,
        format!("DROP TABLE \"{table}\""),
    ))
    .await?;
    let renamed = conn
        .execute_raw(Statement::from_string(
            backend,
            format!("ALTER TABLE \"{temp}\" RENAME TO \"{table}\""),
        ))
        .await;
    conn.execute_raw(Statement::from_string(
        backend,
        "PRAGMA foreign_keys=ON".to_string(),
    ))
    .await?;
    renamed.map(|_| ())
}

async fn repair_rebuilt_table<C: ConnectionTrait>(
    conn: &C,
    backend: DatabaseBackend,
    table: &str,
    temp: &str,
) -> Result<bool, DbErr> {
    let temp_exists = sqlite_table_exists(conn, backend, temp).await?;
    let table_exists = sqlite_table_exists(conn, backend, table).await?;
    if temp_exists && !table_exists {
        conn.execute_raw(Statement::from_string(
            backend,
            format!("ALTER TABLE \"{temp}\" RENAME TO \"{table}\""),
        ))
        .await?;
        return Ok(true);
    }
    if temp_exists && table_exists {
        conn.execute_raw(Statement::from_string(
            backend,
            format!("DROP TABLE \"{temp}\""),
        ))
        .await?;
    }
    Ok(false)
}

async fn sqlite_table_exists<C: ConnectionTrait>(
    conn: &C,
    backend: DatabaseBackend,
    table: &str,
) -> Result<bool, DbErr> {
    let row = conn
        .query_one_raw(Statement::from_string(
            backend,
            format!(
                "SELECT 1 AS present FROM sqlite_master WHERE type = 'table' AND name = '{table}'"
            ),
        ))
        .await?;
    Ok(row.is_some())
}

async fn table_create_sql<C: ConnectionTrait>(
    conn: &C,
    backend: DatabaseBackend,
    table: &str,
) -> Result<Option<String>, DbErr> {
    let row = conn
        .query_one_raw(Statement::from_string(
            backend,
            format!("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = '{table}'"),
        ))
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    row.try_get("", "sql")
        .map(Some)
        .or_else(|_| row.try_get("", "SQL").map(Some))
}

fn schema_without_column(create_sql: &str, column: &str) -> Option<String> {
    let open = create_sql.find('(')?;
    let close = create_sql.rfind(')')?;
    if close <= open {
        return None;
    }
    let kept = split_top_level_commas(&create_sql[open + 1..close])
        .into_iter()
        .filter(|piece| !piece.is_empty() && !mentions_ident(piece, column))
        .collect::<Vec<_>>();
    if kept.is_empty() {
        return None;
    }
    Some(format!(
        "{}(\n{}\n){}",
        &create_sql[..open],
        kept.join(",\n"),
        &create_sql[close + 1..]
    ))
}

fn rename_created_table(create_sql: &str, from: &str, to: &str) -> String {
    let quoted = format!("\"{from}\"");
    if let Some(idx) = create_sql.find(&quoted) {
        let mut out = String::new();
        out.push_str(&create_sql[..idx]);
        out.push('"');
        out.push_str(to);
        out.push('"');
        out.push_str(&create_sql[idx + quoted.len()..]);
        return out;
    }
    create_sql.replacen(from, to, 1)
}

fn column_names_in_create(create_sql: &str) -> Vec<String> {
    let Some(open) = create_sql.find('(') else {
        return Vec::new();
    };
    let Some(close) = create_sql.rfind(')') else {
        return Vec::new();
    };
    split_top_level_commas(&create_sql[open + 1..close])
        .into_iter()
        .filter_map(|piece| {
            let piece = piece.trim();
            let upper = piece.to_ascii_uppercase();
            if upper.starts_with("CONSTRAINT")
                || upper.starts_with("PRIMARY")
                || upper.starts_with("UNIQUE")
                || upper.starts_with("CHECK")
                || upper.starts_with("FOREIGN")
            {
                return None;
            }
            let (name, _) = take_table_name(piece);
            let name = name.trim_matches('"');
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect()
}

fn mentions_ident(sql: &str, ident: &str) -> bool {
    let sql = sql.to_ascii_lowercase();
    let ident = ident.trim_matches('"').to_ascii_lowercase();
    if ident.is_empty() {
        return false;
    }
    let mut rest = sql.as_str();
    while let Some(idx) = rest.find(&ident) {
        let before_ok = rest[..idx]
            .chars()
            .next_back()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_');
        let after_ok = rest[idx + ident.len()..]
            .chars()
            .next()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_');
        if before_ok && after_ok {
            return true;
        }
        rest = &rest[idx + ident.len()..];
    }
    false
}

/// `ON UPDATE` / `ON DELETE` action. Only these keywords are interpolated into DDL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferentialAction {
    Cascade,
    Restrict,
    SetNull,
    SetDefault,
    NoAction,
}

/// A foreign key that may point at a table owned by another plugin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForeignKey {
    pub constraint: &'static str,
    pub table: &'static str,
    pub column: &'static str,
    pub ref_table: &'static str,
    pub ref_column: &'static str,
    pub on_update: ReferentialAction,
    pub on_delete: ReferentialAction,
}

pub const fn fk_restrict(
    constraint: &'static str,
    table: &'static str,
    column: &'static str,
    ref_table: &'static str,
) -> ForeignKey {
    ForeignKey {
        constraint,
        table,
        column,
        ref_table,
        ref_column: "id",
        on_update: ReferentialAction::Cascade,
        on_delete: ReferentialAction::Restrict,
    }
}

pub const fn fk_set_null(
    constraint: &'static str,
    table: &'static str,
    column: &'static str,
    ref_table: &'static str,
) -> ForeignKey {
    ForeignKey {
        constraint,
        table,
        column,
        ref_table,
        ref_column: "id",
        on_update: ReferentialAction::Cascade,
        on_delete: ReferentialAction::SetNull,
    }
}

/// Add `key` when both relations and the column exist and no matching foreign key does.
///
/// The `ALTER TABLE` runs through `EXECUTE` and `%I`, so a missing peer table is never parsed.
pub async fn ensure_foreign_key<C: ConnectionTrait>(
    conn: &C,
    backend: DatabaseBackend,
    key: ForeignKey,
) -> Result<(), DbErr> {
    if backend != DatabaseBackend::Postgres {
        return Ok(());
    }
    validate_foreign_key(key)?;
    exec_sql_conn(conn, backend, &foreign_key_sql(key)).await
}

/// Add each key in `keys`. Missing peer tables are skipped.
pub async fn ensure_foreign_keys<C: ConnectionTrait>(
    conn: &C,
    backend: DatabaseBackend,
    keys: &[ForeignKey],
) -> Result<(), DbErr> {
    for key in keys {
        ensure_foreign_key(conn, backend, *key).await?;
    }
    Ok(())
}

/// [`ensure_foreign_keys`] against a migration schema manager.
pub async fn ensure_schema_foreign_keys(
    manager: &SchemaManager<'_>,
    keys: &[ForeignKey],
) -> Result<(), DbErr> {
    ensure_foreign_keys(
        manager.get_connection(),
        manager.get_database_backend(),
        keys,
    )
    .await
}

/// Constraint names in `keys` are distinct valid SQL identifiers.
pub fn foreign_keys_are_unique(keys: &[ForeignKey]) -> bool {
    let mut names = std::collections::HashSet::new();
    keys.iter().all(|key| {
        [
            key.constraint,
            key.table,
            key.column,
            key.ref_table,
            key.ref_column,
        ]
        .into_iter()
        .all(valid_ident)
            && names.insert(key.constraint)
    })
}

fn validate_foreign_key(key: ForeignKey) -> Result<(), DbErr> {
    for name in [
        key.constraint,
        key.table,
        key.column,
        key.ref_table,
        key.ref_column,
    ] {
        if !valid_ident(name) {
            return Err(DbErr::Custom(format!("invalid sql identifier: {name}")));
        }
    }
    Ok(())
}

fn valid_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_') && name.len() <= 63
}

fn action_sql(action: ReferentialAction) -> &'static str {
    match action {
        ReferentialAction::Cascade => "CASCADE",
        ReferentialAction::Restrict => "RESTRICT",
        ReferentialAction::SetNull => "SET NULL",
        ReferentialAction::SetDefault => "SET DEFAULT",
        ReferentialAction::NoAction => "NO ACTION",
    }
}

fn foreign_key_sql(key: ForeignKey) -> String {
    let on_update = action_sql(key.on_update);
    let on_delete = action_sql(key.on_delete);
    format!(
        r#"DO $fk$
DECLARE
    v_table text := '{table}';
    v_column text := '{column}';
    v_constraint text := '{constraint}';
    v_ref_table text := '{ref_table}';
    v_ref_column text := '{ref_column}';
BEGIN
    IF to_regclass('public.' || v_table) IS NULL
       OR to_regclass('public.' || v_ref_table) IS NULL THEN
        RETURN;
    END IF;
    IF NOT EXISTS (
        SELECT 1
          FROM information_schema.columns
         WHERE table_schema = 'public'
           AND table_name = v_table
           AND column_name = v_column
    ) THEN
        RETURN;
    END IF;
    IF EXISTS (SELECT 1 FROM pg_constraint WHERE conname = v_constraint) THEN
        RETURN;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM pg_constraint AS c
          JOIN pg_class AS src ON src.oid = c.conrelid
          JOIN pg_class AS dst ON dst.oid = c.confrelid
          JOIN pg_namespace AS n ON n.oid = src.relnamespace
          JOIN pg_attribute AS a
            ON a.attrelid = c.conrelid
           AND a.attnum = ANY (c.conkey)
         WHERE c.contype = 'f'
           AND n.nspname = 'public'
           AND src.relname = v_table
           AND dst.relname = v_ref_table
           AND a.attname = v_column
    ) THEN
        RETURN;
    END IF;
    EXECUTE format(
        'ALTER TABLE %I ADD CONSTRAINT %I FOREIGN KEY (%I) REFERENCES %I (%I) ON UPDATE {on_update} ON DELETE {on_delete}',
        v_table, v_constraint, v_column, v_ref_table, v_ref_column
    );
END
$fk$;"#,
        table = key.table,
        column = key.column,
        constraint = key.constraint,
        ref_table = key.ref_table,
        ref_column = key.ref_column,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_sql_keeps_a_do_block_between_commands() {
        let sql = r#"
CREATE TABLE IF NOT EXISTS tax_preferences (id bigserial PRIMARY KEY);
DO $$
BEGIN
    INSERT INTO tax_preferences (id) VALUES (1);
END $$;
ALTER TABLE taxes DROP COLUMN IF EXISTS account_id;
"#;
        let statements = split_sql(sql);
        assert_eq!(statements.len(), 3);
        assert!(statements[0].starts_with("CREATE TABLE"));
        assert!(statements[1].starts_with("DO"));
        assert!(statements[1].contains("INSERT INTO tax_preferences"));
        assert!(statements[2].starts_with("ALTER TABLE"));
    }

    #[test]
    fn sqlite_do_block_keeps_plain_ddl() {
        let sql = r#"
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'crm_leads' AND column_name = 'contact_id'
  ) THEN
    ALTER TABLE crm_leads ADD COLUMN IF NOT EXISTS contact_id bigint;
    ALTER TABLE crm_leads DROP COLUMN IF EXISTS email;
  END IF;
  ALTER TABLE crm_leads ADD CONSTRAINT fk_crm_leads_contact_id FOREIGN KEY (contact_id) REFERENCES crm_contacts(id);
END $$;
"#;
        let statements = sqlite_statements(sql);
        assert_eq!(
            statements,
            vec![
                "ALTER TABLE crm_leads ADD COLUMN contact_id bigint".to_string(),
                "ALTER TABLE crm_leads DROP COLUMN email".to_string(),
            ]
        );
    }

    #[test]
    fn sqlite_rewrites_now_and_btrim() {
        assert_eq!(
            rewrite_sqlite_functions("SELECT NOW(), btrim(x), knownow(), FALSE, 'TRUE'"),
            "SELECT CURRENT_TIMESTAMP, trim(x), knownow(), 0, 'TRUE'"
        );
    }

    #[test]
    fn sqlite_script_unwraps_a_nested_do_block() {
        let sql = r#"
CREATE TABLE IF NOT EXISTS tax_preferences (id integer PRIMARY KEY);
DO $$
BEGIN
    ALTER TABLE taxes ADD COLUMN IF NOT EXISTS account_id bigint;
END $$;
"#;
        assert_eq!(
            sqlite_statements(sql),
            vec![
                "CREATE TABLE IF NOT EXISTS tax_preferences (id integer PRIMARY KEY)".to_string(),
                "ALTER TABLE taxes ADD COLUMN account_id bigint".to_string(),
            ]
        );
    }

    #[test]
    fn sqlite_skips_distinct_on() {
        assert!(sqlite_statements(
            "UPDATE source_docs AS sd SET source_doc_id = sub.payment_id FROM (SELECT DISTINCT ON (payment_batch_id) payment_batch_id, id AS payment_id FROM payments) AS sub"
        )
        .is_empty());
    }

    #[test]
    fn sqlite_rewrites_create_table_like_and_skips_sequences() {
        assert_eq!(
            sqlite_statements(
                "CREATE TABLE quotation_invoices (LIKE draft_invoices INCLUDING DEFAULTS)"
            ),
            vec![
                "DROP TABLE IF EXISTS quotation_invoices".to_string(),
                "CREATE TABLE quotation_invoices AS SELECT * FROM draft_invoices WHERE 0"
                    .to_string(),
            ]
        );
        assert!(sqlite_statements("CREATE SEQUENCE quotation_invoices_id_seq").is_empty());
        assert!(
            sqlite_statements(
                "ALTER SEQUENCE quotation_invoices_id_seq OWNED BY quotation_invoices.id"
            )
            .is_empty()
        );
    }

    #[test]
    fn sqlite_skips_quotation_copy_do_block() {
        let sql = r#"
DO $$
BEGIN
    CREATE TEMP TABLE _q_draft_map (quotation_id bigint) ON COMMIT DROP;
    INSERT INTO draft_invoices (id) SELECT nextval(pg_get_serial_sequence('draft_invoices', 'id'));
    SELECT COALESCE(p.name, format('Product #%s', ql.product_id::text));
END $$;
"#;
        assert!(sqlite_statements(sql).is_empty());
    }

    #[test]
    fn sqlite_skips_postgres_triggers() {
        assert!(sqlite_statements(
            "CREATE TRIGGER trg AFTER DELETE ON draft_invoices FOR EACH ROW EXECUTE PROCEDURE delete_term()"
        )
        .is_empty());
        assert_eq!(
            sqlite_statements(
                "DROP TRIGGER IF EXISTS trg_draft_invoices_delete_payment_term ON draft_invoices"
            ),
            vec!["DROP TRIGGER IF EXISTS trg_draft_invoices_delete_payment_term".to_string()]
        );
    }

    #[test]
    fn sqlite_schema_drops_unique_and_fk_column() {
        let create = r#"CREATE TABLE "draft_payment_terms" (
  "id" integer NOT NULL PRIMARY KEY AUTOINCREMENT,
  "draft_invoice_id" integer NOT NULL,
  "created_at" text NOT NULL,
  CONSTRAINT "fk_draft" FOREIGN KEY ("draft_invoice_id") REFERENCES "draft_invoices" ("id"),
  UNIQUE ("draft_invoice_id")
)"#;
        let rewritten = schema_without_column(create, "draft_invoice_id").expect("rewrite");
        assert!(!mentions_ident(&rewritten, "draft_invoice_id"));
        assert!(rewritten.contains("\"id\""));
        assert!(rewritten.contains("\"created_at\""));
        assert_eq!(
            column_names_in_create(&rewritten),
            vec!["id".to_string(), "created_at".to_string()]
        );
    }

    #[test]
    fn sqlite_rewrites_update_target_alias() {
        assert_eq!(
            sqlite_statements(
                "UPDATE draft_invoices d SET draft_payment_term_id = t.id FROM draft_payment_terms t WHERE t.draft_invoice_id = d.id"
            ),
            vec![
                "UPDATE draft_invoices SET draft_payment_term_id = t.id FROM draft_payment_terms t WHERE t.draft_invoice_id = draft_invoices.id"
                    .to_string()
            ]
        );
    }

    #[test]
    fn sqlite_splits_multi_action_alter() {
        assert_eq!(
            sqlite_statements(
                "ALTER TABLE products ADD COLUMN IF NOT EXISTS a bigint NULL, ADD COLUMN IF NOT EXISTS b numeric(19, 6)"
            ),
            vec![
                "ALTER TABLE products ADD COLUMN a bigint NULL".to_string(),
                "ALTER TABLE products ADD COLUMN b numeric(19, 6)".to_string(),
            ]
        );
    }

    #[test]
    fn sqlite_skips_procedural_do_blocks() {
        let sql = r#"
CREATE TABLE IF NOT EXISTS tax_preferences (id integer PRIMARY KEY);
DO $$
BEGIN
    v_has_account := TRUE;
    INSERT INTO tax_preferences (id) VALUES (1);
END $$;
ALTER TABLE taxes DROP COLUMN IF EXISTS account_id;
"#;
        assert_eq!(
            sqlite_statements(sql),
            vec![
                "CREATE TABLE IF NOT EXISTS tax_preferences (id integer PRIMARY KEY)".to_string(),
                "ALTER TABLE taxes DROP COLUMN account_id".to_string(),
            ]
        );
    }
}
