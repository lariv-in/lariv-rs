//! Finish the product catalog after invoice and purchase lines have moved.
//!
//! Invoice and purchase migrations each run this statement. It no-ops until
//! every installed document plugin has renamed its line tables, then deletes
//! products that were copied into `services`, drops `product_type`, and
//! requires `products.stock_id`.

pub const CLEANUP_PRODUCT_CATALOG: &str = r#"
DO $$
DECLARE
    v_table text;
    v_has_lines boolean;
BEGIN
    IF to_regclass('public.services') IS NULL THEN
        RETURN;
    END IF;

    IF to_regclass('public.draft_invoices') IS NOT NULL
       AND to_regclass('public.draft_invoice_goods_lines') IS NULL THEN
        RETURN;
    END IF;
    IF to_regclass('public.draft_purchases') IS NOT NULL
       AND to_regclass('public.draft_purchase_goods_lines') IS NULL THEN
        RETURN;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'services'
          AND column_name = 'legacy_product_id'
    ) THEN
        FOREACH v_table IN ARRAY ARRAY[
            'draft_invoice_goods_lines',
            'posted_invoice_goods_lines',
            'cancelled_invoice_goods_lines',
            'draft_purchase_goods_lines',
            'posted_purchase_goods_lines',
            'cancelled_purchase_goods_lines'
        ]
        LOOP
            IF to_regclass('public.' || v_table) IS NOT NULL THEN
                EXECUTE format(
                    'SELECT EXISTS (
                        SELECT 1
                        FROM %I AS line
                        JOIN services AS service ON service.legacy_product_id = line.product_id
                    )',
                    v_table
                ) INTO v_has_lines;

                IF v_has_lines THEN
                    RAISE EXCEPTION '% lines still reference a product copied to services', v_table;
                END IF;
            END IF;
        END LOOP;

        DELETE FROM product_taxes
        WHERE product_id IN (
            SELECT legacy_product_id FROM services WHERE legacy_product_id IS NOT NULL
        );
        DELETE FROM products
        WHERE id IN (
            SELECT legacy_product_id FROM services WHERE legacy_product_id IS NOT NULL
        );
        ALTER TABLE services DROP COLUMN legacy_product_id;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'products'
          AND column_name = 'stock_id'
          AND is_nullable = 'YES'
    ) THEN
        IF EXISTS (SELECT 1 FROM products WHERE stock_id IS NULL) THEN
            RAISE EXCEPTION 'products exist without a stock; assign inventory_stocks.id on products.stock_id before migrating';
        END IF;
        ALTER TABLE products ALTER COLUMN stock_id SET NOT NULL;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'products'
          AND column_name = 'product_type'
    ) THEN
        ALTER TABLE products DROP COLUMN product_type;
    END IF;

    IF EXISTS (SELECT 1 FROM pg_type WHERE typname = 'product_type') THEN
        DROP TYPE product_type;
    END IF;
END $$;
"#;
