//! Store `logs.description` as jsonb. Fresh installs already create that type;
//! this converts databases whose column is still text.

use sea_orm::DatabaseBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
        }
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE logs
                 ALTER COLUMN description TYPE jsonb
                 USING description::jsonb",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
        }
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE logs
                 ALTER COLUMN description TYPE text
                 USING description::text",
            )
            .await?;
        Ok(())
    }
}
