use frunk::hlist::HList;
use sea_orm_migration::prelude::*;

use crate::migration::{CollectMigrations, MigrationCapability, RegisterMigrations};

use super::UsersTag;

mod m20240729_000001_create_users_roles;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20240729_000001_create_users_roles::Migration)]
    }
}

impl<M> RegisterMigrations<UsersTag> for MigrationCapability<M>
where
    M: HList + Clone + CollectMigrations + Send,
{
    type Output = MigrationCapability<impl HList + CollectMigrations + Clone + Send>;

    fn register_migrations(self) -> Self::Output {
        self.prepend::<UsersTag, _>(Migrator)
    }
}
