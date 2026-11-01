//! Shared SQL: copy/rename `crm_customers` into `customers` and retarget FKs.

/// Safe to run when already consolidated (no-op if `crm_customers` is gone).
pub const UP: &[&str] = &[
    r#"DO $fix$
DECLARE
    fk record;
    fk_col text;
    fk_def text;
BEGIN
    IF to_regclass('public.crm_customers') IS NULL THEN
        RETURN;
    END IF;

    IF to_regclass('public.customers') IS NULL THEN
        ALTER TABLE crm_customers RENAME TO customers;
        RETURN;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM crm_customers LIMIT 1) THEN
        DROP TABLE crm_customers;
        RETURN;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM customers LIMIT 1) THEN
        INSERT INTO customers (
            id, created_at, updated_at, is_individual, company_id, individual_id
        )
        SELECT id, created_at, updated_at, is_individual, company_id, individual_id
          FROM crm_customers;
    ELSE
        INSERT INTO customers (
            created_at, updated_at, is_individual, company_id, individual_id
        )
        SELECT c.created_at, c.updated_at, c.is_individual, c.company_id, c.individual_id
          FROM crm_customers AS c
         WHERE NOT EXISTS (
                   SELECT 1
                     FROM customers AS x
                    WHERE x.is_individual = c.is_individual
                      AND (
                          (NOT c.is_individual
                           AND x.company_id IS NOT DISTINCT FROM c.company_id)
                          OR (c.is_individual
                              AND x.individual_id IS NOT DISTINCT FROM c.individual_id)
                      )
               );

        FOR fk IN
            SELECT c.oid,
                   c.conname,
                   c.conrelid::regclass AS tbl,
                   c.conkey
              FROM pg_constraint AS c
             WHERE c.contype = 'f'
               AND c.confrelid = 'public.crm_customers'::regclass
        LOOP
            SELECT a.attname
              INTO fk_col
              FROM pg_attribute AS a
             WHERE a.attrelid = fk.tbl
               AND a.attnum = fk.conkey[1]
               AND NOT a.attisdropped;

            EXECUTE format(
                'UPDATE %s AS t
                    SET %I = cust.id
                   FROM crm_customers AS c
                   JOIN customers AS cust
                     ON cust.is_individual = c.is_individual
                    AND (
                        (NOT c.is_individual
                         AND cust.company_id IS NOT DISTINCT FROM c.company_id)
                        OR (c.is_individual
                            AND cust.individual_id IS NOT DISTINCT FROM c.individual_id)
                    )
                  WHERE t.%I = c.id
                    AND t.%I IS DISTINCT FROM cust.id',
                fk.tbl, fk_col, fk_col, fk_col
            );
        END LOOP;
    END IF;

    FOR fk IN
        SELECT c.conname,
               c.conrelid::regclass AS tbl,
               pg_get_constraintdef(c.oid) AS def
          FROM pg_constraint AS c
         WHERE c.contype = 'f'
           AND c.confrelid = 'public.crm_customers'::regclass
    LOOP
        fk_def := replace(fk.def, 'REFERENCES crm_customers(', 'REFERENCES customers(');
        EXECUTE format('ALTER TABLE %s DROP CONSTRAINT %I', fk.tbl, fk.conname);
        EXECUTE format('ALTER TABLE %s ADD CONSTRAINT %I %s', fk.tbl, fk.conname, fk_def);
    END LOOP;

    DROP TABLE crm_customers;

    PERFORM setval(
        pg_get_serial_sequence('customers', 'id'),
        GREATEST(COALESCE((SELECT MAX(id) FROM customers), 1), 1),
        EXISTS (SELECT 1 FROM customers)
    );
END
$fix$"#,
    "CREATE UNIQUE INDEX IF NOT EXISTS ux_crm_customers_company
        ON customers (company_id) WHERE NOT is_individual",
    "CREATE UNIQUE INDEX IF NOT EXISTS ux_crm_customers_individual
        ON customers (individual_id) WHERE is_individual",
];
