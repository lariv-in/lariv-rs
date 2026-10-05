use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum LlmAssistantPreferences {
    Table,
    EmailAttachmentsParentId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantPreferences::Table)
                    .add_column(
                        ColumnDef::new(LlmAssistantPreferences::EmailAttachmentsParentId)
                            .big_integer()
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
                    .table(LlmAssistantPreferences::Table)
                    .drop_column(LlmAssistantPreferences::EmailAttachmentsParentId)
                    .to_owned(),
            )
            .await
    }
}
