//! Historical overtime migrations (`m00022_create_overtime`) stay in the HR hub.

use sea_orm_migration::prelude::*;

use super::HrOvertimeTag;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![]
    }
}

lariv_core::define_register_migrations! {
    plugin: HrOvertimeTag;
    migrator: Migrator;
}
