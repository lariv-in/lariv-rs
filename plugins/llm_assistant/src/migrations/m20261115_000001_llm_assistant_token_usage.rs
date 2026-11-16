use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum LlmAssistantTokenUsage {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    UserId,
    UsedAt,
    Tokens,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LlmAssistantTokenUsage::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LlmAssistantTokenUsage::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantTokenUsage::CreatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantTokenUsage::UpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantTokenUsage::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantTokenUsage::UsedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantTokenUsage::Tokens)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_llm_assistant_token_usage_user_id")
                            .from(
                                LlmAssistantTokenUsage::Table,
                                LlmAssistantTokenUsage::UserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_llm_assistant_token_usage_user_used_at")
                    .table(LlmAssistantTokenUsage::Table)
                    .col(LlmAssistantTokenUsage::UserId)
                    .col(LlmAssistantTokenUsage::UsedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(LlmAssistantTokenUsage::Table)
                    .to_owned(),
            )
            .await
    }
}
