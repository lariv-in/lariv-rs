//! Canonical SeaORM 2.x helpers for plugin migrations.
//!
//! Prefer these patterns in new migrations:
//! - **SeaQuery DDL/DML:** `manager.create_table(...)`, `conn.execute(&query)`, etc.
//! - **Raw SQL string:** `exec_sql(manager, sql).await?`
//! - **Runtime raw SQL:** `exec_sql_conn(conn, backend, sql).await?`

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement};
use sea_orm_migration::prelude::*;

/// Whether the migration connection targets PostgreSQL.
pub fn is_postgres(manager: &SchemaManager<'_>) -> bool {
    manager.get_database_backend() == DatabaseBackend::Postgres
}

/// Execute a raw SQL string during a migration (`execute_raw` wrapper).
pub async fn exec_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute_raw(Statement::from_string(manager.get_database_backend(), sql))
        .await
        .map(|_| ())
}

/// Execute a raw SQL string on any connection (tests, conditional DDL wrappers).
pub async fn exec_sql_conn<C: ConnectionTrait>(
    conn: &C,
    backend: DatabaseBackend,
    sql: &str,
) -> Result<(), DbErr> {
    conn.execute_raw(Statement::from_string(backend, sql))
        .await
        .map(|_| ())
}
