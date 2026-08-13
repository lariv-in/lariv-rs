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
        // Soft-deleted users first (may be referenced by other plugins via CASCADE FKs).
        exec(manager, "DELETE FROM users WHERE deleted_at IS NOT NULL").await?;
        exec(manager, "DELETE FROM roles WHERE deleted_at IS NOT NULL").await?;

        exec(manager, "DROP INDEX IF EXISTS idx_users_deleted_at").await?;
        exec(manager, "DROP INDEX IF EXISTS idx_roles_deleted_at").await?;

        exec(
            manager,
            "ALTER TABLE users DROP COLUMN IF EXISTS deleted_at",
        )
        .await?;
        exec(
            manager,
            "ALTER TABLE roles DROP COLUMN IF EXISTS deleted_at",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec(
            manager,
            "ALTER TABLE roles ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ",
        )
        .await?;
        exec(
            manager,
            "ALTER TABLE users ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ",
        )
        .await?;
        exec(
            manager,
            "CREATE INDEX IF NOT EXISTS idx_roles_deleted_at ON roles (deleted_at)",
        )
        .await?;
        exec(
            manager,
            "CREATE INDEX IF NOT EXISTS idx_users_deleted_at ON users (deleted_at)",
        )
        .await?;
        Ok(())
    }
}
