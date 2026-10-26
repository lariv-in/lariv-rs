use sea_orm_migration::prelude::*;

mod m00001_create_quotations;

use super::FinanceQuotationsTag;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m00001_create_quotations::Migration)]
    }
}

lariv_core::define_register_migrations! {
    plugin: FinanceQuotationsTag;
    migrator: Migrator;
}
