use sea_orm_migration::prelude::*;

use super::InventoryTag;

mod m00001_create_inventory;
mod m00002_movement_party;
mod m00003_movement_eway_bill;
mod m00004_split_movements;
mod m00005_movement_created_by;
mod m00006_movement_out_source;
mod m00007_movement_out_sources;
mod m00008_movement_customer;
mod m00009_vehicle_out_datetime;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_inventory::Migration),
            Box::new(m00002_movement_party::Migration),
            Box::new(m00003_movement_eway_bill::Migration),
            Box::new(m00004_split_movements::Migration),
            Box::new(m00005_movement_created_by::Migration),
            Box::new(m00006_movement_out_source::Migration),
            Box::new(m00007_movement_out_sources::Migration),
            Box::new(m00008_movement_customer::Migration),
            Box::new(m00009_vehicle_out_datetime::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: InventoryTag;
    migrator: Migrator;
}
