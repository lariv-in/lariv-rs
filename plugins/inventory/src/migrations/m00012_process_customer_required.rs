//! Require a customer on every process movement.
//!
//! Existing rows take the customer of the latest in movement that supplied an
//! input stock, then of a process that output that stock.

use lariv_core::db::migration_sql::{exec_sql, is_postgres};
use sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const FROM_IN: &str = "\
UPDATE inventory_stock_movements_process
SET customer_id = (
    SELECT m.customer_id
    FROM inventory_stock_movement_process_lines AS pl
    JOIN inventory_stock_movement_in_lines AS il ON il.stock_id = pl.stock_id
    JOIN inventory_stock_movements_in AS m ON m.id = il.movement_id
    WHERE pl.movement_id = inventory_stock_movements_process.id
      AND pl.side = 'input'
    ORDER BY m.datetime DESC, m.id DESC
    LIMIT 1
)
WHERE customer_id IS NULL
  AND EXISTS (
    SELECT 1
    FROM inventory_stock_movement_process_lines AS pl
    JOIN inventory_stock_movement_in_lines AS il ON il.stock_id = pl.stock_id
    WHERE pl.movement_id = inventory_stock_movements_process.id
      AND pl.side = 'input'
  )";

const FROM_PROCESS: &str = "\
UPDATE inventory_stock_movements_process
SET customer_id = (
    SELECT src.customer_id
    FROM inventory_stock_movement_process_lines AS pl
    JOIN inventory_stock_movement_process_lines AS outl
      ON outl.stock_id = pl.stock_id
     AND outl.side = 'output'
     AND outl.movement_id <> pl.movement_id
    JOIN inventory_stock_movements_process AS src ON src.id = outl.movement_id
    WHERE pl.movement_id = inventory_stock_movements_process.id
      AND pl.side = 'input'
      AND src.customer_id IS NOT NULL
    ORDER BY src.datetime DESC, src.id DESC
    LIMIT 1
)
WHERE customer_id IS NULL
  AND EXISTS (
    SELECT 1
    FROM inventory_stock_movement_process_lines AS pl
    JOIN inventory_stock_movement_process_lines AS outl
      ON outl.stock_id = pl.stock_id
     AND outl.side = 'output'
     AND outl.movement_id <> pl.movement_id
    JOIN inventory_stock_movements_process AS src ON src.id = outl.movement_id
    WHERE pl.movement_id = inventory_stock_movements_process.id
      AND pl.side = 'input'
      AND src.customer_id IS NOT NULL
  )";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, FROM_IN).await?;
        // A process can feed another process. Repeat so a filled row can fill the next.
        for _ in 0..8 {
            let before = null_customer_count(manager).await?;
            if before == 0 {
                break;
            }
            exec_sql(manager, FROM_PROCESS).await?;
            if null_customer_count(manager).await? == before {
                break;
            }
        }
        let missing = null_customer_count(manager).await?;
        if missing > 0 {
            return Err(DbErr::Custom(format!(
                "{missing} process movement(s) have no customer from their input stock movements"
            )));
        }
        if is_postgres(manager) {
            exec_sql(
                manager,
                "ALTER TABLE inventory_stock_movements_process ALTER COLUMN customer_id SET NOT NULL",
            )
            .await?;
        } else {
            rebuild_sqlite_customer_not_null(manager).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            exec_sql(
                manager,
                "ALTER TABLE inventory_stock_movements_process ALTER COLUMN customer_id DROP NOT NULL",
            )
            .await?;
        }
        Ok(())
    }
}

async fn null_customer_count(manager: &SchemaManager<'_>) -> Result<i64, DbErr> {
    let backend = manager.get_database_backend();
    let row = manager
        .get_connection()
        .query_one_raw(Statement::from_string(
            backend,
            "SELECT COUNT(*) AS n FROM inventory_stock_movements_process WHERE customer_id IS NULL",
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("process customer count returned no row".into()))?;
    row.try_get("", "n")
}

/// SQLite cannot `ALTER COLUMN … SET NOT NULL`.
async fn rebuild_sqlite_customer_not_null(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    exec_sql(manager, "PRAGMA foreign_keys = OFF").await?;
    exec_sql(
        manager,
        r#"CREATE TABLE "inventory_stock_movements_process__customer" (
            "id" integer NOT NULL PRIMARY KEY AUTOINCREMENT,
            "created_at" timestamp_with_timezone_text,
            "updated_at" timestamp_with_timezone_text,
            "datetime" timestamp_with_timezone_text NOT NULL,
            "number" text NOT NULL,
            "created_by_id" integer,
            "customer_id" integer NOT NULL
        )"#,
    )
    .await?;
    exec_sql(
        manager,
        r#"INSERT INTO "inventory_stock_movements_process__customer"
            ("id", "created_at", "updated_at", "datetime", "number", "created_by_id", "customer_id")
           SELECT "id", "created_at", "updated_at", "datetime", "number", "created_by_id", "customer_id"
           FROM "inventory_stock_movements_process""#,
    )
    .await?;
    exec_sql(manager, r#"DROP TABLE "inventory_stock_movements_process""#).await?;
    exec_sql(
        manager,
        r#"ALTER TABLE "inventory_stock_movements_process__customer" RENAME TO "inventory_stock_movements_process""#,
    )
    .await?;
    exec_sql(
        manager,
        "UPDATE sqlite_sequence SET name = 'inventory_stock_movements_process' WHERE name = 'inventory_stock_movements_process__customer'",
    )
    .await?;
    exec_sql(
        manager,
        "CREATE INDEX IF NOT EXISTS idx_inventory_stock_movements_process_created_by_id ON inventory_stock_movements_process (created_by_id)",
    )
    .await?;
    exec_sql(
        manager,
        "CREATE INDEX IF NOT EXISTS idx_inventory_stock_movements_process_customer_id ON inventory_stock_movements_process (customer_id)",
    )
    .await?;
    exec_sql(manager, "PRAGMA foreign_keys = ON").await
}
