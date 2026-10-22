//! Bind each product to an inventory stock. The column stays nullable until
//! invoice and purchase migrations have moved service-typed products off this
//! table and every remaining product has a stock.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            r#"ALTER TABLE products
                ADD COLUMN IF NOT EXISTS stock_id bigint NULL
                    REFERENCES inventory_stocks(id) ON UPDATE CASCADE ON DELETE RESTRICT"#,
        )
        .await?;
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
