use lariv_core::db::migration_sql::exec_sql;
use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            "DELETE FROM otp_preferences WHERE deleted_at IS NOT NULL",
        )
        .await?;
        exec_sql(
            manager,
            "DROP INDEX IF EXISTS idx_otp_preferences_deleted_at",
        )
        .await?;
        // SQLite accepts DROP COLUMN, but not DROP COLUMN IF EXISTS.
        let drop_deleted_at = match manager.get_database_backend() {
            DbBackend::Postgres => "DROP COLUMN IF EXISTS deleted_at",
            _ => "DROP COLUMN deleted_at",
        };
        exec_sql(
            manager,
            &format!("ALTER TABLE otp_preferences {drop_deleted_at}"),
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let add_deleted_at = match manager.get_database_backend() {
            DbBackend::Postgres => "ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ",
            _ => "ADD COLUMN deleted_at timestamp_with_timezone_text",
        };
        exec_sql(
            manager,
            &format!("ALTER TABLE otp_preferences {add_deleted_at}"),
        )
        .await?;
        exec_sql(
            manager,
            "CREATE INDEX IF NOT EXISTS idx_otp_preferences_deleted_at ON otp_preferences (deleted_at)",
        )
        .await?;
        Ok(())
    }
}
