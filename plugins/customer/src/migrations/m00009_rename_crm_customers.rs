//! Rename `crm_customers` → `customers` when the legacy `customers` table is gone.
//!
//! Fresh installs still have the old address-shaped `customers` until finance
//! drops it, so this is a no-op there; finance also runs the same rename after
//! that drop.

use lariv_core::db::migration_sql::exec_sql;
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
        exec_sql(manager, UP).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, DOWN).await
    }
}
