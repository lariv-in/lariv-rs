use sea_orm_migration::prelude::*;

use super::TodoTag;

mod m00001_create_todos;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m00001_create_todos::Migration)]
    }
}

lariv_core::define_register_migrations! {
    plugin: TodoTag;
    migrator: Migrator;
}
