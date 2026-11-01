use sea_orm_migration::prelude::*;

mod m00001_create_quotations;
mod m00002_unify_service_catalog;
mod m00003_unify_document_lines;

use super::FinanceQuotationsTag;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_quotations::Migration),
            Box::new(m00002_unify_service_catalog::Migration),
            Box::new(m00003_unify_document_lines::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: FinanceQuotationsTag;
    migrator: Migrator;
}
