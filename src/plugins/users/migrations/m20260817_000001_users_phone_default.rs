use sea_orm::{DbBackend, Statement};
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
        // Unique phones cannot all become ''; stamp a stable per-row default.
        match manager.get_database_backend() {
            DbBackend::Postgres => {
                exec(
                    manager,
                    "UPDATE users SET phone = 'user-' || id::text \
                     WHERE phone IS NULL OR btrim(phone) = ''",
                )
                .await?;
                exec(
                    manager,
                    "ALTER TABLE users ALTER COLUMN phone SET DEFAULT ''",
                )
                .await?;
                exec(manager, "ALTER TABLE users ALTER COLUMN phone SET NOT NULL").await?;
            }
            _ => {
                exec(
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
            exec(
                manager,
                "ALTER TABLE users ALTER COLUMN phone DROP NOT NULL",
            )
            .await?;
            exec(manager, "ALTER TABLE users ALTER COLUMN phone DROP DEFAULT").await?;
        }
        Ok(())
    }
}
