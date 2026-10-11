use sea_orm_migration::prelude::*;

use super::TasksTag;

mod m00001_create_tasks;
mod m00002_task_parent;
mod m00003_periodic_tasks;
mod m00004_push_subscriptions;
mod m00005_vapid_keys;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_tasks::Migration),
            Box::new(m00002_task_parent::Migration),
            Box::new(m00003_periodic_tasks::Migration),
            Box::new(m00004_push_subscriptions::Migration),
            Box::new(m00005_vapid_keys::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: TasksTag;
    migrator: Migrator;
}
