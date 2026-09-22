use crate::db::migration_sql::exec_sql;
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

        exec_sql(
            manager,
            "ALTER TABLE users DROP COLUMN IF EXISTS deleted_at",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE roles DROP COLUMN IF EXISTS deleted_at",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            "ALTER TABLE roles ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE users ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ",
        )
        .await?;
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
