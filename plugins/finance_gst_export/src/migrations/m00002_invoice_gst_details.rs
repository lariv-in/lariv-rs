//! GST supply flag on each invoice line.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in LINE_TABLES {
            exec_sql(
                manager,
                &format!(
                    "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS gst_supply text NOT NULL DEFAULT 'taxable'"
                ),
            )
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in LINE_TABLES {
            exec_sql(
                manager,
                &format!("ALTER TABLE {table} DROP COLUMN IF EXISTS gst_supply"),
            )
            .await?;
        }
        Ok(())
    }
}

const LINE_TABLES: &[&str] = &[
    "draft_invoice_lines",
    "posted_invoice_lines",
    "cancelled_invoice_lines",
    "quotation_invoice_lines",
];
