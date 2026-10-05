use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum LlmAssistantPreferences {
    Table,
    ImapServer,
    ImapPort,
    SmtpServer,
    SmtpPort,
    Email,
    Password,
    MailEncryption,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantPreferences::Table)
                    .add_column(
                        ColumnDef::new(LlmAssistantPreferences::ImapServer)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .add_column(
                        ColumnDef::new(LlmAssistantPreferences::ImapPort)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .add_column(
                        ColumnDef::new(LlmAssistantPreferences::SmtpServer)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .add_column(
                        ColumnDef::new(LlmAssistantPreferences::SmtpPort)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .add_column(
                        ColumnDef::new(LlmAssistantPreferences::Email)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .add_column(
                        ColumnDef::new(LlmAssistantPreferences::Password)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .add_column(
                        ColumnDef::new(LlmAssistantPreferences::MailEncryption)
                            .text()
                            .not_null()
                            .default("tls"),
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
                    .drop_column(LlmAssistantPreferences::MailEncryption)
                    .drop_column(LlmAssistantPreferences::Password)
                    .drop_column(LlmAssistantPreferences::Email)
                    .drop_column(LlmAssistantPreferences::SmtpPort)
                    .drop_column(LlmAssistantPreferences::SmtpServer)
                    .drop_column(LlmAssistantPreferences::ImapPort)
                    .drop_column(LlmAssistantPreferences::ImapServer)
                    .to_owned(),
            )
            .await
    }
}
