use sea_orm_migration::prelude::*;

mod m00001_create_taxes;
mod m00002_seed_service_tax;
mod m00003_tax_type_account;
mod m00004_taxes_drop_deleted_at;
mod m00005_taxes_pg_trgm;
mod m00006_tax_journal_entry_item_templates;
mod m00007_drop_tax_journal_receivable;
mod m00008_default_tax_journal_entry_item_templates;

use super::FinanceTaxesTag;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_taxes::Migration),
            Box::new(m00002_seed_service_tax::Migration),
            Box::new(m00003_tax_type_account::Migration),
            Box::new(m00004_taxes_drop_deleted_at::Migration),
            Box::new(m00005_taxes_pg_trgm::Migration),
            Box::new(m00006_tax_journal_entry_item_templates::Migration),
            Box::new(m00007_drop_tax_journal_receivable::Migration),
            Box::new(m00008_default_tax_journal_entry_item_templates::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: FinanceTaxesTag;
    migrator: Migrator;
}
