use sea_orm_migration::prelude::*;

use super::MeetsTag;

mod m00001_create_meets;
mod m00002_room_meeting_times;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_meets::Migration),
            Box::new(m00002_room_meeting_times::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: MeetsTag;
    migrator: Migrator;
}
