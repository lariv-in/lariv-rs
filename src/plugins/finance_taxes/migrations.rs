use sea_orm_migration::prelude::*;

mod m00001_create_taxes;
mod m00002_seed_service_tax;
mod m00003_tax_type_account;
mod m00004_taxes_drop_deleted_at;
mod m00005_taxes_pg_trgm;

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
        ]
    }
}

crate::define_register_migrations! {
    plugin: FinanceTaxesTag;
    migrator: Migrator;
}
