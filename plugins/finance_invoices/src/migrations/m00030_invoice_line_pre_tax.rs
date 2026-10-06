//! Store formula inputs and the computed pre-tax amount on invoice lines.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        for table in [
            "draft_invoice_lines",
            "posted_invoice_lines",
            "cancelled_invoice_lines",
        ] {
            conn.execute_unprepared(&format!(
                "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS variable_values text NOT NULL DEFAULT '{{}}'"
            ))
            .await?;
            conn.execute_unprepared(&format!(
                "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS pre_tax_amount numeric(19, 6) NOT NULL DEFAULT 0"
            ))
            .await?;
            conn.execute_unprepared(&format!(
                "UPDATE {table} SET pre_tax_amount = ROUND(quantity * rate, 6)"
            ))
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        for table in [
            "cancelled_invoice_lines",
            "posted_invoice_lines",
            "draft_invoice_lines",
        ] {
            conn.execute_unprepared(&format!(
                "ALTER TABLE {table} DROP COLUMN IF EXISTS pre_tax_amount"
            ))
            .await?;
            conn.execute_unprepared(&format!(
                "ALTER TABLE {table} DROP COLUMN IF EXISTS variable_values"
            ))
            .await?;
        }
        Ok(())
    }
}
