use sea_orm_migration::prelude::*;

use super::FilesystemTag;

mod m20260730_000002_create_filesystem_nodes;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(
            m20260730_000002_create_filesystem_nodes::Migration,
        )]
    }
}

crate::define_register_migrations! {
    plugin: FilesystemTag;
    migrator: Migrator;
}
