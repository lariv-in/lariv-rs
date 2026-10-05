use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum LlmAssistantSessions {
    Table,
    Id,
    EmailMessageId,
    EmailReferences,
}

#[derive(DeriveIden)]
enum LlmAssistantProcessedEmails {
    Table,
    Id,
    MessageId,
    ImapUid,
    SessionId,
    ProcessedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantSessions::Table)
                    .add_column(
                        ColumnDef::new(LlmAssistantSessions::EmailMessageId)
                            .text()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(LlmAssistantSessions::EmailReferences)
                            .text()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(LlmAssistantProcessedEmails::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LlmAssistantProcessedEmails::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantProcessedEmails::MessageId)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantProcessedEmails::ImapUid)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantProcessedEmails::SessionId)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantProcessedEmails::ProcessedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_llm_assistant_processed_emails_session_id")
                            .from(
                                LlmAssistantProcessedEmails::Table,
                                LlmAssistantProcessedEmails::SessionId,
                            )
                            .to(LlmAssistantSessions::Table, LlmAssistantSessions::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(LlmAssistantProcessedEmails::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantSessions::Table)
                    .drop_column(LlmAssistantSessions::EmailReferences)
                    .drop_column(LlmAssistantSessions::EmailMessageId)
                    .to_owned(),
            )
            .await
    }
}
