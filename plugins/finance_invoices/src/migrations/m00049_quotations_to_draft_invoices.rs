//! Copy legacy quotations into plain draft invoices.
//!
//! Leaves `quotations` / `quotation_lines` / `quotation_line_taxes` /
//! `quotation_preferences` in place but unused.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 15 days in nanoseconds — same default as `default_payment_term_lines_json`.
const FIFTEEN_DAYS_NANOS: i64 = 1_296_000_000_000_000;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = format!(
            r#"
DO $$
BEGIN
    IF to_regclass('public.quotations') IS NULL
       OR to_regclass('public.quotation_lines') IS NULL THEN
        RETURN;
    END IF;

    CREATE TEMP TABLE _q_draft_map (
        quotation_id bigint PRIMARY KEY,
        draft_id bigint NOT NULL,
        term_id bigint NOT NULL
    ) ON COMMIT DROP;

    INSERT INTO _q_draft_map (quotation_id, draft_id, term_id)
    SELECT
        q.id,
        nextval(pg_get_serial_sequence('draft_invoices', 'id')),
        nextval(pg_get_serial_sequence('draft_payment_terms', 'id'))
    FROM quotations AS q;

    IF NOT EXISTS (SELECT 1 FROM _q_draft_map) THEN
        RETURN;
    END IF;

    INSERT INTO draft_payment_terms (id, created_at, updated_at)
    SELECT term_id, COALESCE(q.created_at, now()), COALESCE(q.updated_at, now())
      FROM _q_draft_map AS m
      JOIN quotations AS q ON q.id = m.quotation_id;

    INSERT INTO draft_payment_term_lines (
        created_at, updated_at, draft_payment_term_id, line_order,
        date_kind, due_date, due_duration, amount_kind, amount, amount_percentage
    )
    SELECT
        now(),
        now(),
        m.term_id,
        0,
        'relative',
        NULL,
        {FIFTEEN_DAYS_NANOS},
        'relative',
        NULL,
        100
    FROM _q_draft_map AS m;

    INSERT INTO draft_invoices (
        id, created_at, updated_at, number, reference, eway_bill_number,
        payment_reference, bank_account, remarks, datetime, delivery_date,
        customer_id, draft_payment_term_id
    )
    SELECT
        m.draft_id,
        q.created_at,
        q.updated_at,
        NULLIF(btrim(q.number), ''),
        NULL,
        NULL,
        NULL,
        q.bank_account,
        NULLIF(btrim(q.remarks), ''),
        ((q.date + time '12:00:00') AT TIME ZONE 'UTC'),
        NULL,
        q.customer_id,
        m.term_id
    FROM quotations AS q
    JOIN _q_draft_map AS m ON m.quotation_id = q.id;

    CREATE TEMP TABLE _q_line_map (
        old_line_id bigint PRIMARY KEY,
        new_line_id bigint NOT NULL,
        draft_id bigint NOT NULL
    ) ON COMMIT DROP;

    INSERT INTO _q_line_map (old_line_id, new_line_id, draft_id)
    SELECT
        ql.id,
        nextval(pg_get_serial_sequence('draft_invoice_lines', 'id')),
        m.draft_id
    FROM quotation_lines AS ql
    JOIN _q_draft_map AS m ON m.quotation_id = ql.quotation_id;

    INSERT INTO draft_invoice_lines (
        id, created_at, updated_at, draft_invoice_id, product_id,
        product_name, product_remarks, hsn_code, sac_code,
        variable_types, sales_price_formula, rate, quantity,
        variable_values, pre_tax_amount, remarks
    )
    SELECT
        lm.new_line_id,
        ql.created_at,
        ql.updated_at,
        lm.draft_id,
        ql.product_id,
        COALESCE(p.name, format('Product #%s', ql.product_id::text)),
        p.remarks,
        CASE WHEN p.product_kind::text = 'Goods' THEN p.hsn_code ELSE NULL END,
        CASE WHEN p.product_kind::text = 'Services' THEN p.hsn_code ELSE NULL END,
        COALESCE(NULLIF(btrim(p.variables), ''), '{{}}'),
        COALESCE(NULLIF(btrim(p.sales_price_formula), ''), '0'),
        ql.rate,
        ql.quantity,
        ql.variable_values,
        ql.pre_tax_amount,
        ql.remarks
    FROM quotation_lines AS ql
    JOIN _q_line_map AS lm ON lm.old_line_id = ql.id
    LEFT JOIN products AS p ON p.id = ql.product_id;

    IF to_regclass('public.quotation_line_taxes') IS NOT NULL THEN
        INSERT INTO draft_invoice_line_taxes (draft_invoice_line_id, tax_id)
        SELECT lm.new_line_id, t.tax_id
          FROM quotation_line_taxes AS t
          JOIN _q_line_map AS lm ON lm.old_line_id = t.line_id
        ON CONFLICT DO NOTHING;
    END IF;
END $$;
"#
        );
        exec_sql(manager, &sql).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
