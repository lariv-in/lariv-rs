//! PostgreSQL `pg_trgm` helpers for fuzzy text search.
//!
//! On Postgres, matches use case-insensitive `word_similarity` (plus `LIKE` so
//! short queries still hit) and results are ranked by trigram score. Other
//! backends use case-insensitive `LIKE`.

use sea_orm::sea_query::{Condition, Expr, ExprTrait, Func, SimpleExpr};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Select,
};

/// Minimum `word_similarity(query, column)` to count as a trigram hit.
pub const WORD_SIMILARITY_THRESHOLD: f64 = 0.25;

/// Default number of rows an AI search tool returns.
pub const DEFAULT_SEARCH_LIMIT: u64 = 20;

/// Hard cap on AI search result rows.
pub const MAX_SEARCH_LIMIT: u64 = 50;

/// Whether `db` is talking to PostgreSQL.
pub fn is_postgres(db: &DatabaseConnection) -> bool {
    matches!(db.get_database_backend(), DatabaseBackend::Postgres)
}

/// Clamp a tool `limit` argument into `[1, MAX_SEARCH_LIMIT]`.
pub fn clamp_search_limit(limit: u64) -> u64 {
    if limit == 0 {
        DEFAULT_SEARCH_LIMIT
    } else {
        limit.min(MAX_SEARCH_LIMIT)
    }
}

/// `CREATE EXTENSION IF NOT EXISTS pg_trgm` on Postgres; no-op elsewhere.
pub async fn enable_extension<C: ConnectionTrait>(
    db: &C,
    backend: DatabaseBackend,
) -> Result<(), DbErr> {
    if !matches!(backend, DatabaseBackend::Postgres) {
        return Ok(());
    }
    db.execute_unprepared("CREATE EXTENSION IF NOT EXISTS pg_trgm")
        .await?;
    Ok(())
}

/// GIN trigram index on a single text column (Postgres only).
///
/// Also ensures `pg_trgm` exists. `index_name`, `table`, and `column` must be
/// simple SQL identifiers (`[A-Za-z0-9_]+`).
pub async fn create_gin_index<C: ConnectionTrait>(
    db: &C,
    backend: DatabaseBackend,
    index_name: &str,
    table: &str,
    column: &str,
) -> Result<(), DbErr> {
    enable_extension(db, backend).await?;
    if !matches!(backend, DatabaseBackend::Postgres) {
        return Ok(());
    }
    assert_ident(index_name);
    assert_ident(table);
    assert_ident(column);
    db.execute_unprepared(&format!(
        "CREATE INDEX IF NOT EXISTS {index_name} ON {table} USING gin ((LOWER({column}::text)) gin_trgm_ops)"
    ))
    .await?;
    Ok(())
}

/// Drop and recreate a case-insensitive trigram GIN index.
pub async fn recreate_gin_index<C: ConnectionTrait>(
    db: &C,
    backend: DatabaseBackend,
    index_name: &str,
    table: &str,
    column: &str,
) -> Result<(), DbErr> {
    drop_gin_index(db, backend, index_name).await?;
    create_gin_index(db, backend, index_name, table, column).await
}

/// Drop a trigram GIN index (Postgres only).
pub async fn drop_gin_index<C: ConnectionTrait>(
    db: &C,
    backend: DatabaseBackend,
    index_name: &str,
) -> Result<(), DbErr> {
    if !matches!(backend, DatabaseBackend::Postgres) {
        return Ok(());
    }
    assert_ident(index_name);
    db.execute_unprepared(&format!("DROP INDEX IF EXISTS {index_name}"))
        .await?;
    Ok(())
}

/// Filter + relevance order for a text query across `columns`.
///
/// Empty `needle` is a no-op. Postgres uses `word_similarity` or `LIKE`;
/// other backends use `LIKE` only.
pub fn apply_text_search<E, C>(
    select: Select<E>,
    backend: DatabaseBackend,
    columns: &[C],
    needle: &str,
) -> Select<E>
where
    E: EntityTrait,
    C: ColumnTrait + Copy,
{
    let needle = needle.trim();
    if needle.is_empty() || columns.is_empty() {
        return select;
    }
    select
        .filter(match_condition(backend, columns, needle))
        .order_by_desc(score_expr(backend, columns, needle))
}

/// Run [`apply_text_search`] and collect up to `limit` rows.
pub async fn search<E, C>(
    db: &DatabaseConnection,
    columns: &[C],
    needle: &str,
    limit: u64,
) -> Result<Vec<E::Model>, DbErr>
where
    E: EntityTrait,
    C: ColumnTrait + Copy,
{
    apply_text_search(E::find(), db.get_database_backend(), columns, needle)
        .limit(clamp_search_limit(limit))
        .all(db)
        .await
}

fn match_condition<C: ColumnTrait + Copy>(
    backend: DatabaseBackend,
    columns: &[C],
    needle: &str,
) -> Condition {
    let postgres = matches!(backend, DatabaseBackend::Postgres);
    let mut cond = Condition::any();
    for col in columns {
        if postgres {
            cond = cond.add(word_similarity_expr(*col, needle).gte(WORD_SIMILARITY_THRESHOLD));
        }
        cond = cond.add(substring_match(*col, needle));
    }
    cond
}

fn score_expr<C: ColumnTrait + Copy>(
    backend: DatabaseBackend,
    columns: &[C],
    needle: &str,
) -> SimpleExpr {
    if !matches!(backend, DatabaseBackend::Postgres) || columns.is_empty() {
        return Expr::val(0).into();
    }
    let scores: Vec<SimpleExpr> = columns
        .iter()
        .map(|col| word_similarity_expr(*col, needle))
        .collect();
    if scores.len() == 1 {
        return scores.into_iter().next().expect("one score");
    }
    Func::greatest(scores).into()
}

fn word_similarity_expr(col: impl sea_orm::sea_query::IntoColumnRef, needle: &str) -> SimpleExpr {
    Func::cust("word_similarity")
        .arg(needle.to_lowercase())
        .arg(lower_col(col))
        .into()
}

fn substring_match(col: impl ColumnTrait, needle: &str) -> SimpleExpr {
    Expr::expr(lower_col(col)).like(like_contains_pattern(needle))
}

fn lower_col(col: impl sea_orm::sea_query::IntoColumnRef) -> SimpleExpr {
    Func::cust("LOWER").arg(Expr::col(col)).into()
}

fn like_contains_pattern(needle: &str) -> String {
    let mut out = String::from("%");
    for c in needle.to_lowercase().chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

fn assert_ident(name: &str) {
    assert!(
        !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "invalid SQL identifier: {name:?}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Iden;
    use sea_orm::sea_query::{PostgresQueryBuilder, Query};

    struct NameCol;

    impl Iden for NameCol {
        fn unquoted(&self, s: &mut dyn std::fmt::Write) {
            let _ = s.write_str("name");
        }
    }

    #[test]
    fn postgres_score_uses_word_similarity() {
        let sql = Query::select()
            .expr(word_similarity_expr(NameCol, "riverside"))
            .to_string(PostgresQueryBuilder);
        assert!(sql.contains("word_similarity"), "{sql}");
        assert!(sql.contains("LOWER") || sql.contains("lower"), "{sql}");
        assert!(sql.contains("riverside"), "{sql}");
    }

    #[test]
    fn like_pattern_is_lowercase_and_escaped() {
        assert_eq!(like_contains_pattern("Riv_er%"), r#"%riv\_er\%%"#);
    }

    #[test]
    fn clamp_limit_defaults_and_caps() {
        assert_eq!(clamp_search_limit(0), DEFAULT_SEARCH_LIMIT);
        assert_eq!(clamp_search_limit(3), 3);
        assert_eq!(clamp_search_limit(999), MAX_SEARCH_LIMIT);
    }

    #[test]
    fn ident_rejects_injection() {
        assert!(std::panic::catch_unwind(|| assert_ident("name")).is_ok());
        assert!(std::panic::catch_unwind(|| assert_ident("name; drop")).is_err());
    }
}
