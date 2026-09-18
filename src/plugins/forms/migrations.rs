use sea_orm_migration::prelude::*;

use super::FormsTag;

mod m00001_create_forms;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m00001_create_forms::Migration)]
    }
}

crate::define_register_migrations! {
    plugin: FormsTag;
    migrator: Migrator;
}
