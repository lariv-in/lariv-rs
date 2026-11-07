use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum LlmAssistantPreferences {
    Table,
    EmailFilter,
    EmailOwnerEmail,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for column in [
            ColumnDef::new(LlmAssistantPreferences::EmailFilter)
                .text()
                .not_null()
                .default("")
                .take(),
            ColumnDef::new(LlmAssistantPreferences::EmailOwnerEmail)
                .text()
                .not_null()
                .default("")
                .take(),
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(LlmAssistantPreferences::Table)
                        .add_column(column)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for column in [
            LlmAssistantPreferences::EmailOwnerEmail,
            LlmAssistantPreferences::EmailFilter,
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(LlmAssistantPreferences::Table)
                        .drop_column(column)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}
