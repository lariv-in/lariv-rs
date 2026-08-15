use sea_orm_migration::prelude::*;

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
        ]
    }
}

crate::define_register_migrations! {
    plugin: FinanceProductsTag;
    migrator: Migrator;
}
