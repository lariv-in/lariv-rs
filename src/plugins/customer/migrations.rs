use sea_orm_migration::prelude::*;

use super::CustomerTag;

mod m00001_create_customers;
mod m00002_split_customer_address;
mod m00003_add_customer_type;
mod m00004_customer_drop_deleted_at;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_customers::Migration),
            Box::new(m00002_split_customer_address::Migration),
            Box::new(m00003_add_customer_type::Migration),
            Box::new(m00004_customer_drop_deleted_at::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: CustomerTag;
    migrator: Migrator;
}
