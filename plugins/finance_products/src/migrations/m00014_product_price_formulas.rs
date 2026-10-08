//! Replace numeric base cost and sales price with price formulas.
//!
//! Existing amounts are stored as their numeric text. A non-empty `price_formula`
//! becomes the sales price formula instead of the numeric sales price.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            "ALTER TABLE products ADD COLUMN IF NOT EXISTS base_price_formula text NOT NULL DEFAULT ''",
        )
        .await?;
        conn.execute_unprepared(
            "ALTER TABLE products ADD COLUMN IF NOT EXISTS sales_price_formula text NOT NULL DEFAULT ''",
        )
        .await?;
        conn.execute_unprepared(
            r#"UPDATE products SET
                base_price_formula = trim(trailing '.' from trim(trailing '0' from base_cost::text)),
                sales_price_formula = CASE
                    WHEN btrim(price_formula) <> '' THEN price_formula
                    ELSE trim(trailing '.' from trim(trailing '0' from sales_price::text))
                END"#,
        )
        .await?;
        conn.execute_unprepared("ALTER TABLE products DROP COLUMN IF EXISTS base_cost")
            .await?;
        conn.execute_unprepared("ALTER TABLE products DROP COLUMN IF EXISTS sales_price")
            .await?;
        conn.execute_unprepared("ALTER TABLE products DROP COLUMN IF EXISTS price_formula")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            "ALTER TABLE products ADD COLUMN IF NOT EXISTS base_cost numeric(19,6) NOT NULL DEFAULT 0",
        )
        .await?;
        conn.execute_unprepared(
            "ALTER TABLE products ADD COLUMN IF NOT EXISTS sales_price numeric(19,6) NOT NULL DEFAULT 0",
        )
        .await?;
        conn.execute_unprepared(
            "ALTER TABLE products ADD COLUMN IF NOT EXISTS price_formula text NOT NULL DEFAULT ''",
        )
        .await?;
        conn.execute_unprepared(
            r#"UPDATE products SET
                base_cost = CASE
                    WHEN btrim(base_price_formula) ~ '^-?[0-9]+(\.[0-9]+)?$'
                    THEN btrim(base_price_formula)::numeric
                    ELSE 0
                END,
                sales_price = CASE
                    WHEN btrim(sales_price_formula) ~ '^-?[0-9]+(\.[0-9]+)?$'
                    THEN btrim(sales_price_formula)::numeric
                    ELSE 0
                END,
                price_formula = CASE
                    WHEN btrim(sales_price_formula) ~ '^-?[0-9]+(\.[0-9]+)?$' THEN ''
                    ELSE sales_price_formula
                END"#,
        )
        .await?;
        conn.execute_unprepared("ALTER TABLE products DROP COLUMN IF EXISTS sales_price_formula")
            .await?;
        conn.execute_unprepared("ALTER TABLE products DROP COLUMN IF EXISTS base_price_formula")
            .await?;
        Ok(())
    }
}
