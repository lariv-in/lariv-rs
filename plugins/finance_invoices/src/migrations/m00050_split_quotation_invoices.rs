//! Split quotations out of draft invoices.
//!
//! Copies every current draft invoice (lines, taxes, payment terms) into
//! parallel quotation tables, preserving ids, then deletes the draft rows.

use lariv_core::db::trigram;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_SQL: &str = r#"
CREATE TABLE quotation_payment_terms (LIKE draft_payment_terms INCLUDING DEFAULTS);
CREATE TABLE quotation_payment_term_lines (LIKE draft_payment_term_lines INCLUDING DEFAULTS);
CREATE TABLE quotation_invoices (LIKE draft_invoices INCLUDING DEFAULTS);
CREATE TABLE quotation_invoice_lines (LIKE draft_invoice_lines INCLUDING DEFAULTS);
CREATE TABLE quotation_invoice_taxes (LIKE draft_invoice_taxes INCLUDING DEFAULTS);
CREATE TABLE quotation_invoice_line_taxes (LIKE draft_invoice_line_taxes INCLUDING DEFAULTS);

ALTER TABLE quotation_invoices RENAME COLUMN draft_payment_term_id TO quotation_payment_term_id;
ALTER TABLE quotation_payment_term_lines RENAME COLUMN draft_payment_term_id TO quotation_payment_term_id;
ALTER TABLE quotation_invoice_lines RENAME COLUMN draft_invoice_id TO quotation_invoice_id;
ALTER TABLE quotation_invoice_taxes RENAME COLUMN draft_invoice_id TO quotation_invoice_id;
ALTER TABLE quotation_invoice_line_taxes RENAME COLUMN draft_invoice_line_id TO quotation_invoice_line_id;

CREATE SEQUENCE quotation_payment_terms_id_seq;
ALTER SEQUENCE quotation_payment_terms_id_seq OWNED BY quotation_payment_terms.id;
ALTER TABLE quotation_payment_terms ALTER COLUMN id SET DEFAULT nextval('quotation_payment_terms_id_seq');

CREATE SEQUENCE quotation_payment_term_lines_id_seq;
ALTER SEQUENCE quotation_payment_term_lines_id_seq OWNED BY quotation_payment_term_lines.id;
ALTER TABLE quotation_payment_term_lines ALTER COLUMN id SET DEFAULT nextval('quotation_payment_term_lines_id_seq');

CREATE SEQUENCE quotation_invoices_id_seq;
ALTER SEQUENCE quotation_invoices_id_seq OWNED BY quotation_invoices.id;
ALTER TABLE quotation_invoices ALTER COLUMN id SET DEFAULT nextval('quotation_invoices_id_seq');

CREATE SEQUENCE quotation_invoice_lines_id_seq;
ALTER SEQUENCE quotation_invoice_lines_id_seq OWNED BY quotation_invoice_lines.id;
ALTER TABLE quotation_invoice_lines ALTER COLUMN id SET DEFAULT nextval('quotation_invoice_lines_id_seq');

ALTER TABLE quotation_payment_terms ADD PRIMARY KEY (id);
ALTER TABLE quotation_payment_term_lines ADD PRIMARY KEY (id);
ALTER TABLE quotation_invoices ADD PRIMARY KEY (id);
ALTER TABLE quotation_invoice_lines ADD PRIMARY KEY (id);
ALTER TABLE quotation_invoice_taxes ADD PRIMARY KEY (quotation_invoice_id, tax_id);
ALTER TABLE quotation_invoice_line_taxes ADD PRIMARY KEY (quotation_invoice_line_id, tax_id);

INSERT INTO quotation_payment_terms SELECT * FROM draft_payment_terms;
INSERT INTO quotation_payment_term_lines SELECT * FROM draft_payment_term_lines;
INSERT INTO quotation_invoices SELECT * FROM draft_invoices;
INSERT INTO quotation_invoice_lines SELECT * FROM draft_invoice_lines;
INSERT INTO quotation_invoice_taxes SELECT * FROM draft_invoice_taxes;
INSERT INTO quotation_invoice_line_taxes SELECT * FROM draft_invoice_line_taxes;

SELECT setval('quotation_payment_terms_id_seq', COALESCE((SELECT MAX(id) FROM quotation_payment_terms), 1), (SELECT COUNT(*) > 0 FROM quotation_payment_terms));
SELECT setval('quotation_payment_term_lines_id_seq', COALESCE((SELECT MAX(id) FROM quotation_payment_term_lines), 1), (SELECT COUNT(*) > 0 FROM quotation_payment_term_lines));
SELECT setval('quotation_invoices_id_seq', COALESCE((SELECT MAX(id) FROM quotation_invoices), 1), (SELECT COUNT(*) > 0 FROM quotation_invoices));
SELECT setval('quotation_invoice_lines_id_seq', COALESCE((SELECT MAX(id) FROM quotation_invoice_lines), 1), (SELECT COUNT(*) > 0 FROM quotation_invoice_lines));

ALTER TABLE quotation_invoices
    ADD CONSTRAINT fk_quotation_invoices_customer
    FOREIGN KEY (customer_id) REFERENCES customers (id) ON DELETE RESTRICT;
ALTER TABLE quotation_invoices
    ADD CONSTRAINT fk_quotation_invoices_payment_term
    FOREIGN KEY (quotation_payment_term_id) REFERENCES quotation_payment_terms (id) ON DELETE SET NULL;
CREATE UNIQUE INDEX uq_quotation_invoices_quotation_payment_term_id
    ON quotation_invoices (quotation_payment_term_id);
CREATE INDEX idx_quotation_invoices_customer_id ON quotation_invoices (customer_id);
CREATE INDEX idx_quotation_invoices_datetime ON quotation_invoices (datetime);

ALTER TABLE quotation_invoice_lines
    ADD CONSTRAINT fk_quotation_invoice_lines_invoice
    FOREIGN KEY (quotation_invoice_id) REFERENCES quotation_invoices (id) ON DELETE CASCADE;
ALTER TABLE quotation_invoice_lines
    ADD CONSTRAINT fk_quotation_invoice_line_product
    FOREIGN KEY (product_id) REFERENCES products (id) ON DELETE RESTRICT;
CREATE INDEX idx_quotation_invoice_lines_invoice_id ON quotation_invoice_lines (quotation_invoice_id);

ALTER TABLE quotation_invoice_taxes
    ADD CONSTRAINT fk_quotation_invoice_taxes_invoice
    FOREIGN KEY (quotation_invoice_id) REFERENCES quotation_invoices (id) ON DELETE CASCADE;
ALTER TABLE quotation_invoice_taxes
    ADD CONSTRAINT fk_quotation_invoice_taxes_tax
    FOREIGN KEY (tax_id) REFERENCES taxes (id) ON DELETE RESTRICT;

ALTER TABLE quotation_invoice_line_taxes
    ADD CONSTRAINT fk_quotation_invoice_line_taxes_line
    FOREIGN KEY (quotation_invoice_line_id) REFERENCES quotation_invoice_lines (id) ON DELETE CASCADE;
ALTER TABLE quotation_invoice_line_taxes
    ADD CONSTRAINT fk_quotation_invoice_line_taxes_tax
    FOREIGN KEY (tax_id) REFERENCES taxes (id) ON DELETE RESTRICT;

ALTER TABLE quotation_payment_term_lines
    ADD CONSTRAINT fk_quotation_payment_term_lines_term
    FOREIGN KEY (quotation_payment_term_id) REFERENCES quotation_payment_terms (id) ON DELETE CASCADE;

CREATE OR REPLACE FUNCTION delete_quotation_payment_term_for_deleted_invoice()
RETURNS trigger AS $$
BEGIN
  IF OLD.quotation_payment_term_id IS NOT NULL THEN
    DELETE FROM quotation_payment_terms WHERE id = OLD.quotation_payment_term_id;
  END IF;
  RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_quotation_invoices_delete_payment_term
AFTER DELETE ON quotation_invoices
FOR EACH ROW EXECUTE PROCEDURE delete_quotation_payment_term_for_deleted_invoice();

DELETE FROM draft_invoices;
"#;

const DOWN_SQL: &str = r#"
INSERT INTO draft_payment_terms (id, created_at, updated_at)
SELECT id, created_at, updated_at FROM quotation_payment_terms
ON CONFLICT (id) DO NOTHING;

INSERT INTO draft_payment_term_lines (
    id, created_at, updated_at, draft_payment_term_id, line_order, date_kind,
    due_date, due_duration, amount_kind, amount, amount_percentage
)
SELECT
    id, created_at, updated_at, quotation_payment_term_id, line_order, date_kind,
    due_date, due_duration, amount_kind, amount, amount_percentage
FROM quotation_payment_term_lines
ON CONFLICT (id) DO NOTHING;

INSERT INTO draft_invoices (
    id, created_at, updated_at, number, reference, eway_bill_number, payment_reference,
    bank_account, remarks, datetime, delivery_date, customer_id, draft_payment_term_id
)
SELECT
    id, created_at, updated_at, number, reference, eway_bill_number, payment_reference,
    bank_account, remarks, datetime, delivery_date, customer_id, quotation_payment_term_id
FROM quotation_invoices
ON CONFLICT (id) DO NOTHING;

INSERT INTO draft_invoice_lines (
    id, created_at, updated_at, draft_invoice_id, product_id, product_name, product_remarks,
    hsn_code, sac_code, variable_types, sales_price_formula, rate, quantity, variable_values,
    pre_tax_amount, remarks
)
SELECT
    id, created_at, updated_at, quotation_invoice_id, product_id, product_name, product_remarks,
    hsn_code, sac_code, variable_types, sales_price_formula, rate, quantity, variable_values,
    pre_tax_amount, remarks
FROM quotation_invoice_lines
ON CONFLICT (id) DO NOTHING;

INSERT INTO draft_invoice_taxes (draft_invoice_id, tax_id)
SELECT quotation_invoice_id, tax_id FROM quotation_invoice_taxes
ON CONFLICT DO NOTHING;

INSERT INTO draft_invoice_line_taxes (draft_invoice_line_id, tax_id)
SELECT quotation_invoice_line_id, tax_id FROM quotation_invoice_line_taxes
ON CONFLICT DO NOTHING;

DROP TRIGGER IF EXISTS trg_quotation_invoices_delete_payment_term ON quotation_invoices;
DROP FUNCTION IF EXISTS delete_quotation_payment_term_for_deleted_invoice();
DROP TABLE IF EXISTS quotation_invoice_line_taxes;
DROP TABLE IF EXISTS quotation_invoice_taxes;
DROP TABLE IF EXISTS quotation_invoice_lines;
DROP TABLE IF EXISTS quotation_invoices;
DROP TABLE IF EXISTS quotation_payment_term_lines;
DROP TABLE IF EXISTS quotation_payment_terms;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(UP_SQL).await?;
        let backend = manager.get_database_backend();
        trigram::create_gin_index(
            db,
            backend,
            "quotation_invoices_number_trgm_idx",
            "quotation_invoices",
            "number",
        )
        .await?;
        trigram::create_gin_index(
            db,
            backend,
            "quotation_invoices_reference_trgm_idx",
            "quotation_invoices",
            "reference",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = manager.get_database_backend();
        trigram::drop_gin_index(db, backend, "quotation_invoices_number_trgm_idx").await?;
        trigram::drop_gin_index(db, backend, "quotation_invoices_reference_trgm_idx").await?;
        db.execute_unprepared(DOWN_SQL).await?;
        Ok(())
    }
}
