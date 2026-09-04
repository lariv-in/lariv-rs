use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum LlmAssistantSessions {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum LlmAssistantSessionCompactions {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    LlmAssistantSessionId,
    ThroughMessageId,
    Summary,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LlmAssistantSessionCompactions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LlmAssistantSessionCompactions::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantSessionCompactions::CreatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantSessionCompactions::UpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantSessionCompactions::LlmAssistantSessionId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantSessionCompactions::ThroughMessageId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantSessionCompactions::Summary)
                            .text()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_llm_assistant_session_compactions_session_id")
                            .from(
                                LlmAssistantSessionCompactions::Table,
                                LlmAssistantSessionCompactions::LlmAssistantSessionId,
                            )
                            .to(LlmAssistantSessions::Table, LlmAssistantSessions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_llm_assistant_session_compactions_session_id")
                    .table(LlmAssistantSessionCompactions::Table)
                    .col(LlmAssistantSessionCompactions::LlmAssistantSessionId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(LlmAssistantSessionCompactions::Table)
                    .to_owned(),
            )
            .await
    }
}
