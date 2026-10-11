//! Current purchase document schema (draft, posted, cancelled). No payment recording.

use lariv_core::db::migration_sql::exec_sql;
use lariv_core::db::trigram;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS draft_purchase_payment_terms (
        id bigserial PRIMARY KEY,
        created_at timestamptz,
        updated_at timestamptz
    )",
    "CREATE TABLE IF NOT EXISTS draft_purchase_payment_term_lines (
        id bigserial PRIMARY KEY,
        created_at timestamptz,
        updated_at timestamptz,
        draft_payment_term_id bigint NOT NULL REFERENCES draft_purchase_payment_terms(id) ON DELETE CASCADE,
        line_order integer NOT NULL,
        date_kind varchar(32) NOT NULL,
        due_date date,
        due_duration bigint,
        amount_kind varchar(32) NOT NULL,
        amount numeric(19,6),
        amount_percentage numeric(19,6)
    )",
    "CREATE TABLE IF NOT EXISTS posted_purchase_payment_terms (
        id bigserial PRIMARY KEY,
        created_at timestamptz,
        updated_at timestamptz
    )",
    "CREATE TABLE IF NOT EXISTS posted_purchase_payment_term_lines (
        id bigserial PRIMARY KEY,
        created_at timestamptz,
        updated_at timestamptz,
        posted_payment_term_id bigint NOT NULL REFERENCES posted_purchase_payment_terms(id) ON DELETE CASCADE,
        line_order integer NOT NULL,
        due_date date NOT NULL,
        amount numeric(19,6) NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS draft_purchases (
        id bigserial PRIMARY KEY,
        created_at timestamptz,
        updated_at timestamptz,
        number text,
        reference text,
        payment_reference text,
        bank_account text,
        remarks text,
        datetime timestamptz NOT NULL,
        delivery_date date,
        vendor_is_individual boolean NOT NULL,
        vendor_contact_id bigint REFERENCES crm_contacts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        vendor_company_id bigint REFERENCES crm_companies(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        draft_payment_term_id bigint UNIQUE REFERENCES draft_purchase_payment_terms(id) ON DELETE SET NULL
    )",
    "CREATE TABLE IF NOT EXISTS draft_purchase_taxes (
        draft_purchase_id bigint NOT NULL REFERENCES draft_purchases(id) ON DELETE CASCADE,
        tax_id bigint NOT NULL REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        PRIMARY KEY (draft_purchase_id, tax_id)
    )",
    "CREATE TABLE IF NOT EXISTS draft_purchase_lines (
        id bigserial PRIMARY KEY,
        created_at timestamptz,
        updated_at timestamptz,
        draft_purchase_id bigint NOT NULL REFERENCES draft_purchases(id) ON DELETE CASCADE,
        product_id bigint NOT NULL REFERENCES products(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        rate numeric(19,6) NOT NULL,
        quantity numeric(19,6) NOT NULL,
        variable_values text NOT NULL DEFAULT '{}',
        pre_tax_amount numeric(19,6) NOT NULL,
        remarks text
    )",
    "CREATE TABLE IF NOT EXISTS draft_purchase_line_taxes (
        draft_purchase_line_id bigint NOT NULL REFERENCES draft_purchase_lines(id) ON DELETE CASCADE,
        tax_id bigint NOT NULL REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        PRIMARY KEY (draft_purchase_line_id, tax_id)
    )",
    "CREATE TABLE IF NOT EXISTS posted_purchases (
        id bigserial PRIMARY KEY,
        created_at timestamptz,
        updated_at timestamptz,
        draft_purchase_id bigint NOT NULL UNIQUE REFERENCES draft_purchases(id) ON DELETE RESTRICT,
        posted_at timestamptz,
        number text NOT NULL,
        reference text,
        payment_reference text,
        bank_account text,
        remarks text,
        account_payable_id bigint NOT NULL REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        account_expense_id bigint NOT NULL REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        account_input_tax_id bigint NOT NULL REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        journal_id bigint NOT NULL REFERENCES journals(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        datetime timestamptz NOT NULL,
        delivery_date date,
        vendor_is_individual boolean NOT NULL,
        vendor_contact_id bigint REFERENCES crm_contacts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        vendor_company_id bigint REFERENCES crm_companies(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        journal_entry_id bigint NOT NULL REFERENCES journal_entries(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        posted_payment_term_id bigint UNIQUE REFERENCES posted_purchase_payment_terms(id) ON DELETE SET NULL
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS uix_posted_purchases_number ON posted_purchases (number)",
    "CREATE INDEX IF NOT EXISTS idx_posted_purchases_journal_entry_id ON posted_purchases (journal_entry_id)",
    "CREATE TABLE IF NOT EXISTS posted_purchase_taxes (
        posted_purchase_id bigint NOT NULL REFERENCES posted_purchases(id) ON DELETE CASCADE,
        tax_id bigint NOT NULL REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        PRIMARY KEY (posted_purchase_id, tax_id)
    )",
    "CREATE TABLE IF NOT EXISTS posted_purchase_lines (
        id bigserial PRIMARY KEY,
        created_at timestamptz,
        updated_at timestamptz,
        posted_purchase_id bigint NOT NULL REFERENCES posted_purchases(id) ON DELETE CASCADE,
        product_id bigint NOT NULL REFERENCES products(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        rate numeric(19,6) NOT NULL,
        quantity numeric(19,6) NOT NULL,
        variable_values text NOT NULL DEFAULT '{}',
        pre_tax_amount numeric(19,6) NOT NULL,
        remarks text,
        journal_entry_item_id bigint NOT NULL REFERENCES journal_entry_items(id) ON UPDATE CASCADE ON DELETE RESTRICT
    )",
    "CREATE TABLE IF NOT EXISTS posted_purchase_line_taxes (
        posted_purchase_line_id bigint NOT NULL REFERENCES posted_purchase_lines(id) ON DELETE CASCADE,
        tax_id bigint NOT NULL REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        PRIMARY KEY (posted_purchase_line_id, tax_id)
    )",
    "CREATE TABLE IF NOT EXISTS cancelled_purchases (
        id bigserial PRIMARY KEY,
        created_at timestamptz,
        updated_at timestamptz,
        posted_purchase_id bigint NOT NULL REFERENCES posted_purchases(id) ON DELETE RESTRICT,
        posted_at timestamptz,
        cancelled_at timestamptz,
        number text NOT NULL,
        reference text,
        payment_reference text,
        bank_account text,
        remarks text,
        account_payable_id bigint NOT NULL REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        account_expense_id bigint NOT NULL REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        account_input_tax_id bigint NOT NULL REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        journal_id bigint NOT NULL REFERENCES journals(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        datetime timestamptz NOT NULL,
        delivery_date date,
        vendor_is_individual boolean NOT NULL,
        vendor_contact_id bigint REFERENCES crm_contacts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        vendor_company_id bigint REFERENCES crm_companies(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        reversed_journal_entry_id bigint NOT NULL REFERENCES journal_entries(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        reason text,
        posted_payment_term_id bigint UNIQUE REFERENCES posted_purchase_payment_terms(id) ON DELETE SET NULL
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS uix_cancelled_purchases_number ON cancelled_purchases (number)",
    "CREATE INDEX IF NOT EXISTS idx_cancelled_purchases_reversed_journal_entry_id ON cancelled_purchases (reversed_journal_entry_id)",
    "CREATE TABLE IF NOT EXISTS cancelled_purchase_taxes (
        cancelled_purchase_id bigint NOT NULL REFERENCES cancelled_purchases(id) ON DELETE CASCADE,
        tax_id bigint NOT NULL REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        PRIMARY KEY (cancelled_purchase_id, tax_id)
    )",
    "CREATE TABLE IF NOT EXISTS cancelled_purchase_lines (
        id bigserial PRIMARY KEY,
        created_at timestamptz,
        updated_at timestamptz,
        cancelled_purchase_id bigint NOT NULL REFERENCES cancelled_purchases(id) ON DELETE CASCADE,
        product_id bigint NOT NULL REFERENCES products(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        rate numeric(19,6) NOT NULL,
        quantity numeric(19,6) NOT NULL,
        variable_values text NOT NULL DEFAULT '{}',
        pre_tax_amount numeric(19,6) NOT NULL,
        remarks text,
        journal_entry_item_id bigint NOT NULL REFERENCES journal_entry_items(id) ON UPDATE CASCADE ON DELETE RESTRICT
    )",
    "CREATE TABLE IF NOT EXISTS cancelled_purchase_line_taxes (
        cancelled_purchase_line_id bigint NOT NULL REFERENCES cancelled_purchase_lines(id) ON DELETE CASCADE,
        tax_id bigint NOT NULL REFERENCES taxes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        PRIMARY KEY (cancelled_purchase_line_id, tax_id)
    )",
    "CREATE TABLE IF NOT EXISTS purchase_preferences (
        id bigserial PRIMARY KEY,
        created_at timestamptz,
        updated_at timestamptz,
        account_payable_id bigint REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        account_expense_id bigint REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        account_input_tax_id bigint REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        journal_id bigint REFERENCES journals(id) ON UPDATE CASCADE ON DELETE RESTRICT,
        purchase_number_format text,
        purchase_date_format text,
        purchase_datetime_format text,
        purchase_pdf_template text,
        purchase_logo_vnode_id bigint,
        purchase_signature_vnode_id bigint,
        company_name text,
        company_address text,
        company_phone text,
        company_gstin text,
        place_of_supply text,
        default_bank_account text
    )",
    r#"CREATE OR REPLACE FUNCTION delete_draft_purchase_payment_term()
       RETURNS trigger AS $$
       BEGIN
         IF OLD.draft_payment_term_id IS NOT NULL THEN
           DELETE FROM draft_purchase_payment_terms WHERE id = OLD.draft_payment_term_id;
         END IF;
         RETURN OLD;
       END;
       $$ LANGUAGE plpgsql"#,
    r#"CREATE OR REPLACE FUNCTION delete_posted_purchase_payment_term()
       RETURNS trigger AS $$
       BEGIN
         IF OLD.posted_payment_term_id IS NOT NULL THEN
           DELETE FROM posted_purchase_payment_terms WHERE id = OLD.posted_payment_term_id;
         END IF;
         RETURN OLD;
       END;
       $$ LANGUAGE plpgsql"#,
    "DROP TRIGGER IF EXISTS trg_draft_purchases_delete_payment_term ON draft_purchases",
    "CREATE TRIGGER trg_draft_purchases_delete_payment_term
        AFTER DELETE ON draft_purchases
        FOR EACH ROW EXECUTE PROCEDURE delete_draft_purchase_payment_term()",
    "DROP TRIGGER IF EXISTS trg_posted_purchases_delete_payment_term ON posted_purchases",
    "CREATE TRIGGER trg_posted_purchases_delete_payment_term
        AFTER DELETE ON posted_purchases
        FOR EACH ROW EXECUTE PROCEDURE delete_posted_purchase_payment_term()",
    "DROP TRIGGER IF EXISTS trg_cancelled_purchases_delete_payment_term ON cancelled_purchases",
    "CREATE TRIGGER trg_cancelled_purchases_delete_payment_term
        AFTER DELETE ON cancelled_purchases
        FOR EACH ROW EXECUTE PROCEDURE delete_posted_purchase_payment_term()",
];

const DOWN: &[&str] = &[
    "DROP TRIGGER IF EXISTS trg_cancelled_purchases_delete_payment_term ON cancelled_purchases",
    "DROP TRIGGER IF EXISTS trg_posted_purchases_delete_payment_term ON posted_purchases",
    "DROP TRIGGER IF EXISTS trg_draft_purchases_delete_payment_term ON draft_purchases",
    "DROP FUNCTION IF EXISTS delete_posted_purchase_payment_term()",
    "DROP FUNCTION IF EXISTS delete_draft_purchase_payment_term()",
    "DROP TABLE IF EXISTS purchase_preferences",
    "DROP TABLE IF EXISTS cancelled_purchase_line_taxes",
    "DROP TABLE IF EXISTS cancelled_purchase_lines",
    "DROP TABLE IF EXISTS cancelled_purchase_taxes",
    "DROP TABLE IF EXISTS cancelled_purchases",
    "DROP TABLE IF EXISTS posted_purchase_line_taxes",
    "DROP TABLE IF EXISTS posted_purchase_lines",
    "DROP TABLE IF EXISTS posted_purchase_taxes",
    "DROP TABLE IF EXISTS posted_purchases",
    "DROP TABLE IF EXISTS draft_purchase_line_taxes",
    "DROP TABLE IF EXISTS draft_purchase_lines",
    "DROP TABLE IF EXISTS draft_purchase_taxes",
    "DROP TABLE IF EXISTS draft_purchases",
    "DROP TABLE IF EXISTS posted_purchase_payment_term_lines",
    "DROP TABLE IF EXISTS posted_purchase_payment_terms",
    "DROP TABLE IF EXISTS draft_purchase_payment_term_lines",
    "DROP TABLE IF EXISTS draft_purchase_payment_terms",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in UP {
            exec_sql(manager, sql).await?;
        }
        let db = manager.get_connection();
        let backend = manager.get_database_backend();
        for (index, table, column) in [
            (
                "draft_purchases_number_trgm_idx",
                "draft_purchases",
                "number",
            ),
            (
                "draft_purchases_reference_trgm_idx",
                "draft_purchases",
                "reference",
            ),
            (
                "posted_purchases_number_trgm_idx",
                "posted_purchases",
                "number",
            ),
            (
                "posted_purchases_reference_trgm_idx",
                "posted_purchases",
                "reference",
            ),
        ] {
            trigram::create_gin_index(db, backend, index, table, column).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = manager.get_database_backend();
        for index in [
            "posted_purchases_reference_trgm_idx",
            "posted_purchases_number_trgm_idx",
            "draft_purchases_reference_trgm_idx",
            "draft_purchases_number_trgm_idx",
        ] {
            trigram::drop_gin_index(db, backend, index).await?;
        }
        for sql in DOWN {
            exec_sql(manager, sql).await?;
        }
        Ok(())
    }
}
