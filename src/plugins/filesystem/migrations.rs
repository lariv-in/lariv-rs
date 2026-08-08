use sea_orm_migration::prelude::*;

use super::FilesystemTag;

mod m20260730_000002_create_filesystem_nodes;
mod m20260808_000001_filesystem_drop_deleted_at;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260730_000002_create_filesystem_nodes::Migration),
            Box::new(m20260808_000001_filesystem_drop_deleted_at::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: FilesystemTag;
    migrator: Migrator;
}
