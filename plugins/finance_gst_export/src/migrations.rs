use sea_orm_migration::prelude::*;

mod m00001_gst_tax_components;
mod m00002_invoice_gst_details;

use super::FinanceGstExportTag;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_gst_tax_components::Migration),
            Box::new(m00002_invoice_gst_details::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: FinanceGstExportTag;
    migrator: Migrator;
}
