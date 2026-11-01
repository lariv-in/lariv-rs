//! Finish `crm_customers` → `customers` when both tables exist.
//!
//! Checkpoint restores that revive `crm_customers` after this version is already
//! recorded are fixed by `m00011` (same consolidate steps).

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

use super::consolidate_crm_customers;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in consolidate_crm_customers::UP {
            exec_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
