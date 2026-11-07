//! Copy each payment's customer from its posted invoice, retarget batch source
//! documents onto one child payment, then drop batches and the invoice FK.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(
            manager,
            "ALTER TABLE payments ADD COLUMN IF NOT EXISTS customer_id bigint",
        )
        .await?;
        exec_sql(
            manager,
            r#"UPDATE payments AS p
               SET customer_id = i.customer_id
               FROM posted_invoices AS i
               WHERE p.posted_invoice_id = i.id
                 AND p.customer_id IS NULL"#,
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE payments ALTER COLUMN customer_id SET NOT NULL",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE payments DROP CONSTRAINT IF EXISTS fk_payments_customer_id",
        )
        .await?;
        exec_sql(
            manager,
            r#"ALTER TABLE payments
               ADD CONSTRAINT fk_payments_customer_id
               FOREIGN KEY (customer_id) REFERENCES customers(id)
               ON DELETE RESTRICT ON UPDATE CASCADE"#,
        )
        .await?;
        exec_sql(
            manager,
            "CREATE INDEX IF NOT EXISTS idx_payments_customer_id ON payments (customer_id)",
        )
        .await?;

        exec_sql(
            manager,
            r#"UPDATE source_docs
               SET source_doc_type = 'p_finance_payments.Payment',
                   updated_at = NOW()
               WHERE source_doc_type = 'p_finance_invoices.Payment'"#,
        )
        .await?;
        exec_sql(
            manager,
            r#"UPDATE source_docs AS sd
               SET source_doc_type = 'p_finance_payments.Payment',
                   source_doc_id = sub.payment_id,
                   updated_at = NOW()
               FROM (
                   SELECT DISTINCT ON (payment_batch_id)
                       payment_batch_id,
                       id AS payment_id
                   FROM payments
                   WHERE payment_batch_id IS NOT NULL
                   ORDER BY payment_batch_id, id
               ) AS sub
               WHERE sd.source_doc_type = 'p_finance_invoices.PaymentBatch'
                 AND sd.source_doc_id = sub.payment_batch_id"#,
        )
        .await?;
        exec_sql(
            manager,
            r#"UPDATE source_docs
               SET source_doc_type = 'p_finance_payments.Payment',
                   source_doc_id = 0,
                   updated_at = NOW()
               WHERE source_doc_type = 'p_finance_invoices.PaymentBatch'"#,
        )
        .await?;

        exec_sql(
            manager,
            "ALTER TABLE payments DROP CONSTRAINT IF EXISTS fk_payments_payment_batch_id",
        )
        .await?;
        exec_sql(
            manager,
            "DROP INDEX IF EXISTS idx_payments_payment_batch_id",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE payments DROP COLUMN IF EXISTS payment_batch_id",
        )
        .await?;
        exec_sql(manager, "DROP TABLE IF EXISTS payment_batches").await?;

        exec_sql(
            manager,
            "ALTER TABLE payments DROP CONSTRAINT IF EXISTS fk_payments_posted_invoice_id",
        )
        .await?;
        exec_sql(
            manager,
            "DROP INDEX IF EXISTS idx_payments_posted_invoice_id",
        )
        .await?;
        exec_sql(
            manager,
            "ALTER TABLE payments DROP COLUMN IF EXISTS posted_invoice_id",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let _ = manager;
        Err(DbErr::Custom(
            "payment customer migration cannot be reversed".into(),
        ))
    }
}
