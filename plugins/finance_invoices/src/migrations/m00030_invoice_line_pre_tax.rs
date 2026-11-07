//! Store formula inputs and the computed pre-tax amount on invoice lines.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            "draft_invoice_lines",
            "posted_invoice_lines",
            "cancelled_invoice_lines",
        ] {
            exec_sql(manager, &format!(
                "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS variable_values text NOT NULL DEFAULT '{{}}'"
            ))
            .await?;
            exec_sql(manager, &format!(
                "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS pre_tax_amount numeric(19, 6) NOT NULL DEFAULT 0"
            ))
            .await?;
            exec_sql(
                manager,
                &format!("UPDATE {table} SET pre_tax_amount = ROUND(quantity * rate, 6)"),
            )
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            "cancelled_invoice_lines",
            "posted_invoice_lines",
            "draft_invoice_lines",
        ] {
            exec_sql(
                manager,
                &format!("ALTER TABLE {table} DROP COLUMN IF EXISTS pre_tax_amount"),
            )
            .await?;
            exec_sql(
                manager,
                &format!("ALTER TABLE {table} DROP COLUMN IF EXISTS variable_values"),
            )
            .await?;
        }
        Ok(())
    }
}
