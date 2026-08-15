//! Deleting an invoice deletes the payment term it points at.
//!
//! The invoice → term FK is ON DELETE SET NULL so a term can be removed without
//! deleting the invoice. The inverse (invoice gone, term leftover) is enforced
//! here: term lines already cascade from the term.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const CREATE_DRAFT_FN: &str = r#"
CREATE OR REPLACE FUNCTION delete_draft_payment_term_for_deleted_invoice()
RETURNS trigger AS $$
BEGIN
  IF OLD.draft_payment_term_id IS NOT NULL THEN
    DELETE FROM draft_payment_terms WHERE id = OLD.draft_payment_term_id;
  END IF;
  RETURN OLD;
END;
$$ LANGUAGE plpgsql
"#;

const CREATE_POSTED_FN: &str = r#"
CREATE OR REPLACE FUNCTION delete_posted_payment_term_for_deleted_invoice()
RETURNS trigger AS $$
BEGIN
  IF OLD.posted_payment_term_id IS NOT NULL THEN
    DELETE FROM posted_payment_terms WHERE id = OLD.posted_payment_term_id;
  END IF;
  RETURN OLD;
END;
$$ LANGUAGE plpgsql
"#;

const CREATE_DRAFT_TRIGGER: &str = r#"
CREATE TRIGGER trg_draft_invoices_delete_payment_term
AFTER DELETE ON draft_invoices
FOR EACH ROW EXECUTE PROCEDURE delete_draft_payment_term_for_deleted_invoice()
"#;

const CREATE_POSTED_TRIGGER: &str = r#"
CREATE TRIGGER trg_posted_invoices_delete_payment_term
AFTER DELETE ON posted_invoices
FOR EACH ROW EXECUTE PROCEDURE delete_posted_payment_term_for_deleted_invoice()
"#;

const CREATE_CANCELLED_TRIGGER: &str = r#"
CREATE TRIGGER trg_cancelled_invoices_delete_payment_term
AFTER DELETE ON cancelled_invoices
FOR EACH ROW EXECUTE PROCEDURE delete_posted_payment_term_for_deleted_invoice()
"#;

const DROP_DRAFT_TRIGGER: &str =
    "DROP TRIGGER IF EXISTS trg_draft_invoices_delete_payment_term ON draft_invoices";
const DROP_POSTED_TRIGGER: &str =
    "DROP TRIGGER IF EXISTS trg_posted_invoices_delete_payment_term ON posted_invoices";
const DROP_CANCELLED_TRIGGER: &str =
    "DROP TRIGGER IF EXISTS trg_cancelled_invoices_delete_payment_term ON cancelled_invoices";
const DROP_DRAFT_FN: &str =
    "DROP FUNCTION IF EXISTS delete_draft_payment_term_for_deleted_invoice()";
const DROP_POSTED_FN: &str =
    "DROP FUNCTION IF EXISTS delete_posted_payment_term_for_deleted_invoice()";

async fn execute(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute_unprepared(sql)
        .await
        .map(|_| ())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute(manager, CREATE_DRAFT_FN).await?;
        execute(manager, CREATE_POSTED_FN).await?;
        execute(manager, CREATE_DRAFT_TRIGGER).await?;
        execute(manager, CREATE_POSTED_TRIGGER).await?;
        execute(manager, CREATE_CANCELLED_TRIGGER).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute(manager, DROP_DRAFT_TRIGGER).await?;
        execute(manager, DROP_POSTED_TRIGGER).await?;
        execute(manager, DROP_CANCELLED_TRIGGER).await?;
        execute(manager, DROP_DRAFT_FN).await?;
        execute(manager, DROP_POSTED_FN).await
    }
}
