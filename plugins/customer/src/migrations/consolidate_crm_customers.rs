//! Shared SQL: copy/rename `crm_customers` into `customers` and retarget FKs.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

/// Safe to run when already consolidated (no-op if `crm_customers` is gone).
///
/// The index statements assume `customers` is already the party table. A fresh
/// install still has the address-shaped `customers` here, so those run only
/// when `is_individual` exists.
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

pub async fn apply(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() != DbBackend::Sqlite {
        let Some((consolidate, indexes)) = UP.split_first() else {
            return Ok(());
        };
        exec_sql(manager, consolidate).await?;
        if column_exists(manager, "customers", "is_individual").await? {
            for sql in indexes {
                exec_sql(manager, sql).await?;
            }
        }
        return Ok(());
    }
    apply_sqlite(manager).await
}

/// Same decisions as the Postgres `DO` block. SQLite cannot rewrite foreign
/// keys in place, and a fresh install's `customers` table does not yet have
/// `company_id`. An empty `crm_customers` is dropped; a later migration
/// recreates it and renames it after the legacy table is gone.
async fn apply_sqlite(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if table_exists(manager, "crm_customers").await? {
        if !table_exists(manager, "customers").await? {
            exec_sql(manager, "ALTER TABLE crm_customers RENAME TO customers").await?;
        } else if !has_row(manager, "crm_customers").await? {
            exec_sql(manager, "DROP TABLE crm_customers").await?;
        } else if column_exists(manager, "customers", "is_individual").await? {
            copy_crm_customers(manager).await?;
            exec_sql(manager, "DROP TABLE crm_customers").await?;
            exec_sql(
                manager,
                "UPDATE sqlite_sequence SET seq = COALESCE((SELECT MAX(id) FROM customers), 0) WHERE name = 'customers'",
            )
            .await?;
        } else {
            return Err(DbErr::Custom(
                "crm_customers has rows but customers is still the legacy address table".into(),
            ));
        }
    }
    if column_exists(manager, "customers", "company_id").await? {
        for sql in UP.iter().skip(1) {
            exec_sql(manager, sql).await?;
        }
    }
    Ok(())
}

async fn copy_crm_customers(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if !has_row(manager, "customers").await? {
        return exec_sql(
            manager,
            r#"
INSERT INTO customers (
    id, created_at, updated_at, is_individual, company_id, individual_id
)
SELECT id, created_at, updated_at, is_individual, company_id, individual_id
  FROM crm_customers
"#,
        )
        .await;
    }
    exec_sql(
        manager,
        r#"
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
              (NOT c.is_individual AND x.company_id IS NOT DISTINCT FROM c.company_id)
              OR (c.is_individual AND x.individual_id IS NOT DISTINCT FROM c.individual_id)
          )
 )
"#,
    )
    .await
}

async fn table_exists(manager: &SchemaManager<'_>, table: &str) -> Result<bool, DbErr> {
    row_exists(
        manager,
        &format!("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '{table}'"),
    )
    .await
}

async fn column_exists(
    manager: &SchemaManager<'_>,
    table: &str,
    column: &str,
) -> Result<bool, DbErr> {
    let sql = if manager.get_database_backend() == DbBackend::Sqlite {
        format!("SELECT 1 FROM pragma_table_info('{table}') WHERE name = '{column}'")
    } else {
        format!(
            "SELECT 1 FROM information_schema.columns \
             WHERE table_schema = 'public' AND table_name = '{table}' AND column_name = '{column}'"
        )
    };
    row_exists(manager, &sql).await
}

async fn has_row(manager: &SchemaManager<'_>, table: &str) -> Result<bool, DbErr> {
    row_exists(manager, &format!("SELECT 1 FROM \"{table}\" LIMIT 1")).await
}

async fn row_exists(manager: &SchemaManager<'_>, sql: &str) -> Result<bool, DbErr> {
    let row = manager
        .get_connection()
        .query_one_raw(Statement::from_string(manager.get_database_backend(), sql))
        .await?;
    Ok(row.is_some())
}
