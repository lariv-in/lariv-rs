//! Service catalog, default taxes, and a copy of service-typed products.

use lariv_core::db::trigram;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            r#"CREATE TABLE IF NOT EXISTS services (
                id bigserial PRIMARY KEY,
                created_at timestamptz NULL,
                updated_at timestamptz NULL,
                legacy_product_id bigint NULL UNIQUE,
                reference text NULL,
                remarks text NULL,
                name text NOT NULL,
                sac_code bigint NOT NULL,
                variables text NOT NULL DEFAULT '{}',
                base_price_formula text NOT NULL,
                sales_price_formula text NOT NULL
            )"#,
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS uq_services_reference ON services (reference)",
        )
        .await?;
        conn.execute_unprepared(
            r#"CREATE TABLE IF NOT EXISTS service_taxes (
                service_id bigint NOT NULL REFERENCES services(id) ON UPDATE CASCADE ON DELETE CASCADE,
                tax_id bigint NOT NULL REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                PRIMARY KEY (service_id, tax_id)
            )"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"CREATE TABLE IF NOT EXISTS service_preferences (
                id bigserial PRIMARY KEY,
                created_at timestamptz NULL,
                updated_at timestamptz NULL,
                cost_of_sales_account_id bigint NULL REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                cost_credit_account_id bigint NULL REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE RESTRICT
            )"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"CREATE TABLE IF NOT EXISTS service_preferences_taxes (
                service_preferences_id bigint NOT NULL REFERENCES service_preferences(id) ON UPDATE CASCADE ON DELETE CASCADE,
                tax_id bigint NOT NULL REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                PRIMARY KEY (service_preferences_id, tax_id)
            )"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"
            DO $$
            BEGIN
                IF to_regclass('public.products') IS NULL THEN
                    RETURN;
                END IF;
                IF NOT EXISTS (
                    SELECT 1
                    FROM information_schema.columns
                    WHERE table_schema = 'public'
                      AND table_name = 'products'
                      AND column_name = 'product_type'
                ) THEN
                    RETURN;
                END IF;
                INSERT INTO services (
                    legacy_product_id, created_at, updated_at, reference, remarks, name,
                    sac_code, variables, base_price_formula, sales_price_formula
                )
                SELECT
                    id, created_at, updated_at, reference, remarks, name,
                    hsn_code, variables, base_price_formula, sales_price_formula
                FROM products
                WHERE product_type::text IN ('Services', 'Both')
                  AND NOT EXISTS (
                      SELECT 1 FROM services WHERE services.legacy_product_id = products.id
                  );
                INSERT INTO service_taxes (service_id, tax_id)
                SELECT service.id, product_tax.tax_id
                FROM services AS service
                JOIN product_taxes AS product_tax
                  ON product_tax.product_id = service.legacy_product_id
                ON CONFLICT DO NOTHING;
            END $$;
            "#,
        )
        .await?;
        let backend = manager.get_database_backend();
        trigram::create_gin_index(conn, backend, "services_name_trgm_idx", "services", "name")
            .await?;
        trigram::create_gin_index(
            conn,
            backend,
            "services_reference_trgm_idx",
            "services",
            "reference",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared("DROP TABLE IF EXISTS service_preferences_taxes")
            .await?;
        conn.execute_unprepared("DROP TABLE IF EXISTS service_preferences")
            .await?;
        conn.execute_unprepared("DROP TABLE IF EXISTS service_taxes")
            .await?;
        conn.execute_unprepared("DROP TABLE IF EXISTS services")
            .await?;
        Ok(())
    }
}
