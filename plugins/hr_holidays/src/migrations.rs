//! Historical holiday migrations (`m00012_create_holidays`) stay in the HR hub.

use sea_orm_migration::prelude::*;

use super::HrHolidaysTag;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![]
    }
}

lariv_core::define_register_migrations! {
    plugin: HrHolidaysTag;
    migrator: Migrator;
}
