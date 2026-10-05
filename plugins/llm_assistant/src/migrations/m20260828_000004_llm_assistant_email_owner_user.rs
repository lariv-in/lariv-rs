use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum LlmAssistantPreferences {
    Table,
    EmailOwnerEmail,
    EmailOwnerUserId,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantPreferences::Table)
                    .add_column(
                        ColumnDef::new(LlmAssistantPreferences::EmailOwnerUserId)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantPreferences::Table)
                    .drop_column(LlmAssistantPreferences::EmailOwnerEmail)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_llm_assistant_prefs_email_owner_user_id")
                    .from(
                        LlmAssistantPreferences::Table,
                        LlmAssistantPreferences::EmailOwnerUserId,
                    )
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_llm_assistant_prefs_email_owner_user_id")
                    .table(LlmAssistantPreferences::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantPreferences::Table)
                    .add_column(
                        ColumnDef::new(LlmAssistantPreferences::EmailOwnerEmail)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .drop_column(LlmAssistantPreferences::EmailOwnerUserId)
                    .to_owned(),
            )
            .await
    }
}
