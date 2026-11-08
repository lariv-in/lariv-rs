use sea_orm_migration::prelude::*;

use super::FilesystemSyncGitTag;

mod m20261108_000001_create_filesystem_git_syncs;
mod m20261108_000002_git_identifier_and_last_synced;
mod m20261108_000003_directory_primary_key;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261108_000001_create_filesystem_git_syncs::Migration),
            Box::new(m20261108_000002_git_identifier_and_last_synced::Migration),
            Box::new(m20261108_000003_directory_primary_key::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: FilesystemSyncGitTag;
    migrator: Migrator;
}
