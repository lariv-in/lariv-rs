//! Product variable schema and price formula.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            "ALTER TABLE products ADD COLUMN IF NOT EXISTS variables text NOT NULL DEFAULT '{}'",
        )
        .await?;
        conn.execute_unprepared(
            "ALTER TABLE products ADD COLUMN IF NOT EXISTS price_formula text NOT NULL DEFAULT ''",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared("ALTER TABLE products DROP COLUMN IF EXISTS price_formula")
            .await?;
        conn.execute_unprepared("ALTER TABLE products DROP COLUMN IF EXISTS variables")
            .await?;
        Ok(())
    }
}
