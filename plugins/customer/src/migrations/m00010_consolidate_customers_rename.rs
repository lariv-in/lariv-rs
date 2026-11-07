//! Finish `crm_customers` → `customers` when both tables exist.
//!
//! Checkpoint restores that revive `crm_customers` after this version is already
//! recorded are fixed by `m00011` (same consolidate steps).

use lariv_core::db::migration_sql::exec_sql;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

use super::consolidate_crm_customers;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        consolidate_crm_customers::apply(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DbBackend::Sqlite {
            let crm_shaped = manager
                .get_connection()
                .query_one_raw(Statement::from_string(
                    DbBackend::Sqlite,
                    "SELECT 1 FROM pragma_table_info('customers') WHERE name = 'is_individual'",
                ))
                .await?
                .is_some();
            let crm_table = manager
                .get_connection()
                .query_one_raw(Statement::from_string(
                    DbBackend::Sqlite,
                    "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'crm_customers'",
                ))
                .await?
                .is_some();
            if crm_shaped && !crm_table {
                exec_sql(manager, "ALTER TABLE customers RENAME TO crm_customers").await?;
            }
            return Ok(());
        }
        exec_sql(
            manager,
            r#"DO $fix$
BEGIN
    IF to_regclass('public.customers') IS NOT NULL
       AND to_regclass('public.crm_customers') IS NULL THEN
        ALTER TABLE customers RENAME TO crm_customers;
    END IF;
END
$fix$"#,
        )
        .await
    }
}
