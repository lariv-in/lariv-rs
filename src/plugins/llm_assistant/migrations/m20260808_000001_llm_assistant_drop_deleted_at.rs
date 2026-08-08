use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn exec(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(
            manager.get_connection().get_database_backend(),
            sql.to_string(),
        ))
        .await
        .map(|_| ())
}

async fn purge_and_drop(manager: &SchemaManager<'_>, table: &str) -> Result<(), DbErr> {
    exec(
        manager,
        &format!("DELETE FROM {table} WHERE deleted_at IS NOT NULL"),
    )
    .await?;
    exec(
        manager,
        &format!("ALTER TABLE {table} DROP COLUMN IF EXISTS deleted_at"),
    )
    .await
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec(
            manager,
            "DROP INDEX IF EXISTS idx_llm_assistant_sessions_deleted_at",
        )
        .await?;
        exec(manager, "DROP INDEX IF EXISTS idx_skills_deleted_at").await?;

        // Leaves first: FR payload → FR parts → payload parts → message_parts →
        // messages → sessions; skills; video_metadata last (parts may reference it).
        for table in [
            "llm_assistant_session_message_function_response_blobs",
            "llm_assistant_session_message_function_response_file_data",
            "llm_assistant_session_message_function_response_parts",
            "llm_assistant_session_message_inline_data",
            "llm_assistant_session_message_texts",
            "llm_assistant_session_message_file_data",
            "llm_assistant_session_message_function_calls",
            "llm_assistant_session_message_function_responses",
            "llm_assistant_session_executable_codes",
            "llm_assistant_session_message_code_execution_results",
            "llm_assistant_session_message_tool_calls",
            "llm_assistant_session_message_tool_responses",
            "llm_assistant_session_message_media_resolutions",
            "llm_assistant_session_message_parts",
            "llm_assistant_session_messages",
            "llm_assistant_sessions",
            "skills",
            "video_metadata",
        ] {
            purge_and_drop(manager, table).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            "video_metadata",
            "skills",
            "llm_assistant_sessions",
            "llm_assistant_session_messages",
            "llm_assistant_session_message_parts",
            "llm_assistant_session_message_inline_data",
            "llm_assistant_session_message_texts",
            "llm_assistant_session_message_file_data",
            "llm_assistant_session_message_function_calls",
            "llm_assistant_session_message_function_responses",
            "llm_assistant_session_message_function_response_parts",
            "llm_assistant_session_message_function_response_blobs",
            "llm_assistant_session_message_function_response_file_data",
            "llm_assistant_session_executable_codes",
            "llm_assistant_session_message_code_execution_results",
            "llm_assistant_session_message_tool_calls",
            "llm_assistant_session_message_tool_responses",
            "llm_assistant_session_message_media_resolutions",
        ] {
            exec(
                manager,
                &format!("ALTER TABLE {table} ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ"),
            )
            .await?;
        }
        exec(
            manager,
            "CREATE INDEX IF NOT EXISTS idx_llm_assistant_sessions_deleted_at ON llm_assistant_sessions (deleted_at)",
        )
        .await?;
        exec(
            manager,
            "CREATE INDEX IF NOT EXISTS idx_skills_deleted_at ON skills (deleted_at)",
        )
        .await?;
        Ok(())
    }
}
