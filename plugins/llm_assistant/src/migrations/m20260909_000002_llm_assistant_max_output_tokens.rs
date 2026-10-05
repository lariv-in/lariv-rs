use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum LlmAssistantPreferences {
    Table,
    MaxOutputTokens,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantPreferences::Table)
                    .add_column(
                        ColumnDef::new(LlmAssistantPreferences::MaxOutputTokens)
                            .integer()
                            .not_null()
                            .default(65_536),
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
                    .drop_column(LlmAssistantPreferences::MaxOutputTokens)
                    .to_owned(),
            )
            .await
    }
}
