//! Point invoice service lines at unified service products and drop obsolete snapshot columns.

use lariv_plugin_finance_products::migrations::PRODUCT_SERVICE_ID_MAP;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const GOODS_TABLES: &[&str] = &[
    "draft_invoice_goods_lines",
    "posted_invoice_goods_lines",
    "cancelled_invoice_goods_lines",
];

const SERVICE_TABLES: &[&str] = &[
    "draft_invoice_service_lines",
    "posted_invoice_service_lines",
    "cancelled_invoice_service_lines",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        for table in SERVICE_TABLES {
            // Drop every service_id FK before remapping: m00045 may have added
            // `fk_*` while Postgres still keeps the original `*_service_id_fkey`
            // pointing at `services`, which rejects product ids.
            drop_service_fks(conn, table).await?;
            conn.execute_unprepared(&format!(
                r#"UPDATE {table} AS line
                   SET service_id = map.product_id
                   FROM {PRODUCT_SERVICE_ID_MAP} AS map
                   WHERE line.service_id = map.service_id"#
            ))
            .await?;
            add_service_product_fk(conn, table).await?;
            conn.execute_unprepared(&format!(
                r#"ALTER TABLE {table}
                   DROP COLUMN IF EXISTS service_reference,
                   DROP COLUMN IF EXISTS base_price_formula"#
            ))
            .await?;
        }
        for table in GOODS_TABLES {
            conn.execute_unprepared(&format!(
                r#"ALTER TABLE {table}
                   DROP COLUMN IF EXISTS product_reference,
                   DROP COLUMN IF EXISTS base_price_formula"#
            ))
            .await?;
        }
        // Purchases m00007 still copies from `services` when that plugin stack is
        // present. Without purchase catalogs, drop the legacy sales services tables.
        conn.execute_unprepared(
            r#"
            DO $$
            BEGIN
                IF to_regclass('public.purchase_products') IS NULL
                   AND to_regclass('public.services') IS NOT NULL THEN
                    DROP TABLE IF EXISTS service_journal_entry_item_templates CASCADE;
                    DROP TABLE IF EXISTS service_preference_journal_items CASCADE;
                    DROP TABLE IF EXISTS service_preferences_taxes CASCADE;
                    DROP TABLE IF EXISTS service_preferences CASCADE;
                    DROP TABLE IF EXISTS service_taxes CASCADE;
                    DROP TABLE IF EXISTS services CASCADE;
                END IF;
            END $$;
            "#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        for table in GOODS_TABLES {
            conn.execute_unprepared(&format!(
                r#"ALTER TABLE {table}
                   ADD COLUMN IF NOT EXISTS product_reference text,
                   ADD COLUMN IF NOT EXISTS base_price_formula text NOT NULL DEFAULT '0'"#
            ))
            .await?;
        }
        for table in SERVICE_TABLES {
            conn.execute_unprepared(&format!(
                r#"ALTER TABLE {table}
                   ADD COLUMN IF NOT EXISTS service_reference text,
                   ADD COLUMN IF NOT EXISTS base_price_formula text NOT NULL DEFAULT '0'"#
            ))
            .await?;
            rewrite_service_fk_down(conn, table).await?;
        }
        Ok(())
    }
}

async fn drop_service_fks(conn: &impl sea_orm::ConnectionTrait, table: &str) -> Result<(), DbErr> {
    for fk in [
        format!("fk_{table}_service_id"),
        format!("{table}_service_id_fkey"),
    ] {
        conn.execute_unprepared(&format!(
            "ALTER TABLE {table} DROP CONSTRAINT IF EXISTS {fk}"
        ))
        .await?;
    }
    Ok(())
}

async fn add_service_product_fk(
    conn: &impl sea_orm::ConnectionTrait,
    table: &str,
) -> Result<(), DbErr> {
    let fk = format!("fk_{table}_service_id");
    conn.execute_unprepared(&format!(
        r#"ALTER TABLE {table}
           ADD CONSTRAINT {fk}
           FOREIGN KEY (service_id) REFERENCES products(id)
           ON UPDATE CASCADE ON DELETE SET NULL"#
    ))
    .await?;
    Ok(())
}

async fn rewrite_service_fk_down(
    conn: &impl sea_orm::ConnectionTrait,
    table: &str,
) -> Result<(), DbErr> {
    drop_service_fks(conn, table).await?;
    let fk = format!("fk_{table}_service_id");
    conn.execute_unprepared(&format!(
        r#"ALTER TABLE {table}
           ADD CONSTRAINT {fk}
           FOREIGN KEY (service_id) REFERENCES services(id)
           ON UPDATE CASCADE ON DELETE SET NULL"#
    ))
    .await?;
    Ok(())
}
