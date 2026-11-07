//! Unify the product catalog: kind, SAC/HSN, document-side journals, service import map.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

/// Maps legacy `services.id` to unified `products.id` after catalog merge.
pub const PRODUCT_SERVICE_ID_MAP: &str = "product_service_id_map";

const UP: &str = r#"
DO $$
DECLARE
    v_service record;
    v_product_id bigint;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'product_kind') THEN
        CREATE TYPE product_kind AS ENUM ('Goods', 'Services');
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'products'
          AND column_name = 'product_kind'
    ) THEN
        ALTER TABLE products
            ADD COLUMN product_kind product_kind NOT NULL DEFAULT 'Goods';
    END IF;

    ALTER TABLE products ADD COLUMN IF NOT EXISTS sac_code bigint NULL;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'products'
          AND column_name = 'hsn_code'
          AND is_nullable = 'NO'
    ) THEN
        ALTER TABLE products ALTER COLUMN hsn_code DROP NOT NULL;
    END IF;

    ALTER TABLE products DROP CONSTRAINT IF EXISTS fk_products_stock_id;
    DROP INDEX IF EXISTS idx_products_stock_id;
    ALTER TABLE products DROP COLUMN IF EXISTS stock_id;

    DROP INDEX IF EXISTS uq_products_reference;
    DROP INDEX IF EXISTS products_reference_trgm_idx;
    ALTER TABLE products DROP COLUMN IF EXISTS reference;

    ALTER TABLE products DROP COLUMN IF EXISTS base_price_formula;

    IF to_regclass('public.services') IS NOT NULL THEN
        CREATE TABLE IF NOT EXISTS product_service_id_map (
            service_id bigint PRIMARY KEY,
            product_id bigint NOT NULL UNIQUE
        );

        IF NOT EXISTS (
            SELECT 1
            FROM pg_constraint
            WHERE conname = 'fk_product_service_id_map_product_id'
        ) THEN
            ALTER TABLE product_service_id_map
                ADD CONSTRAINT fk_product_service_id_map_product_id
                    FOREIGN KEY (product_id) REFERENCES products(id)
                    ON UPDATE CASCADE ON DELETE CASCADE;
        END IF;

        FOR v_service IN
            SELECT id, created_at, updated_at, name, remarks, sac_code, variables, sales_price_formula
            FROM services
            WHERE NOT EXISTS (
                SELECT 1 FROM product_service_id_map AS map WHERE map.service_id = services.id
            )
            ORDER BY id
        LOOP
            INSERT INTO products (
                created_at, updated_at, product_kind, name, remarks, sac_code,
                variables, sales_price_formula, hsn_code
            ) VALUES (
                v_service.created_at,
                v_service.updated_at,
                'Services'::product_kind,
                v_service.name,
                v_service.remarks,
                v_service.sac_code,
                v_service.variables,
                v_service.sales_price_formula,
                NULL
            )
            RETURNING id INTO v_product_id;

            INSERT INTO product_service_id_map (service_id, product_id)
            VALUES (v_service.id, v_product_id);
        END LOOP;

        INSERT INTO product_taxes (product_id, tax_id)
        SELECT map.product_id, st.tax_id
        FROM service_taxes AS st
        JOIN product_service_id_map AS map ON map.service_id = st.service_id
        ON CONFLICT DO NOTHING;

        INSERT INTO product_journal_entry_item_templates (
            created_at, updated_at, product_id, journal_id, account_id, formula
        )
        SELECT
            item.created_at,
            item.updated_at,
            map.product_id,
            item.journal_id,
            item.account_id,
            item.formula
        FROM service_journal_entry_item_templates AS item
        JOIN product_service_id_map AS map ON map.service_id = item.service_id
        WHERE NOT EXISTS (
            SELECT 1
            FROM product_journal_entry_item_templates AS existing
            WHERE existing.product_id = map.product_id
              AND existing.journal_id = item.journal_id
              AND existing.account_id = item.account_id
              AND existing.formula = item.formula
        );

        IF to_regclass('public.service_preference_journal_items') IS NOT NULL
           AND to_regclass('public.product_preference_journal_items') IS NOT NULL
           AND NOT EXISTS (
               SELECT 1
               FROM product_preference_journal_items
               WHERE product_preferences_id = 1
           )
        THEN
            INSERT INTO product_preference_journal_items (
                created_at, updated_at, product_preferences_id,
                journal_id, account_id, formula
            )
            SELECT
                item.created_at,
                item.updated_at,
                1,
                item.journal_id,
                item.account_id,
                item.formula
            FROM service_preference_journal_items AS item
            WHERE item.service_preferences_id = 1;
        END IF;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'products'
          AND column_name = 'product_type'
    ) THEN
        UPDATE products
           SET product_kind = 'Services'::product_kind
         WHERE product_type::text IN ('Services', 'Both')
           AND product_kind = 'Goods'::product_kind;
        ALTER TABLE products DROP COLUMN product_type;
        DROP TYPE IF EXISTS product_type;
    END IF;
END $$;
"#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, UP).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
