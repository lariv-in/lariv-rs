//! Quantity on invoice lines is a formula measure: kind, magnitude, and unit.
//!
//! Existing rows stay a count (`quantity` / `Number`). Quotation lines share
//! the draft line editor, so they get the same columns.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const LINE_TABLES: [&str; 4] = [
    "draft_invoice_lines",
    "posted_invoice_lines",
    "cancelled_invoice_lines",
    "quotation_invoice_lines",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in LINE_TABLES {
            exec_sql(
                manager,
                &format!(
                    "ALTER TABLE {table} ADD COLUMN qty_kind text NOT NULL DEFAULT 'quantity'"
                ),
            )
            .await?;
            exec_sql(
                manager,
                &format!("ALTER TABLE {table} ADD COLUMN qty_unit text NOT NULL DEFAULT 'Number'"),
            )
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in LINE_TABLES {
            exec_sql(
                manager,
                &format!("ALTER TABLE {table} DROP COLUMN qty_kind"),
            )
            .await?;
            exec_sql(
                manager,
                &format!("ALTER TABLE {table} DROP COLUMN qty_unit"),
            )
            .await?;
        }
        Ok(())
    }
}
