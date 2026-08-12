use sea_orm_migration::prelude::*;

use super::FinanceIndianTag;

mod m00001_indian_gst_accounts_and_taxes;
mod m00002_default_preferences;
mod m00003_default_ledger;
mod m00004_default_product_taxes;
mod m00005_remove_default_igst_product_tax;
mod m00006_default_accounting_currency;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_indian_gst_accounts_and_taxes::Migration),
            Box::new(m00002_default_preferences::Migration),
            Box::new(m00003_default_ledger::Migration),
            Box::new(m00004_default_product_taxes::Migration),
            Box::new(m00005_remove_default_igst_product_tax::Migration),
            Box::new(m00006_default_accounting_currency::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: FinanceIndianTag;
    migrator: Migrator;
}
