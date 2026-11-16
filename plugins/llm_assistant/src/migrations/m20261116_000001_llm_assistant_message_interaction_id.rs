use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum LlmAssistantSessionMessages {
    Table,
    InteractionId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantSessionMessages::Table)
                    .add_column(ColumnDef::new(LlmAssistantSessionMessages::InteractionId).text())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantSessionMessages::Table)
                    .drop_column(LlmAssistantSessionMessages::InteractionId)
                    .to_owned(),
            )
            .await
    }
}
