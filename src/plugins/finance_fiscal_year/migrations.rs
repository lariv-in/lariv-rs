use sea_orm_migration::prelude::*;

use super::FinanceFiscalYearTag;

mod m00001_create_fiscal_years;
mod m00002_fiscal_year_drop_deleted_at;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_fiscal_years::Migration),
            Box::new(m00002_fiscal_year_drop_deleted_at::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: FinanceFiscalYearTag;
    migrator: Migrator;
}
