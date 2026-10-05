//! Replace `users.is_superuser` with the built-in `superuser` role.
//!
//! Runs after `m20261003_000002_role_names`, which adds `users.role`.
//! A superuser's previous role name is replaced by `superuser`.

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
            "UPDATE users SET role = 'superuser' WHERE is_superuser = TRUE",
        )
        .await?;

        let drop_col = match manager.get_database_backend() {
            DbBackend::Postgres => "DROP COLUMN IF EXISTS is_superuser",
            _ => "DROP COLUMN is_superuser",
        };
        exec_sql(manager, &format!("ALTER TABLE users {drop_col}")).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let add_col = match manager.get_database_backend() {
            DbBackend::Postgres => {
                "ADD COLUMN IF NOT EXISTS is_superuser BOOLEAN NOT NULL DEFAULT FALSE"
            }
            _ => "ADD COLUMN is_superuser INTEGER NOT NULL DEFAULT 0",
        };
        exec_sql(manager, &format!("ALTER TABLE users {add_col}")).await?;
        exec_sql(
            manager,
            "UPDATE users SET is_superuser = TRUE WHERE role = 'superuser'",
        )
        .await?;
        Ok(())
    }
}
