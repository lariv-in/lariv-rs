//! Drop the company stored on each stock.
use lariv_core::db::migration_sql::{exec_sql, is_postgres};

use sea_orm::{DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum InventoryStocks {
    #[iden = "inventory_stocks"]
    Table,
    CompanyId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !is_postgres(manager) {
            // `company_id` is part of a foreign key, so SQLite will not drop the column.
            return drop_company_sqlite(manager).await;
        }
        if is_postgres(manager) {
            manager
                .drop_foreign_key(
                    ForeignKey::drop()
                        .name("fk_inventory_stocks_company_id")
                        .table(InventoryStocks::Table)
                        .to_owned(),
                )
                .await?;
        };
        manager
            .drop_index(
                Index::drop()
                    .name("idx_inventory_stocks_company_id")
                    .table(InventoryStocks::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(InventoryStocks::Table)
                    .drop_column(InventoryStocks::CompanyId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "dropping the stock company is not reversible".into(),
        ))
    }
}

async fn drop_company_sqlite(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    // A previous attempt may have dropped `inventory_stocks` and left the copy
    // behind when the rename hit SQLite's statement cache.
    let copy_exists = table_exists(manager, "inventory_stocks__noco").await?;
    let stocks_exist = table_exists(manager, "inventory_stocks").await?;
    if copy_exists && !stocks_exist {
        exec_sql(
            manager,
            r#"ALTER TABLE "inventory_stocks__noco" RENAME TO "inventory_stocks""#,
        )
        .await?;
        return Ok(());
    }
    if copy_exists {
        exec_sql(manager, r#"DROP TABLE "inventory_stocks__noco""#).await?;
    }
    exec_sql(manager, "PRAGMA foreign_keys = OFF").await?;
    exec_sql(
        manager,
        r#"
CREATE TABLE "inventory_stocks__noco" (
    "id" integer NOT NULL PRIMARY KEY AUTOINCREMENT,
    "created_at" timestamp_with_timezone_text,
    "updated_at" timestamp_with_timezone_text,
    "name" text NOT NULL,
    "qty_type" varchar(32) NOT NULL,
    "qty_unit" text NOT NULL
)"#,
    )
    .await?;
    exec_sql(
        manager,
        r#"
INSERT INTO "inventory_stocks__noco" (id, created_at, updated_at, name, qty_type, qty_unit)
SELECT id, created_at, updated_at, name, qty_type, qty_unit
FROM inventory_stocks"#,
    )
    .await?;
    exec_sql(manager, r#"DROP TABLE "inventory_stocks""#).await?;
    exec_sql(
        manager,
        r#"ALTER TABLE "inventory_stocks__noco" RENAME TO "inventory_stocks""#,
    )
    .await?;
    exec_sql(manager, "PRAGMA foreign_keys = ON").await
}

async fn table_exists(manager: &SchemaManager<'_>, table: &str) -> Result<bool, DbErr> {
    let row = manager
        .get_connection()
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            format!("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '{table}'"),
        ))
        .await?;
    Ok(row.is_some())
}
