use sea_orm_migration::prelude::*;

mod m00001_create_purchases;
mod m00002_formula_measure_values;
mod m00003_formula_fixed_units;
mod m00004_decimal_unit_none;

use super::FinancePurchasesTag;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_purchases::Migration),
            Box::new(m00002_formula_measure_values::Migration),
            Box::new(m00003_formula_fixed_units::Migration),
            Box::new(m00004_decimal_unit_none::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: FinancePurchasesTag;
    migrator: Migrator;
}
