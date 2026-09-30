use crate::db::migration_sql::exec_sql;
use crate::plugins::filesystem::permissions::NodePermissions;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum LlmAssistantPreferences {
    Table,
    ChatAttachmentsParentId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantPreferences::Table)
                    .add_column(
                        ColumnDef::new(LlmAssistantPreferences::ChatAttachmentsParentId)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Seed `/chat_attachments` at filesystem root when missing.
        // Filesystem migrations run first and leave `permissions` NOT NULL with no default.
        let permissions = NodePermissions::for_directory().bits();
        exec_sql(
            manager,
            &format!(
                r#"
INSERT INTO filesystem_nodes (created_at, updated_at, name, is_directory, file_path, parent_id, permissions)
SELECT CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 'chat_attachments', TRUE, NULL, NULL, {permissions}
WHERE NOT EXISTS (
    SELECT 1 FROM filesystem_nodes
    WHERE name = 'chat_attachments'
      AND parent_id IS NULL
      AND is_directory = TRUE
)
"#
            ),
        )
        .await?;

        // Point the singleton prefs row at that folder when unset.
        exec_sql(
            manager,
            r#"
UPDATE llm_assistant_preferences
SET chat_attachments_parent_id = (
    SELECT id FROM filesystem_nodes
    WHERE name = 'chat_attachments'
      AND parent_id IS NULL
      AND is_directory = TRUE
    LIMIT 1
)
WHERE id = 1
  AND chat_attachments_parent_id IS NULL
"#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LlmAssistantPreferences::Table)
                    .drop_column(LlmAssistantPreferences::ChatAttachmentsParentId)
                    .to_owned(),
            )
            .await
    }
}
