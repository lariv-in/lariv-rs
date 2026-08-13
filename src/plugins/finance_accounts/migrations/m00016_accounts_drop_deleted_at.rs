use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const REWRITE_FUNCTION: &str = r#"
CREATE OR REPLACE FUNCTION accounts_enforce_parent_balance_type() RETURNS TRIGGER AS $fn$
BEGIN
  IF NEW.parent_id IS NOT NULL THEN
    IF NOT EXISTS (
      SELECT 1 FROM accounts AS p
      WHERE p.id = NEW.parent_id
        AND p.balance_type = NEW.balance_type
    ) THEN
      RAISE EXCEPTION 'balance_type must match the parent account balance_type';
    END IF;
  END IF;
  IF TG_OP = 'UPDATE' AND NEW.balance_type IS DISTINCT FROM OLD.balance_type THEN
    IF EXISTS (
      SELECT 1 FROM accounts AS c
      WHERE c.parent_id = NEW.id
        AND c.balance_type IS DISTINCT FROM NEW.balance_type
    ) THEN
      RAISE EXCEPTION 'cannot change balance_type while child accounts have a different balance_type';
    END IF;
  END IF;
  RETURN NEW;
END;
$fn$ LANGUAGE plpgsql
"#;

async fn execute(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(
            manager.get_connection().get_database_backend(),
            sql.to_string(),
        ))
        .await
        .map(|_| ())
}

async fn execute_if_exists(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    // Invoice/creditnote tables may not exist yet depending on plugin install order.
    let wrapped = format!(
        r#"
DO $do$
BEGIN
  BEGIN
    {sql};
  EXCEPTION
    WHEN undefined_table THEN NULL;
  END;
END
$do$;
"#
    );
    execute(manager, &wrapped).await
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Clear invoice/creditnote FKs that point at soft-deleted JE/items (tables may be absent).
        execute_if_exists(
            manager,
            r#"DELETE FROM posted_invoice_lines
               WHERE journal_entry_item_id IN (
                 SELECT id FROM journal_entry_items WHERE deleted_at IS NOT NULL
               )
               OR journal_entry_item_id IN (
                 SELECT jei.id FROM journal_entry_items jei
                 JOIN journal_entries je ON je.id = jei.journal_entry_id
                 WHERE je.deleted_at IS NOT NULL
               )"#,
        )
        .await?;
        execute_if_exists(
            manager,
            r#"DELETE FROM cancelled_invoice_lines
               WHERE journal_entry_item_id IN (
                 SELECT id FROM journal_entry_items WHERE deleted_at IS NOT NULL
               )
               OR journal_entry_item_id IN (
                 SELECT jei.id FROM journal_entry_items jei
                 JOIN journal_entries je ON je.id = jei.journal_entry_id
                 WHERE je.deleted_at IS NOT NULL
               )"#,
        )
        .await?;
        execute_if_exists(
            manager,
            r#"DELETE FROM paid_invoices
               WHERE posted_invoice_id IN (
                 SELECT id FROM posted_invoices
                 WHERE deleted_at IS NOT NULL
                    OR journal_entry_id IN (
                      SELECT id FROM journal_entries WHERE deleted_at IS NOT NULL
                    )
               )"#,
        )
        .await?;
        execute_if_exists(
            manager,
            r#"DELETE FROM partially_paid_invoices
               WHERE posted_invoice_id IN (
                 SELECT id FROM posted_invoices
                 WHERE deleted_at IS NOT NULL
                    OR journal_entry_id IN (
                      SELECT id FROM journal_entries WHERE deleted_at IS NOT NULL
                    )
               )"#,
        )
        .await?;
        execute_if_exists(
            manager,
            r#"DELETE FROM payments
               WHERE deleted_at IS NOT NULL
                  OR journal_entry_id IN (
                    SELECT id FROM journal_entries WHERE deleted_at IS NOT NULL
                  )
                  OR posted_invoice_id IN (
                    SELECT id FROM posted_invoices
                    WHERE deleted_at IS NOT NULL
                       OR journal_entry_id IN (
                         SELECT id FROM journal_entries WHERE deleted_at IS NOT NULL
                       )
                  )"#,
        )
        .await?;
        execute_if_exists(
            manager,
            r#"DELETE FROM payment_batches
               WHERE deleted_at IS NOT NULL
                  OR journal_entry_id IN (
                    SELECT id FROM journal_entries WHERE deleted_at IS NOT NULL
                  )"#,
        )
        .await?;
        execute_if_exists(
            manager,
            r#"DELETE FROM cancelled_invoices
               WHERE deleted_at IS NOT NULL
                  OR posted_invoice_id IN (
                    SELECT id FROM posted_invoices
                    WHERE deleted_at IS NOT NULL
                       OR journal_entry_id IN (
                         SELECT id FROM journal_entries WHERE deleted_at IS NOT NULL
                       )
                  )
                  OR credit_note_id IN (
                    SELECT id FROM credit_notes
                    WHERE deleted_at IS NOT NULL
                       OR journal_entry_id IN (
                         SELECT id FROM journal_entries WHERE deleted_at IS NOT NULL
                       )
                       OR reversed_journal_entry_id IN (
                         SELECT id FROM journal_entries WHERE deleted_at IS NOT NULL
                       )
                  )"#,
        )
        .await?;
        execute_if_exists(
            manager,
            r#"DELETE FROM posted_invoice_lines
               WHERE deleted_at IS NOT NULL
                  OR posted_invoice_id IN (
                    SELECT id FROM posted_invoices
                    WHERE deleted_at IS NOT NULL
                       OR journal_entry_id IN (
                         SELECT id FROM journal_entries WHERE deleted_at IS NOT NULL
                       )
                  )"#,
        )
        .await?;
        execute_if_exists(
            manager,
            r#"DELETE FROM posted_invoices
               WHERE deleted_at IS NOT NULL
                  OR journal_entry_id IN (
                    SELECT id FROM journal_entries WHERE deleted_at IS NOT NULL
                  )"#,
        )
        .await?;
        execute_if_exists(
            manager,
            r#"DELETE FROM credit_notes
               WHERE deleted_at IS NOT NULL
                  OR journal_entry_id IN (
                    SELECT id FROM journal_entries WHERE deleted_at IS NOT NULL
                  )
                  OR reversed_journal_entry_id IN (
                    SELECT id FROM journal_entries WHERE deleted_at IS NOT NULL
                  )"#,
        )
        .await?;

        // Accounts-owned purge (FK order).
        execute(
            manager,
            r#"DELETE FROM journal_entry_items
               WHERE deleted_at IS NOT NULL
                  OR journal_entry_id IN (
                    SELECT id FROM journal_entries WHERE deleted_at IS NOT NULL
                  )"#,
        )
        .await?;
        execute(
            manager,
            "DELETE FROM journal_entries WHERE deleted_at IS NOT NULL",
        )
        .await?;
        execute(
            manager,
            "DELETE FROM source_docs WHERE deleted_at IS NOT NULL",
        )
        .await?;
        execute(manager, "DELETE FROM journals WHERE deleted_at IS NOT NULL").await?;
        execute(
            manager,
            "DELETE FROM currencies WHERE deleted_at IS NOT NULL",
        )
        .await?;
        execute(
            manager,
            "DELETE FROM accounting_preferences WHERE deleted_at IS NOT NULL",
        )
        .await?;
        execute(manager, "DELETE FROM accounts WHERE deleted_at IS NOT NULL").await?;

        execute(manager, REWRITE_FUNCTION).await?;

        for (index, table) in [
            ("idx_journal_entry_items_deleted_at", "journal_entry_items"),
            ("idx_journal_entries_deleted_at", "journal_entries"),
            ("idx_source_docs_deleted_at", "source_docs"),
            ("idx_journals_deleted_at", "journals"),
            ("idx_currencies_deleted_at", "currencies"),
            (
                "idx_accounting_preferences_deleted_at",
                "accounting_preferences",
            ),
            ("idx_accounts_deleted_at", "accounts"),
        ] {
            manager
                .drop_index(
                    Index::drop()
                        .name(index)
                        .table(Alias::new(table))
                        .to_owned(),
                )
                .await?;
        }

        for (table, col) in [
            ("journal_entry_items", "deleted_at"),
            ("journal_entries", "deleted_at"),
            ("source_docs", "deleted_at"),
            ("journals", "deleted_at"),
            ("currencies", "deleted_at"),
            ("accounting_preferences", "deleted_at"),
            ("accounts", "deleted_at"),
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Alias::new(table))
                        .drop_column(Alias::new(col))
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            "accounts",
            "accounting_preferences",
            "currencies",
            "journals",
            "source_docs",
            "journal_entries",
            "journal_entry_items",
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Alias::new(table))
                        .add_column(
                            ColumnDef::new(Alias::new("deleted_at")).timestamp_with_time_zone(),
                        )
                        .to_owned(),
                )
                .await?;
        }

        for (index, table) in [
            ("idx_accounts_deleted_at", "accounts"),
            (
                "idx_accounting_preferences_deleted_at",
                "accounting_preferences",
            ),
            ("idx_currencies_deleted_at", "currencies"),
            ("idx_journals_deleted_at", "journals"),
            ("idx_source_docs_deleted_at", "source_docs"),
            ("idx_journal_entries_deleted_at", "journal_entries"),
            ("idx_journal_entry_items_deleted_at", "journal_entry_items"),
        ] {
            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name(index)
                        .table(Alias::new(table))
                        .col(Alias::new("deleted_at"))
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}
