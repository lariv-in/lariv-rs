//! Merge purchase-owned catalogs into separate `products` rows and retarget line FKs.
//!
//! Sales and purchase catalogs stay as distinct product rows (one journal template
//! each). When a purchase catalog id already exists on `products`, insert a new
//! product and remap purchase document lines to it.

use lariv_core::db::migration_sql::{
    self, FK_CANCELLED_PURCHASE_GOODS_LINE_PRODUCT, FK_CANCELLED_PURCHASE_SERVICE_LINE_SERVICE,
    FK_DRAFT_PURCHASE_GOODS_LINE_PRODUCT, FK_DRAFT_PURCHASE_SERVICE_LINE_SERVICE,
    FK_POSTED_PURCHASE_GOODS_LINE_PRODUCT, FK_POSTED_PURCHASE_SERVICE_LINE_SERVICE,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
DO $$
DECLARE
    v_pp record;
    v_ps record;
    v_product_id bigint;
    r record;
BEGIN
    IF to_regclass('public.purchase_products') IS NULL THEN
        RETURN;
    END IF;

    CREATE TEMP TABLE purchase_product_id_map (
        purchase_product_id bigint PRIMARY KEY,
        product_id bigint NOT NULL
    ) ON COMMIT DROP;

    CREATE TEMP TABLE purchase_service_product_map (
        purchase_service_id bigint PRIMARY KEY,
        product_id bigint NOT NULL
    ) ON COMMIT DROP;

    FOR v_pp IN SELECT * FROM purchase_products ORDER BY id
    LOOP
        IF EXISTS (SELECT 1 FROM products WHERE id = v_pp.id) THEN
            INSERT INTO products (
                created_at, updated_at, product_kind, name, remarks, hsn_code, sac_code,
                variables, sales_price_formula
            ) VALUES (
                v_pp.created_at,
                v_pp.updated_at,
                'Goods'::product_kind,
                v_pp.name || ' Purchase',
                v_pp.remarks,
                v_pp.hsn_code,
                NULL,
                v_pp.variables,
                v_pp.sales_price_formula
            )
            RETURNING id INTO v_product_id;
        ELSE
            INSERT INTO products (
                id, created_at, updated_at, product_kind, name, remarks, hsn_code, sac_code,
                variables, sales_price_formula
            ) VALUES (
                v_pp.id,
                v_pp.created_at,
                v_pp.updated_at,
                'Goods'::product_kind,
                v_pp.name || ' Purchase',
                v_pp.remarks,
                v_pp.hsn_code,
                NULL,
                v_pp.variables,
                v_pp.sales_price_formula
            );
            v_product_id := v_pp.id;
        END IF;

        INSERT INTO purchase_product_id_map (purchase_product_id, product_id)
        VALUES (v_pp.id, v_product_id);

        INSERT INTO product_taxes (product_id, tax_id)
        SELECT v_product_id, ppt.tax_id
          FROM purchase_product_taxes AS ppt
         WHERE ppt.product_id = v_pp.id
        ON CONFLICT DO NOTHING;

        INSERT INTO product_journal_entry_item_templates (
            created_at, updated_at, product_id, journal_id, account_id, formula
        )
        SELECT
            item.created_at,
            item.updated_at,
            v_product_id,
            item.journal_id,
            item.account_id,
            item.formula
          FROM purchase_product_journal_entry_item_templates AS item
         WHERE item.product_id = v_pp.id
           AND NOT EXISTS (
               SELECT 1
                 FROM product_journal_entry_item_templates AS existing
                WHERE existing.product_id = v_product_id
                  AND existing.journal_id = item.journal_id
                  AND existing.account_id = item.account_id
                  AND existing.formula = item.formula
           );
    END LOOP;

    FOR v_ps IN SELECT * FROM purchase_services ORDER BY id
    LOOP
        -- Always a distinct catalog row from sales services.
        INSERT INTO products (
            created_at, updated_at, product_kind, name, remarks, hsn_code, sac_code,
            variables, sales_price_formula
        ) VALUES (
            v_ps.created_at,
            v_ps.updated_at,
            'Services'::product_kind,
            v_ps.name || ' Purchase',
            v_ps.remarks,
            NULL,
            v_ps.sac_code,
            v_ps.variables,
            v_ps.sales_price_formula
        )
        RETURNING id INTO v_product_id;

        INSERT INTO purchase_service_product_map (purchase_service_id, product_id)
        VALUES (v_ps.id, v_product_id);

        INSERT INTO product_taxes (product_id, tax_id)
        SELECT v_product_id, pst.tax_id
          FROM purchase_service_taxes AS pst
         WHERE pst.service_id = v_ps.id
        ON CONFLICT DO NOTHING;

        INSERT INTO product_journal_entry_item_templates (
            created_at, updated_at, product_id, journal_id, account_id, formula
        )
        SELECT
            item.created_at,
            item.updated_at,
            v_product_id,
            item.journal_id,
            item.account_id,
            item.formula
          FROM purchase_service_journal_entry_item_templates AS item
         WHERE item.service_id = v_ps.id
           AND NOT EXISTS (
               SELECT 1
                 FROM product_journal_entry_item_templates AS existing
                WHERE existing.product_id = v_product_id
                  AND existing.journal_id = item.journal_id
                  AND existing.account_id = item.account_id
                  AND existing.formula = item.formula
           );
    END LOOP;

    IF pg_get_serial_sequence('products', 'id') IS NOT NULL THEN
        PERFORM setval(
            pg_get_serial_sequence('products', 'id'),
            GREATEST(COALESCE((SELECT MAX(id) FROM products), 1), 1),
            EXISTS (SELECT 1 FROM products)
        );
    END IF;

    -- Drop line FKs before remapping: new product ids are not in purchase_*.
    -- Between m00005 and m00013 the line tables are still `*_goods_lines` /
    -- `*_service_lines` (not the final unified `*_purchase_lines` names).
    FOR r IN
        SELECT c.conname AS constraint_name, src.relname AS table_name
          FROM pg_constraint c
          JOIN pg_class src ON src.oid = c.conrelid
          JOIN pg_class dst ON dst.oid = c.confrelid
          JOIN pg_namespace n ON n.oid = src.relnamespace
         WHERE c.contype = 'f'
           AND n.nspname = 'public'
           AND dst.relname IN ('purchase_products', 'purchase_services')
           AND src.relname IN (
               'draft_purchase_goods_lines',
               'posted_purchase_goods_lines',
               'cancelled_purchase_goods_lines',
               'draft_purchase_service_lines',
               'posted_purchase_service_lines',
               'cancelled_purchase_service_lines'
           )
    LOOP
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I', r.table_name, r.constraint_name);
    END LOOP;

    IF to_regclass('public.draft_purchase_goods_lines') IS NOT NULL THEN
        UPDATE draft_purchase_goods_lines AS line
           SET product_id = map.product_id
          FROM purchase_product_id_map AS map
         WHERE line.product_id = map.purchase_product_id;
    END IF;
    IF to_regclass('public.posted_purchase_goods_lines') IS NOT NULL THEN
        UPDATE posted_purchase_goods_lines AS line
           SET product_id = map.product_id
          FROM purchase_product_id_map AS map
         WHERE line.product_id = map.purchase_product_id;
    END IF;
    IF to_regclass('public.cancelled_purchase_goods_lines') IS NOT NULL THEN
        UPDATE cancelled_purchase_goods_lines AS line
           SET product_id = map.product_id
          FROM purchase_product_id_map AS map
         WHERE line.product_id = map.purchase_product_id;
    END IF;

    IF to_regclass('public.draft_purchase_service_lines') IS NOT NULL THEN
        UPDATE draft_purchase_service_lines AS line
           SET service_id = map.product_id
          FROM purchase_service_product_map AS map
         WHERE line.service_id = map.purchase_service_id;
    END IF;
    IF to_regclass('public.posted_purchase_service_lines') IS NOT NULL THEN
        UPDATE posted_purchase_service_lines AS line
           SET service_id = map.product_id
          FROM purchase_service_product_map AS map
         WHERE line.service_id = map.purchase_service_id;
    END IF;
    IF to_regclass('public.cancelled_purchase_service_lines') IS NOT NULL THEN
        UPDATE cancelled_purchase_service_lines AS line
           SET service_id = map.product_id
          FROM purchase_service_product_map AS map
         WHERE line.service_id = map.purchase_service_id;
    END IF;
END $$;

DROP TABLE IF EXISTS purchase_product_preference_journal_items;
DROP TABLE IF EXISTS purchase_service_preference_journal_items;
DROP TABLE IF EXISTS purchase_product_journal_entry_item_templates;
DROP TABLE IF EXISTS purchase_service_journal_entry_item_templates;
DROP TABLE IF EXISTS purchase_product_preferences_taxes;
DROP TABLE IF EXISTS purchase_service_preferences_taxes;
DROP TABLE IF EXISTS purchase_product_preferences;
DROP TABLE IF EXISTS purchase_service_preferences;
DROP TABLE IF EXISTS purchase_product_taxes;
DROP TABLE IF EXISTS purchase_service_taxes;
DROP TABLE IF EXISTS purchase_products;
DROP TABLE IF EXISTS purchase_services;

-- Sales services catalog is fully folded into products by now.
DROP TABLE IF EXISTS service_journal_entry_item_templates CASCADE;
DROP TABLE IF EXISTS service_preference_journal_items CASCADE;
DROP TABLE IF EXISTS service_preferences_taxes CASCADE;
DROP TABLE IF EXISTS service_preferences CASCADE;
DROP TABLE IF EXISTS service_taxes CASCADE;
DROP TABLE IF EXISTS services CASCADE;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP).await?;
        migration_sql::ensure_schema_foreign_keys(
            manager,
            &[
                FK_DRAFT_PURCHASE_GOODS_LINE_PRODUCT,
                FK_POSTED_PURCHASE_GOODS_LINE_PRODUCT,
                FK_CANCELLED_PURCHASE_GOODS_LINE_PRODUCT,
                FK_DRAFT_PURCHASE_SERVICE_LINE_SERVICE,
                FK_POSTED_PURCHASE_SERVICE_LINE_SERVICE,
                FK_CANCELLED_PURCHASE_SERVICE_LINE_SERVICE,
            ],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
