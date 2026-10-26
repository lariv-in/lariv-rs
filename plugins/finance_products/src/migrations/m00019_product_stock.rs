//! Bind each product to an inventory stock when that plugin is installed.
//!
//! Without `inventory_stocks`, the column is `NOT NULL DEFAULT 0` so the
//! product entity can load. The foreign key is attached only once the stock
//! table exists.

use lariv_core::db::migration_sql::{self, FK_PRODUCTS_STOCK};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            r#"DO $$
BEGIN
    IF to_regclass('public.inventory_stocks') IS NULL THEN
        ALTER TABLE products
            ADD COLUMN IF NOT EXISTS stock_id bigint NOT NULL DEFAULT 0;
    ELSE
        ALTER TABLE products
            ADD COLUMN IF NOT EXISTS stock_id bigint NULL;
    END IF;
END $$"#,
        )
        .await?;
        migration_sql::ensure_schema_foreign_keys(manager, &[FK_PRODUCTS_STOCK]).await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_products_stock_id ON products (stock_id)",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared("ALTER TABLE products DROP COLUMN IF EXISTS stock_id")
            .await?;
        Ok(())
    }
}
