//! Finish the product catalog after invoice and purchase lines have moved.
//!
//! Invoice and purchase migrations each run this statement. It no-ops until
//! every installed document plugin has renamed its line tables. It then moves
//! any goods lines that still point at a product copied into `services`.
//! That move also lives on the invoice and purchase migrations, which skip it
//! when `services` is not installed yet. A later run has to finish the move
//! before those products can be deleted. It then drops `product_type` and
//! requires `products.stock_id`.

pub const CLEANUP_PRODUCT_CATALOG: &str = r#"
DO $$
DECLARE
    v_table text;
    v_has_lines boolean;
    v_spec record;
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
        FOR v_spec IN
            SELECT *
            FROM (VALUES
                ('draft_invoice_goods_lines', 'draft_invoice_service_lines', 'draft_invoice_id',
                 'draft_invoice_goods_line_taxes', 'draft_invoice_goods_line_id',
                 'draft_invoice_service_line_taxes', 'draft_invoice_service_line_id', '0'),
                ('posted_invoice_goods_lines', 'posted_invoice_service_lines', 'posted_invoice_id',
                 'posted_invoice_goods_line_taxes', 'posted_invoice_goods_line_id',
                 'posted_invoice_service_line_taxes', 'posted_invoice_service_line_id', '1'),
                ('cancelled_invoice_goods_lines', 'cancelled_invoice_service_lines', 'cancelled_invoice_id',
                 'cancelled_invoice_goods_line_taxes', 'cancelled_invoice_goods_line_id',
                 'cancelled_invoice_service_line_taxes', 'cancelled_invoice_service_line_id', '1'),
                ('draft_purchase_goods_lines', 'draft_purchase_service_lines', 'draft_purchase_id',
                 'draft_purchase_goods_line_taxes', 'draft_purchase_goods_line_id',
                 'draft_purchase_service_line_taxes', 'draft_purchase_service_line_id', '0'),
                ('posted_purchase_goods_lines', 'posted_purchase_service_lines', 'posted_purchase_id',
                 'posted_purchase_goods_line_taxes', 'posted_purchase_goods_line_id',
                 'posted_purchase_service_line_taxes', 'posted_purchase_service_line_id', '1'),
                ('cancelled_purchase_goods_lines', 'cancelled_purchase_service_lines', 'cancelled_purchase_id',
                 'cancelled_purchase_goods_line_taxes', 'cancelled_purchase_goods_line_id',
                 'cancelled_purchase_service_line_taxes', 'cancelled_purchase_service_line_id', '1')
            ) AS spec(
                goods, service, parent_col, goods_tax, goods_tax_fk, service_tax, service_tax_fk, with_journal
            )
        LOOP
            IF to_regclass('public.' || v_spec.goods) IS NULL
               OR to_regclass('public.' || v_spec.service) IS NULL THEN
                CONTINUE;
            END IF;

            EXECUTE format(
                'ALTER TABLE %I ADD COLUMN IF NOT EXISTS migrated_from_line_id bigint',
                v_spec.service
            );

            IF v_spec.with_journal = '1' THEN
                EXECUTE format(
                    'INSERT INTO %I (
                        created_at, updated_at, %I, service_id, rate, quantity,
                        variable_values, pre_tax_amount, remarks, journal_entry_item_id,
                        migrated_from_line_id
                     )
                     SELECT line.created_at, line.updated_at, line.%I, service.id,
                            line.rate, line.quantity, line.variable_values, line.pre_tax_amount,
                            line.remarks, line.journal_entry_item_id, line.id
                     FROM %I AS line
                     JOIN services AS service ON service.legacy_product_id = line.product_id
                     WHERE NOT EXISTS (
                         SELECT 1 FROM %I existing
                         WHERE existing.migrated_from_line_id = line.id
                     )',
                    v_spec.service, v_spec.parent_col, v_spec.parent_col, v_spec.goods, v_spec.service
                );
            ELSE
                EXECUTE format(
                    'INSERT INTO %I (
                        created_at, updated_at, %I, service_id, rate, quantity,
                        variable_values, pre_tax_amount, remarks, migrated_from_line_id
                     )
                     SELECT line.created_at, line.updated_at, line.%I, service.id,
                            line.rate, line.quantity, line.variable_values, line.pre_tax_amount,
                            line.remarks, line.id
                     FROM %I AS line
                     JOIN services AS service ON service.legacy_product_id = line.product_id
                     WHERE NOT EXISTS (
                         SELECT 1 FROM %I existing
                         WHERE existing.migrated_from_line_id = line.id
                     )',
                    v_spec.service, v_spec.parent_col, v_spec.parent_col, v_spec.goods, v_spec.service
                );
            END IF;

            IF to_regclass('public.' || v_spec.goods_tax) IS NOT NULL
               AND to_regclass('public.' || v_spec.service_tax) IS NOT NULL THEN
                EXECUTE format(
                    'INSERT INTO %I (%I, tax_id)
                     SELECT moved.id, tax.tax_id
                     FROM %I AS moved
                     JOIN %I AS tax ON tax.%I = moved.migrated_from_line_id
                     ON CONFLICT DO NOTHING',
                    v_spec.service_tax, v_spec.service_tax_fk, v_spec.service,
                    v_spec.goods_tax, v_spec.goods_tax_fk
                );
                EXECUTE format(
                    'DELETE FROM %I
                     WHERE %I IN (
                         SELECT migrated_from_line_id FROM %I
                         WHERE migrated_from_line_id IS NOT NULL
                     )',
                    v_spec.goods_tax, v_spec.goods_tax_fk, v_spec.service
                );
            END IF;

            EXECUTE format(
                'DELETE FROM %I
                 WHERE id IN (
                     SELECT migrated_from_line_id FROM %I
                     WHERE migrated_from_line_id IS NOT NULL
                 )',
                v_spec.goods, v_spec.service
            );
            EXECUTE format(
                'ALTER TABLE %I DROP COLUMN IF EXISTS migrated_from_line_id',
                v_spec.service
            );
        END LOOP;

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

    IF to_regclass('public.inventory_stocks') IS NOT NULL
       AND EXISTS (
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
