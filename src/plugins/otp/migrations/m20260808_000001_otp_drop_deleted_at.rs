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

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec(
            manager,
            "DELETE FROM otp_preferences WHERE deleted_at IS NOT NULL",
        )
        .await?;
        exec(
            manager,
            "DROP INDEX IF EXISTS idx_otp_preferences_deleted_at",
        )
        .await?;
        exec(
            manager,
            "ALTER TABLE otp_preferences DROP COLUMN IF EXISTS deleted_at",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec(
            manager,
            "ALTER TABLE otp_preferences ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ",
        )
        .await?;
        exec(
            manager,
            "CREATE INDEX IF NOT EXISTS idx_otp_preferences_deleted_at ON otp_preferences (deleted_at)",
        )
        .await?;
        Ok(())
    }
}
