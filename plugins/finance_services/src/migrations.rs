use sea_orm_migration::prelude::*;

mod m00001_create_services;

use super::FinanceServicesTag;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m00001_create_services::Migration)]
    }
}

lariv_core::define_register_migrations! {
    plugin: FinanceServicesTag;
    migrator: Migrator;
}
