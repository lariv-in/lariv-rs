//! Per-invoice GST return tables are unused while filing is Excel-only.
use lariv_core::db::migration_sql::exec_sql;

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in TABLES {
            exec_sql(manager, &format!("DROP TABLE IF EXISTS {table}")).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (table, id_col, parent) in TABLES_WITH_PARENT {
            exec_sql(
                manager,
                &format!(
                    "CREATE TABLE IF NOT EXISTS {table} ( \
                        {id_col} bigint PRIMARY KEY REFERENCES {parent}(id) ON DELETE CASCADE, \
                        gst_section text NOT NULL DEFAULT 'regular', \
                        gst_inv_typ text NOT NULL DEFAULT 'R', \
                        reverse_charge boolean NOT NULL DEFAULT false, \
                        etin text, \
                        supplier_gstin text, \
                        exp_typ text, \
                        shipping_bill_number text, \
                        shipping_bill_date date, \
                        port_code text, \
                        amendment_of_number text, \
                        amendment_of_date date \
                    )"
                ),
            )
            .await?;
        }
        Ok(())
    }
}

const TABLES: &[&str] = &[
    "draft_invoice_gst_details",
    "posted_invoice_gst_details",
    "cancelled_invoice_gst_details",
    "quotation_invoice_gst_details",
];

const TABLES_WITH_PARENT: &[(&str, &str, &str)] = &[
    (
        "draft_invoice_gst_details",
        "draft_invoice_id",
        "draft_invoices",
    ),
    (
        "posted_invoice_gst_details",
        "posted_invoice_id",
        "posted_invoices",
    ),
    (
        "cancelled_invoice_gst_details",
        "cancelled_invoice_id",
        "cancelled_invoices",
    ),
    (
        "quotation_invoice_gst_details",
        "quotation_invoice_id",
        "quotation_invoices",
    ),
];
