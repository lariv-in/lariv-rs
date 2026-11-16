use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum LlmAssistantTokenUsage {
    Table,
    Model,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantTokenUsage::Table)
                    .add_column(
                        ColumnDef::new(LlmAssistantTokenUsage::Model)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantTokenUsage::Table)
                    .drop_column(LlmAssistantTokenUsage::Model)
                    .to_owned(),
            )
            .await
    }
}
