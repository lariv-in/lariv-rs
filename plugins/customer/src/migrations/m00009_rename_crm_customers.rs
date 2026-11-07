//! Rename `crm_customers` → `customers` when the legacy `customers` table is gone.
//!
//! Fresh installs still have the old address-shaped `customers` until finance
//! drops it, so this is a no-op there; finance also runs the same rename after
//! that drop.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
DO $rename$
BEGIN
    IF to_regclass('public.crm_customers') IS NOT NULL
       AND to_regclass('public.customers') IS NULL THEN
        ALTER TABLE crm_customers RENAME TO customers;
    END IF;
END
$rename$;
"#;

const DOWN: &str = r#"
DO $rename$
BEGIN
    IF to_regclass('public.customers') IS NOT NULL
       AND to_regclass('public.crm_customers') IS NULL THEN
        ALTER TABLE customers RENAME TO crm_customers;
    END IF;
END
$rename$;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DbBackend::Sqlite {
            // Legacy `customers` is still present, so this stays a no-op.
            if table_exists(manager, "crm_customers").await?
                && !table_exists(manager, "customers").await?
            {
                exec_sql(manager, "ALTER TABLE crm_customers RENAME TO customers").await?;
            }
            return Ok(());
        }
        exec_sql(manager, UP).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DbBackend::Sqlite {
            if table_exists(manager, "customers").await?
                && !table_exists(manager, "crm_customers").await?
            {
                exec_sql(manager, "ALTER TABLE customers RENAME TO crm_customers").await?;
            }
            return Ok(());
        }
        exec_sql(manager, DOWN).await
    }
}

async fn table_exists(manager: &SchemaManager<'_>, table: &str) -> Result<bool, DbErr> {
    let row = manager
        .get_connection()
        .query_one_raw(Statement::from_string(
            manager.get_database_backend(),
            format!("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '{table}'"),
        ))
        .await?;
    Ok(row.is_some())
}
