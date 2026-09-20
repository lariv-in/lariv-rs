use crate::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;


#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager,
            "DELETE FROM otp_preferences WHERE deleted_at IS NOT NULL",
        )
        .await?;
        exec_sql(manager,
            "DROP INDEX IF EXISTS idx_otp_preferences_deleted_at",
        )
        .await?;
        exec_sql(manager,
            "ALTER TABLE otp_preferences DROP COLUMN IF EXISTS deleted_at",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager,
            "ALTER TABLE otp_preferences ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ",
        )
        .await?;
        exec_sql(manager,
            "CREATE INDEX IF NOT EXISTS idx_otp_preferences_deleted_at ON otp_preferences (deleted_at)",
        )
        .await?;
        Ok(())
    }
}
