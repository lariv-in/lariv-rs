use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum FilesystemNodes {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum LlmAssistantGeminiFileCache {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    VnodeId,
    ContentSha256,
    FileUri,
    FileName,
    MimeType,
    DisplayName,
    ExpiresAt,
}

#[derive(DeriveIden)]
enum FrFileData {
    #[sea_orm(iden = "llm_assistant_session_message_function_response_file_data")]
    Table,
    VnodeId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LlmAssistantGeminiFileCache::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LlmAssistantGeminiFileCache::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantGeminiFileCache::CreatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantGeminiFileCache::UpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantGeminiFileCache::VnodeId)
                            .big_integer()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantGeminiFileCache::ContentSha256)
                            .blob()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantGeminiFileCache::FileUri)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantGeminiFileCache::FileName)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantGeminiFileCache::MimeType)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantGeminiFileCache::DisplayName)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantGeminiFileCache::ExpiresAt)
                            .timestamp_with_time_zone(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_llm_assistant_gemini_file_cache_vnode_id")
                            .from(
                                LlmAssistantGeminiFileCache::Table,
                                LlmAssistantGeminiFileCache::VnodeId,
                            )
                            .to(FilesystemNodes::Table, FilesystemNodes::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(FrFileData::Table)
                    .add_column(ColumnDef::new(FrFileData::VnodeId).big_integer().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(FrFileData::Table)
                    .drop_column(FrFileData::VnodeId)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(LlmAssistantGeminiFileCache::Table)
                    .to_owned(),
            )
            .await
    }
}
