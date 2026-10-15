//! Point stock movements at `crm_customers` and drop the company/individual columns.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &[&str] = &[
    "INSERT INTO crm_customers (is_individual, company_id, individual_id, created_at, updated_at)
     SELECT FALSE, party.company_id, NULL, NOW(), NOW()
     FROM (
        SELECT customer_company AS company_id FROM inventory_stock_movements_in
        WHERE NOT bill_to_individual AND customer_company IS NOT NULL
        UNION
        SELECT customer_company FROM inventory_stock_movements_out
        WHERE NOT bill_to_individual AND customer_company IS NOT NULL
     ) AS party
     WHERE NOT EXISTS (
        SELECT 1 FROM crm_customers AS existing
        WHERE NOT existing.is_individual AND existing.company_id = party.company_id
     )",
    "INSERT INTO crm_customers (is_individual, company_id, individual_id, created_at, updated_at)
     SELECT TRUE, NULL, party.individual_id, NOW(), NOW()
     FROM (
        SELECT customer_individual AS individual_id FROM inventory_stock_movements_in
        WHERE bill_to_individual AND customer_individual IS NOT NULL
        UNION
        SELECT customer_individual FROM inventory_stock_movements_out
        WHERE bill_to_individual AND customer_individual IS NOT NULL
     ) AS party
     WHERE NOT EXISTS (
        SELECT 1 FROM crm_customers AS existing
        WHERE existing.is_individual AND existing.individual_id = party.individual_id
     )",
    "ALTER TABLE inventory_stock_movements_in ADD COLUMN IF NOT EXISTS customer_id bigint",
    "ALTER TABLE inventory_stock_movements_out ADD COLUMN IF NOT EXISTS customer_id bigint",
    "UPDATE inventory_stock_movements_in AS m
     SET customer_id = c.id
     FROM crm_customers AS c
     WHERE NOT m.bill_to_individual
       AND NOT c.is_individual
       AND c.company_id = m.customer_company",
    "UPDATE inventory_stock_movements_in AS m
     SET customer_id = c.id
     FROM crm_customers AS c
     WHERE m.bill_to_individual
       AND c.is_individual
       AND c.individual_id = m.customer_individual",
    "UPDATE inventory_stock_movements_out AS m
     SET customer_id = c.id
     FROM crm_customers AS c
     WHERE NOT m.bill_to_individual
       AND NOT c.is_individual
       AND c.company_id = m.customer_company",
    "UPDATE inventory_stock_movements_out AS m
     SET customer_id = c.id
     FROM crm_customers AS c
     WHERE m.bill_to_individual
       AND c.is_individual
       AND c.individual_id = m.customer_individual",
    "DO $$ BEGIN
        IF EXISTS (SELECT 1 FROM inventory_stock_movements_in WHERE customer_id IS NULL)
           OR EXISTS (SELECT 1 FROM inventory_stock_movements_out WHERE customer_id IS NULL)
        THEN
            RAISE EXCEPTION 'stock movement is missing a migrated customer';
        END IF;
     END $$",
    "ALTER TABLE inventory_stock_movements_in ALTER COLUMN customer_id SET NOT NULL",
    "ALTER TABLE inventory_stock_movements_out ALTER COLUMN customer_id SET NOT NULL",
    "ALTER TABLE inventory_stock_movements_in DROP CONSTRAINT IF EXISTS fk_inventory_stock_movements_in_customer_id",
    "ALTER TABLE inventory_stock_movements_in ADD CONSTRAINT fk_inventory_stock_movements_in_customer_id
        FOREIGN KEY (customer_id) REFERENCES crm_customers(id)
        ON DELETE RESTRICT ON UPDATE CASCADE",
    "ALTER TABLE inventory_stock_movements_out DROP CONSTRAINT IF EXISTS fk_inventory_stock_movements_out_customer_id",
    "ALTER TABLE inventory_stock_movements_out ADD CONSTRAINT fk_inventory_stock_movements_out_customer_id
        FOREIGN KEY (customer_id) REFERENCES crm_customers(id)
        ON DELETE RESTRICT ON UPDATE CASCADE",
    "ALTER TABLE inventory_stock_movements_in DROP CONSTRAINT IF EXISTS ck_inventory_stock_movements_in_bill_to",
    "ALTER TABLE inventory_stock_movements_in DROP CONSTRAINT IF EXISTS fk_inventory_stock_movements_in_customer_individual",
    "ALTER TABLE inventory_stock_movements_in DROP CONSTRAINT IF EXISTS fk_inventory_stock_movements_in_customer_company",
    "ALTER TABLE inventory_stock_movements_in DROP COLUMN IF EXISTS bill_to_individual",
    "ALTER TABLE inventory_stock_movements_in DROP COLUMN IF EXISTS customer_individual",
    "ALTER TABLE inventory_stock_movements_in DROP COLUMN IF EXISTS customer_company",
    "ALTER TABLE inventory_stock_movements_out DROP CONSTRAINT IF EXISTS ck_inventory_stock_movements_out_bill_to",
    "ALTER TABLE inventory_stock_movements_out DROP CONSTRAINT IF EXISTS fk_inventory_stock_movements_out_customer_individual",
    "ALTER TABLE inventory_stock_movements_out DROP CONSTRAINT IF EXISTS fk_inventory_stock_movements_out_customer_company",
    "ALTER TABLE inventory_stock_movements_out DROP COLUMN IF EXISTS bill_to_individual",
    "ALTER TABLE inventory_stock_movements_out DROP COLUMN IF EXISTS customer_individual",
    "ALTER TABLE inventory_stock_movements_out DROP COLUMN IF EXISTS customer_company",
    "CREATE INDEX IF NOT EXISTS idx_inventory_stock_movements_in_customer_id
        ON inventory_stock_movements_in (customer_id)",
    "CREATE INDEX IF NOT EXISTS idx_inventory_stock_movements_out_customer_id
        ON inventory_stock_movements_out (customer_id)",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in UP {
            exec_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "stock movement customer migration is not reversible".into(),
        ))
    }
}
