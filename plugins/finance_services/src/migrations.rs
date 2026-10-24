use sea_orm_migration::prelude::*;

mod m00001_create_services;
mod m00002_create_service_journal_entry_item_templates;
mod m00003_default_service_journal_entry_item_templates;
mod m00004_journal_formula_unit_rate;

use super::FinanceServicesTag;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_services::Migration),
            Box::new(m00002_create_service_journal_entry_item_templates::Migration),
            Box::new(m00003_default_service_journal_entry_item_templates::Migration),
            Box::new(m00004_journal_formula_unit_rate::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: FinanceServicesTag;
    migrator: Migrator;
}
