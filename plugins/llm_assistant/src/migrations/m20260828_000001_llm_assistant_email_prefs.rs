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
        // SQLite allows one ALTER TABLE option per statement.
        let columns = [
            text_column(LlmAssistantPreferences::ImapServer, ""),
            text_column(LlmAssistantPreferences::ImapPort, ""),
            text_column(LlmAssistantPreferences::SmtpServer, ""),
            text_column(LlmAssistantPreferences::SmtpPort, ""),
            text_column(LlmAssistantPreferences::Email, ""),
            text_column(LlmAssistantPreferences::Password, ""),
            text_column(LlmAssistantPreferences::MailEncryption, "tls"),
        ];
        for column in columns {
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
            LlmAssistantPreferences::MailEncryption,
            LlmAssistantPreferences::Password,
            LlmAssistantPreferences::Email,
            LlmAssistantPreferences::SmtpPort,
            LlmAssistantPreferences::SmtpServer,
            LlmAssistantPreferences::ImapPort,
            LlmAssistantPreferences::ImapServer,
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

fn text_column(name: impl IntoIden, default: &str) -> ColumnDef {
    ColumnDef::new(name)
        .text()
        .not_null()
        .default(default)
        .take()
}
