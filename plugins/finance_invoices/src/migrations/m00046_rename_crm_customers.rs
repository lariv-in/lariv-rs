//! After the legacy `customers` table is gone, rename `crm_customers` → `customers`.
//!
//! Customer `m00009` does the same when safe earlier; this covers fresh installs
//! where the old table blocked that rename until `m00033`.

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
