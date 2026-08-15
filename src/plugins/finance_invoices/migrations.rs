use sea_orm_migration::prelude::*;

mod m00001_create_invoice_status_and_payment_terms;
mod m00002_create_invoices;
mod m00003_unique_invoice_numbers_posted_cancelled;
mod m00004_payment_term_relatives_duration;
mod m00005_payment_and_settlements;
mod m00006_partially_paid_prior_chain;
mod m00007_payment_taxes;
mod m00008_create_invoice_preferences;
mod m00009_drop_draft_invoice_gl_journal;
mod m00010_invoice_preferences_tax_payable;
mod m00011_create_payment_preferences;
mod m00012_invoice_reference_and_payment_fields;
mod m00013_payment_batches;
mod m00014_invoices_drop_deleted_at;
mod m00015_invoice_preferences_presentation;
mod m00016_recreate_accounting_preferences;
mod m00017_payment_terms_refactor;
mod m00018_invoice_pdf_vnode_assets;
mod m00019_invoice_company_presentation;
mod m00020_invoice_payment_term_fk;
mod m00021_delete_invoice_deletes_payment_term;
mod m00022_invoice_pg_trgm;
mod m00023_invoice_pg_trgm_lower;

use super::FinanceInvoicesTag;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_invoice_status_and_payment_terms::Migration),
            Box::new(m00002_create_invoices::Migration),
            Box::new(m00003_unique_invoice_numbers_posted_cancelled::Migration),
            Box::new(m00004_payment_term_relatives_duration::Migration),
            Box::new(m00005_payment_and_settlements::Migration),
            Box::new(m00006_partially_paid_prior_chain::Migration),
            Box::new(m00007_payment_taxes::Migration),
            Box::new(m00008_create_invoice_preferences::Migration),
            Box::new(m00009_drop_draft_invoice_gl_journal::Migration),
            Box::new(m00010_invoice_preferences_tax_payable::Migration),
            Box::new(m00011_create_payment_preferences::Migration),
            Box::new(m00012_invoice_reference_and_payment_fields::Migration),
            Box::new(m00013_payment_batches::Migration),
            Box::new(m00014_invoices_drop_deleted_at::Migration),
            Box::new(m00015_invoice_preferences_presentation::Migration),
            Box::new(m00016_recreate_accounting_preferences::Migration),
            Box::new(m00017_payment_terms_refactor::Migration),
            Box::new(m00018_invoice_pdf_vnode_assets::Migration),
            Box::new(m00019_invoice_company_presentation::Migration),
            Box::new(m00020_invoice_payment_term_fk::Migration),
            Box::new(m00021_delete_invoice_deletes_payment_term::Migration),
            Box::new(m00022_invoice_pg_trgm::Migration),
            Box::new(m00023_invoice_pg_trgm_lower::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: FinanceInvoicesTag;
    migrator: Migrator;
}
