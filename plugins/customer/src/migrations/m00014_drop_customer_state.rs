//! Drop the GST `state` column from the party customer table.
//!
//! Place of supply comes from the company (or, later, the contact). The legacy
//! address-shaped `customers` table still has `state` until finance copies it
//! onto companies, so this only drops the column from a party table
//! (`is_individual` present) and from `crm_customers`.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
DO $drop_state$
BEGIN
    IF to_regclass('public.crm_customers') IS NOT NULL THEN
        ALTER TABLE crm_customers DROP COLUMN IF EXISTS state;
    END IF;
    IF to_regclass('public.customers') IS NOT NULL
       AND EXISTS (
            SELECT 1
              FROM information_schema.columns
             WHERE table_schema = 'public'
               AND table_name = 'customers'
               AND column_name = 'is_individual'
       ) THEN
        ALTER TABLE customers DROP COLUMN IF EXISTS state;
    END IF;
END
$drop_state$;
"#;

const DOWN: &str = r#"
DO $add_state$
BEGIN
    IF to_regclass('public.crm_customers') IS NOT NULL THEN
        ALTER TABLE crm_customers ADD COLUMN IF NOT EXISTS state text;
    END IF;
    IF to_regclass('public.customers') IS NOT NULL
       AND EXISTS (
            SELECT 1
              FROM information_schema.columns
             WHERE table_schema = 'public'
               AND table_name = 'customers'
               AND column_name = 'is_individual'
       ) THEN
        ALTER TABLE customers ADD COLUMN IF NOT EXISTS state text;
    END IF;
END
$add_state$;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DbBackend::Sqlite {
            return drop_sqlite(manager).await;
        }
        exec_sql(manager, UP).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DbBackend::Sqlite {
            return add_sqlite(manager).await;
        }
        exec_sql(manager, DOWN).await
    }
}

async fn drop_sqlite(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    for table in ["crm_customers", "customers"] {
        if !table_exists(manager, table).await? {
            continue;
        }
        if table == "customers" && !column_exists(manager, table, "is_individual").await? {
            continue;
        }
        if !column_exists(manager, table, "state").await? {
            continue;
        }
        exec_sql(manager, &format!("ALTER TABLE {table} DROP COLUMN state")).await?;
    }
    Ok(())
}

async fn add_sqlite(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    for table in ["crm_customers", "customers"] {
        if !table_exists(manager, table).await? {
            continue;
        }
        if table == "customers" && !column_exists(manager, table, "is_individual").await? {
            continue;
        }
        if column_exists(manager, table, "state").await? {
            continue;
        }
        exec_sql(
            manager,
            &format!("ALTER TABLE {table} ADD COLUMN state text"),
        )
        .await?;
    }
    Ok(())
}

async fn table_exists(manager: &SchemaManager<'_>, table: &str) -> Result<bool, DbErr> {
    let row = manager
        .get_connection()
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            format!("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '{table}'"),
        ))
        .await?;
    Ok(row.is_some())
}

async fn column_exists(
    manager: &SchemaManager<'_>,
    table: &str,
    column: &str,
) -> Result<bool, DbErr> {
    let row = manager
        .get_connection()
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            format!("SELECT 1 FROM pragma_table_info('{table}') WHERE name = '{column}'"),
        ))
        .await?;
    Ok(row.is_some())
}
