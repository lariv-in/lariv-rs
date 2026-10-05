use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum LlmAssistantSessions {
    Table,
    IsSubagent,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantSessions::Table)
                    .add_column(
                        ColumnDef::new(LlmAssistantSessions::IsSubagent)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantSessions::Table)
                    .drop_column(LlmAssistantSessions::IsSubagent)
                    .to_owned(),
            )
            .await
    }
}
