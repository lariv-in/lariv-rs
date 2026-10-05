use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum FilesystemNodes {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum LlmAssistantSessions {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum LlmAssistantSessionVnodeReads {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    SessionId,
    VnodeId,
    ReadAt,
    VnodeUpdatedAt,
    ContentSha256,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LlmAssistantSessionVnodeReads::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LlmAssistantSessionVnodeReads::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantSessionVnodeReads::CreatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantSessionVnodeReads::UpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantSessionVnodeReads::SessionId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantSessionVnodeReads::VnodeId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantSessionVnodeReads::ReadAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantSessionVnodeReads::VnodeUpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantSessionVnodeReads::ContentSha256)
                            .blob()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_llm_assistant_session_vnode_reads_session_id")
                            .from(
                                LlmAssistantSessionVnodeReads::Table,
                                LlmAssistantSessionVnodeReads::SessionId,
                            )
                            .to(LlmAssistantSessions::Table, LlmAssistantSessions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_llm_assistant_session_vnode_reads_vnode_id")
                            .from(
                                LlmAssistantSessionVnodeReads::Table,
                                LlmAssistantSessionVnodeReads::VnodeId,
                            )
                            .to(FilesystemNodes::Table, FilesystemNodes::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_llm_assistant_session_vnode_reads_session_vnode")
                    .table(LlmAssistantSessionVnodeReads::Table)
                    .col(LlmAssistantSessionVnodeReads::SessionId)
                    .col(LlmAssistantSessionVnodeReads::VnodeId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(LlmAssistantSessionVnodeReads::Table)
                    .to_owned(),
            )
            .await
    }
}
