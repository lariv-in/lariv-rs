use sea_orm_migration::prelude::*;

mod m00001_create_purchases;
mod m00002_formula_measure_values;
mod m00003_formula_fixed_units;
mod m00004_decimal_unit_none;
mod m00005_goods_and_service_lines;
mod m00006_purchase_customer_fk;
mod m00007_purchase_products_and_services;
mod m00008_drop_purchase_gl_preferences;
mod m00009_default_purchase_journal_entry_item_templates;
mod m00010_purchase_vendors_from_customers;

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
            Box::new(m00005_goods_and_service_lines::Migration),
            Box::new(m00006_purchase_customer_fk::Migration),
            // Databases that already stored `customer_id` copy those rows onto
            // `purchase_vendors` and rename the column. Fresh installs already
            // have `vendor_id` and skip the copy.
            Box::new(m00010_purchase_vendors_from_customers::Migration),
            Box::new(m00007_purchase_products_and_services::Migration),
            // Seed from purchase preference accounts while those columns still
            // exist. A blank preference uses the accounts on the latest posted
            // purchase. Databases that already dropped the columns fall back
            // to the chart accounts the old poster used.
            Box::new(m00009_default_purchase_journal_entry_item_templates::Migration),
            Box::new(m00008_drop_purchase_gl_preferences::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: FinancePurchasesTag;
    migrator: Migrator;
}
