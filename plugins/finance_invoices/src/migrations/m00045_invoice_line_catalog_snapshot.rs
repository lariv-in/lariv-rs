//! Snapshot catalog fields onto invoice lines so deleting a product/service
//! nulls the FK but leaves display/pricing metadata on the line.
//!
//! Tolerates the unified product catalog (`reference` / `base_price_formula`
//! dropped, services optionally already folded into `products`).
use lariv_core::db::migration_sql::{exec_sql, exec_sql_conn};

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
        for table in GOODS_TABLES {
            exec_sql(
                manager,
                &format!(
                    r#"ALTER TABLE {table}
                   ADD COLUMN IF NOT EXISTS product_name text,
                   ADD COLUMN IF NOT EXISTS product_reference text,
                   ADD COLUMN IF NOT EXISTS product_remarks text,
                   ADD COLUMN IF NOT EXISTS hsn_code bigint,
                   ADD COLUMN IF NOT EXISTS variable_types text,
                   ADD COLUMN IF NOT EXISTS base_price_formula text,
                   ADD COLUMN IF NOT EXISTS sales_price_formula text"#
                ),
            )
            .await?;
            exec_sql(
                manager,
                &format!(
                    r#"
                DO $migration$
                DECLARE
                    has_reference boolean;
                    has_base boolean;
                    ref_expr text;
                    base_expr text;
                BEGIN
                    SELECT EXISTS (
                        SELECT 1
                        FROM information_schema.columns
                        WHERE table_schema = 'public'
                          AND table_name = 'products'
                          AND column_name = 'reference'
                    ) INTO has_reference;
                    SELECT EXISTS (
                        SELECT 1
                        FROM information_schema.columns
                        WHERE table_schema = 'public'
                          AND table_name = 'products'
                          AND column_name = 'base_price_formula'
                    ) INTO has_base;
                    ref_expr := CASE WHEN has_reference THEN 'p.reference' ELSE 'NULL' END;
                    base_expr := CASE
                        WHEN has_base THEN 'p.base_price_formula'
                        ELSE '''0'''
                    END;
                    EXECUTE format(
                        'UPDATE {table} AS line
                            SET product_name = p.name,
                                product_reference = %s,
                                product_remarks = p.remarks,
                                hsn_code = p.hsn_code,
                                variable_types = p.variables,
                                base_price_formula = %s,
                                sales_price_formula = p.sales_price_formula
                            FROM products p
                            WHERE line.product_id = p.id
                              AND line.product_name IS NULL',
                        ref_expr,
                        base_expr
                    );
                END
                $migration$;
                "#
                ),
            )
            .await?;
            // Orphans (product already gone): keep the row with placeholders.
            exec_sql(manager, &format!(
                r#"UPDATE {table}
                   SET product_name = COALESCE(product_name, format('Product #%s', product_id::text)),
                       hsn_code = COALESCE(hsn_code, 0),
                       variable_types = COALESCE(variable_types, '{{}}'),
                       base_price_formula = COALESCE(base_price_formula, '0'),
                       sales_price_formula = COALESCE(sales_price_formula, '0')
                   WHERE product_name IS NULL"#
            ))
            .await?;
            exec_sql(
                manager,
                &format!(
                    r#"ALTER TABLE {table}
                   ALTER COLUMN product_name SET NOT NULL,
                   ALTER COLUMN hsn_code SET NOT NULL,
                   ALTER COLUMN variable_types SET NOT NULL,
                   ALTER COLUMN base_price_formula SET NOT NULL,
                   ALTER COLUMN sales_price_formula SET NOT NULL,
                   ALTER COLUMN product_id DROP NOT NULL"#
                ),
            )
            .await?;
            rewrite_product_fk(conn, table).await?;
        }
        for table in SERVICE_TABLES {
            exec_sql(
                manager,
                &format!(
                    r#"ALTER TABLE {table}
                   ADD COLUMN IF NOT EXISTS service_name text,
                   ADD COLUMN IF NOT EXISTS service_reference text,
                   ADD COLUMN IF NOT EXISTS service_remarks text,
                   ADD COLUMN IF NOT EXISTS sac_code bigint,
                   ADD COLUMN IF NOT EXISTS variable_types text,
                   ADD COLUMN IF NOT EXISTS base_price_formula text,
                   ADD COLUMN IF NOT EXISTS sales_price_formula text"#
                ),
            )
            .await?;
            exec_sql(
                manager,
                &format!(
                    r#"
                DO $migration$
                DECLARE
                    has_services boolean;
                    has_reference boolean;
                    has_base boolean;
                    catalog text;
                    id_col text;
                    ref_expr text;
                    base_expr text;
                    sac_expr text;
                BEGIN
                    has_services := to_regclass('public.services') IS NOT NULL;
                    catalog := CASE WHEN has_services THEN 'services' ELSE 'products' END;
                    id_col := CASE WHEN has_services THEN 'id' ELSE 'id' END;

                    SELECT EXISTS (
                        SELECT 1
                        FROM information_schema.columns
                        WHERE table_schema = 'public'
                          AND table_name = catalog
                          AND column_name = 'reference'
                    ) INTO has_reference;
                    SELECT EXISTS (
                        SELECT 1
                        FROM information_schema.columns
                        WHERE table_schema = 'public'
                          AND table_name = catalog
                          AND column_name = 'base_price_formula'
                    ) INTO has_base;

                    ref_expr := CASE WHEN has_reference THEN 's.reference' ELSE 'NULL' END;
                    base_expr := CASE
                        WHEN has_base THEN 's.base_price_formula'
                        ELSE '''0'''
                    END;
                    sac_expr := CASE
                        WHEN has_services THEN 's.sac_code'
                        ELSE 's.sac_code'
                    END;

                    EXECUTE format(
                        'UPDATE {table} AS line
                            SET service_name = s.name,
                                service_reference = %s,
                                service_remarks = s.remarks,
                                sac_code = %s,
                                variable_types = s.variables,
                                base_price_formula = %s,
                                sales_price_formula = s.sales_price_formula
                            FROM %I s
                            WHERE line.service_id = s.%I
                              AND line.service_name IS NULL',
                        ref_expr,
                        sac_expr,
                        base_expr,
                        catalog,
                        id_col
                    );
                END
                $migration$;
                "#
                ),
            )
            .await?;
            exec_sql(manager, &format!(
                r#"UPDATE {table}
                   SET service_name = COALESCE(service_name, format('Service #%s', service_id::text)),
                       sac_code = COALESCE(sac_code, 0),
                       variable_types = COALESCE(variable_types, '{{}}'),
                       base_price_formula = COALESCE(base_price_formula, '0'),
                       sales_price_formula = COALESCE(sales_price_formula, '0')
                   WHERE service_name IS NULL"#
            ))
            .await?;
            exec_sql(
                manager,
                &format!(
                    r#"ALTER TABLE {table}
                   ALTER COLUMN service_name SET NOT NULL,
                   ALTER COLUMN sac_code SET NOT NULL,
                   ALTER COLUMN variable_types SET NOT NULL,
                   ALTER COLUMN base_price_formula SET NOT NULL,
                   ALTER COLUMN sales_price_formula SET NOT NULL,
                   ALTER COLUMN service_id DROP NOT NULL"#
                ),
            )
            .await?;
            rewrite_service_fk(conn, table).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in GOODS_TABLES {
            exec_sql(
                manager,
                &format!(
                    r#"ALTER TABLE {table}
                   DROP COLUMN IF EXISTS product_name,
                   DROP COLUMN IF EXISTS product_reference,
                   DROP COLUMN IF EXISTS product_remarks,
                   DROP COLUMN IF EXISTS hsn_code,
                   DROP COLUMN IF EXISTS variable_types,
                   DROP COLUMN IF EXISTS base_price_formula,
                   DROP COLUMN IF EXISTS sales_price_formula"#
                ),
            )
            .await?;
        }
        for table in SERVICE_TABLES {
            exec_sql(
                manager,
                &format!(
                    r#"ALTER TABLE {table}
                   DROP COLUMN IF EXISTS service_name,
                   DROP COLUMN IF EXISTS service_reference,
                   DROP COLUMN IF EXISTS service_remarks,
                   DROP COLUMN IF EXISTS sac_code,
                   DROP COLUMN IF EXISTS variable_types,
                   DROP COLUMN IF EXISTS base_price_formula,
                   DROP COLUMN IF EXISTS sales_price_formula"#
                ),
            )
            .await?;
        }
        Ok(())
    }
}

async fn rewrite_product_fk(
    conn: &impl sea_orm::ConnectionTrait,
    table: &str,
) -> Result<(), DbErr> {
    let fk = format!("fk_{table}_product_id");
    // Legacy names from before the goods/service rename.
    let legacy = match table {
        "draft_invoice_goods_lines" => Some("fk_draft_invoice_lines_product_id"),
        "posted_invoice_goods_lines" => Some("fk_posted_invoice_lines_product_id"),
        "cancelled_invoice_goods_lines" => Some("fk_cancelled_invoice_lines_product_id"),
        _ => None,
    };
    if let Some(old) = legacy {
        exec_sql_conn(
            conn,
            conn.get_database_backend(),
            &format!("ALTER TABLE {table} DROP CONSTRAINT IF EXISTS {old}"),
        )
        .await?;
    }
    exec_sql_conn(
        conn,
        conn.get_database_backend(),
        &format!("ALTER TABLE {table} DROP CONSTRAINT IF EXISTS {fk}"),
    )
    .await?;
    exec_sql_conn(
        conn,
        conn.get_database_backend(),
        &format!(
            r#"ALTER TABLE {table}
           ADD CONSTRAINT {fk}
           FOREIGN KEY (product_id) REFERENCES products(id)
           ON UPDATE CASCADE ON DELETE SET NULL"#
        ),
    )
    .await?;
    Ok(())
}

async fn rewrite_service_fk(
    conn: &impl sea_orm::ConnectionTrait,
    table: &str,
) -> Result<(), DbErr> {
    let fk = format!("fk_{table}_service_id");
    exec_sql_conn(
        conn,
        conn.get_database_backend(),
        &format!("ALTER TABLE {table} DROP CONSTRAINT IF EXISTS {fk}"),
    )
    .await?;
    // Prefer legacy `services` while it still exists; otherwise the unified catalog.
    exec_sql_conn(
        conn,
        conn.get_database_backend(),
        &format!(
            r#"
        DO $migration$
        BEGIN
            IF to_regclass('public.services') IS NOT NULL THEN
                EXECUTE '
                    ALTER TABLE {table}
                    ADD CONSTRAINT {fk}
                    FOREIGN KEY (service_id) REFERENCES services(id)
                    ON UPDATE CASCADE ON DELETE SET NULL
                ';
            ELSE
                EXECUTE '
                    ALTER TABLE {table}
                    ADD CONSTRAINT {fk}
                    FOREIGN KEY (service_id) REFERENCES products(id)
                    ON UPDATE CASCADE ON DELETE SET NULL
                ';
            END IF;
        END
        $migration$;
        "#
        ),
    )
    .await?;
    Ok(())
}
