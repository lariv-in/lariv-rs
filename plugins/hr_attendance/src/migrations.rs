//! Historical attendance migrations (`m00013_create_attendances`, `m00020_attendance_optional_end`,
//! `m00031_attendance_location`) stay in the HR hub.

use sea_orm_migration::prelude::*;

use super::HrAttendanceTag;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![]
    }
}

lariv_core::define_register_migrations! {
    plugin: HrAttendanceTag;
    migrator: Migrator;
}
