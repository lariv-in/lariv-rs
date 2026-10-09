use sea_orm_migration::prelude::*;

use super::InventoryTag;

mod m00001_create_inventory;
mod m00002_movement_party;
mod m00003_movement_eway_bill;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_inventory::Migration),
            Box::new(m00002_movement_party::Migration),
            Box::new(m00003_movement_eway_bill::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: InventoryTag;
    migrator: Migrator;
}
