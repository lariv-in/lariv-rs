use sea_orm_migration::prelude::*;

mod m00001_payment_customer_drop_invoice_and_batches;
mod m00002_payment_number;
mod m00003_gst_advance;

use super::FinancePaymentsTag;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_payment_customer_drop_invoice_and_batches::Migration),
            Box::new(m00002_payment_number::Migration),
            Box::new(m00003_gst_advance::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: FinancePaymentsTag;
    migrator: Migrator;
}
