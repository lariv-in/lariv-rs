use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmLeadTags {
    Table,
    Color,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_connection().get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            return exec_sql(
                manager,
                "ALTER TABLE crm_lead_tags ADD COLUMN IF NOT EXISTS color TEXT NOT NULL DEFAULT '#6366f1'",
            )
            .await;
        }

        manager
            .alter_table(
                Table::alter()
                    .table(CrmLeadTags::Table)
                    .add_column(
                        ColumnDef::new(CrmLeadTags::Color)
                            .text()
                            .not_null()
                            .default("#6366f1"),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CrmLeadTags::Table)
                    .drop_column(CrmLeadTags::Color)
                    .to_owned(),
            )
            .await
    }
}
