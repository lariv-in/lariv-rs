//! GST advance flag, place of supply, advance taxes, and adjustments.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            "ALTER TABLE payments ADD COLUMN IF NOT EXISTS is_gst_advance boolean NOT NULL DEFAULT false",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE payments ADD COLUMN IF NOT EXISTS gst_pos text",
        )
        .await?;
        exec_sql(
            manager,
            "CREATE TABLE IF NOT EXISTS payment_gst_taxes ( \
                payment_id bigint NOT NULL REFERENCES payments(id) ON DELETE CASCADE, \
                tax_id bigint NOT NULL REFERENCES taxes(id) ON DELETE CASCADE, \
                PRIMARY KEY (payment_id, tax_id) \
            )",
        )
        .await?;
        exec_sql(
            manager,
            "CREATE TABLE IF NOT EXISTS gst_advance_adjustments ( \
                id bigserial PRIMARY KEY, \
                payment_id bigint NOT NULL REFERENCES payments(id) ON DELETE CASCADE, \
                adjusted_on date NOT NULL, \
                gross_amount numeric(19, 6) NOT NULL \
            )",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, "DROP TABLE IF EXISTS gst_advance_adjustments").await?;
        exec_sql(manager, "DROP TABLE IF EXISTS payment_gst_taxes").await?;
        exec_sql(
            manager,
            "ALTER TABLE payments DROP COLUMN IF EXISTS gst_pos",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE payments DROP COLUMN IF EXISTS is_gst_advance",
        )
        .await?;
        Ok(())
    }
}
