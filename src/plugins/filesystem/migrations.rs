use frunk::hlist::HList;
use sea_orm_migration::prelude::*;

use crate::migration::{CollectMigrations, MigrationCapability, RegisterMigrations};

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

impl<M> RegisterMigrations<FilesystemTag> for MigrationCapability<M>
where
    M: HList + Clone + CollectMigrations + Send,
{
    type Output = MigrationCapability<impl HList + CollectMigrations + Clone + Send>;

    fn register_migrations(self) -> Self::Output {
        self.prepend::<FilesystemTag, _>(Migrator)
    }
}
