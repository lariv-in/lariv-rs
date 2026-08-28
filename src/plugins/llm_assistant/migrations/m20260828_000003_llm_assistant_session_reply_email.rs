use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum LlmAssistantSessions {
    Table,
    ReplyEmail,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantSessions::Table)
                    .add_column(
                        ColumnDef::new(LlmAssistantSessions::ReplyEmail)
                            .text()
                            .null(),
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
                    .drop_column(LlmAssistantSessions::ReplyEmail)
                    .to_owned(),
            )
            .await
    }
}
