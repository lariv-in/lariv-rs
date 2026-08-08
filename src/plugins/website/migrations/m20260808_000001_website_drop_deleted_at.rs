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
            r#"DELETE FROM p_website_route_references
               WHERE db_route_id IN (SELECT id FROM db_routes WHERE deleted_at IS NOT NULL)"#,
        )
        .await?;
        exec(manager, "DELETE FROM db_routes WHERE deleted_at IS NOT NULL").await?;

        exec(manager, "DROP INDEX IF EXISTS idx_db_routes_deleted_at").await?;
        exec(
            manager,
            "ALTER TABLE db_routes DROP COLUMN IF EXISTS deleted_at",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec(
            manager,
            "ALTER TABLE db_routes ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ",
        )
        .await?;
        exec(
            manager,
            "CREATE INDEX IF NOT EXISTS idx_db_routes_deleted_at ON db_routes (deleted_at)",
        )
        .await?;
        Ok(())
    }
}
