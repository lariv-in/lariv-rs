use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            "CREATE TABLE IF NOT EXISTS gst_tax_components ( \
                tax_id bigint PRIMARY KEY REFERENCES taxes(id) ON DELETE CASCADE, \
                component text NOT NULL CHECK (component IN ('igst', 'cgst', 'sgst', 'cess')) \
            )",
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, "DROP TABLE IF EXISTS gst_tax_components").await
    }
}
