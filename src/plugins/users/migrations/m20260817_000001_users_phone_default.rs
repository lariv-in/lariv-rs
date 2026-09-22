use crate::db::migration_sql::exec_sql;
use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Unique phones cannot all become ''; stamp a stable per-row default.
        match manager.get_database_backend() {
            DbBackend::Postgres => {
                exec_sql(
                    manager,
                    "UPDATE users SET phone = 'user-' || id::text \
                     WHERE phone IS NULL OR btrim(phone) = ''",
                )
                .await?;
                exec_sql(
                    manager,
                    "ALTER TABLE users ALTER COLUMN phone SET DEFAULT ''",
                )
                .await?;
                exec_sql(manager, "ALTER TABLE users ALTER COLUMN phone SET NOT NULL").await?;
            }
            _ => {
                exec_sql(
                    manager,
                    "UPDATE users SET phone = 'user-' || id \
                     WHERE phone IS NULL OR trim(phone) = ''",
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DbBackend::Postgres {
            exec_sql(
                manager,
                "ALTER TABLE users ALTER COLUMN phone DROP NOT NULL",
            )
            .await?;
            exec_sql(manager, "ALTER TABLE users ALTER COLUMN phone DROP DEFAULT").await?;
        }
        Ok(())
    }
}
