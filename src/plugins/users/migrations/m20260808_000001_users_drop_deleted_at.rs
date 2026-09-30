use crate::db::migration_sql::exec_sql;
use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Soft-deleted users first (may be referenced by other plugins via CASCADE FKs).
        exec_sql(manager, "DELETE FROM users WHERE deleted_at IS NOT NULL").await?;
        exec_sql(manager, "DELETE FROM roles WHERE deleted_at IS NOT NULL").await?;

        exec_sql(manager, "DROP INDEX IF EXISTS idx_users_deleted_at").await?;
        exec_sql(manager, "DROP INDEX IF EXISTS idx_roles_deleted_at").await?;

        // SQLite accepts DROP COLUMN, but not DROP COLUMN IF EXISTS.
        let drop_deleted_at = match manager.get_database_backend() {
            DbBackend::Postgres => "DROP COLUMN IF EXISTS deleted_at",
            _ => "DROP COLUMN deleted_at",
        };
        exec_sql(manager, &format!("ALTER TABLE users {drop_deleted_at}")).await?;
        exec_sql(manager, &format!("ALTER TABLE roles {drop_deleted_at}")).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let add_deleted_at = match manager.get_database_backend() {
            DbBackend::Postgres => "ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ",
            _ => "ADD COLUMN deleted_at timestamp_with_timezone_text",
        };
        exec_sql(manager, &format!("ALTER TABLE roles {add_deleted_at}")).await?;
        exec_sql(manager, &format!("ALTER TABLE users {add_deleted_at}")).await?;
        exec_sql(
            manager,
            "CREATE INDEX IF NOT EXISTS idx_roles_deleted_at ON roles (deleted_at)",
        )
        .await?;
        exec_sql(
            manager,
            "CREATE INDEX IF NOT EXISTS idx_users_deleted_at ON users (deleted_at)",
        )
        .await?;
        Ok(())
    }
}
