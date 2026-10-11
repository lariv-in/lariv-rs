use sea_orm_migration::prelude::*;

use super::TasksTag;

mod m00001_create_tasks;
mod m00002_task_parent;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_tasks::Migration),
            Box::new(m00002_task_parent::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: TasksTag;
    migrator: Migrator;
}
