use sea_orm_migration::prelude::*;

pub mod formula_measure_sql;
mod m00001_create_products;
mod m00002_product_gl_accounts;
mod m00003_product_type_reference_remarks;
mod m00004_drop_product_input_tax_account;
mod m00005_create_product_preferences;
mod m00006_drop_product_gl_accounts;
mod m00007_product_preferences_taxes;
mod m00008_optional_product_reference;
mod m00009_products_drop_deleted_at;
mod m00010_product_pg_trgm;
mod m00011_product_pg_trgm_lower;
mod m00012_product_price_formula;
mod m00013_product_description;
mod m00014_product_price_formulas;
mod m00015_drop_product_description;
mod m00016_formula_measure;
mod m00017_formula_fixed_units;
mod m00018_decimal_unit_none;
mod m00019_product_stock;
mod m00020_product_journal_entry_templates;
mod m00021_product_journal_entry_item_templates;
mod m00022_drop_product_journal_receivable;
mod m00023_default_product_journal_entry_item_templates;
pub mod product_catalog_cleanup;

use super::FinanceProductsTag;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_products::Migration),
            Box::new(m00002_product_gl_accounts::Migration),
            Box::new(m00003_product_type_reference_remarks::Migration),
            Box::new(m00004_drop_product_input_tax_account::Migration),
            Box::new(m00005_create_product_preferences::Migration),
            Box::new(m00006_drop_product_gl_accounts::Migration),
            Box::new(m00007_product_preferences_taxes::Migration),
            Box::new(m00008_optional_product_reference::Migration),
            Box::new(m00009_products_drop_deleted_at::Migration),
            Box::new(m00010_product_pg_trgm::Migration),
            Box::new(m00011_product_pg_trgm_lower::Migration),
            Box::new(m00012_product_price_formula::Migration),
            Box::new(m00013_product_description::Migration),
            Box::new(m00014_product_price_formulas::Migration),
            Box::new(m00015_drop_product_description::Migration),
            Box::new(m00016_formula_measure::Migration),
            Box::new(m00017_formula_fixed_units::Migration),
            Box::new(m00018_decimal_unit_none::Migration),
            Box::new(m00019_product_stock::Migration),
            Box::new(m00020_product_journal_entry_templates::Migration),
            Box::new(m00021_product_journal_entry_item_templates::Migration),
            Box::new(m00022_drop_product_journal_receivable::Migration),
            Box::new(m00023_default_product_journal_entry_item_templates::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: FinanceProductsTag;
    migrator: Migrator;
}
