use sea_orm_migration::prelude::*;

use super::TasksTag;

mod m00001_create_tasks;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m00001_create_tasks::Migration)]
    }
}

crate::define_register_migrations! {
    plugin: TasksTag;
    migrator: Migrator;
}
