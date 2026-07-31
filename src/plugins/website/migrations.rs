use frunk::hlist::HList;
use sea_orm_migration::prelude::*;

use crate::migration::{CollectMigrations, MigrationCapability, RegisterMigrations};

use super::WebsiteTag;

mod m20260731_000001_create_db_routes;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260731_000001_create_db_routes::Migration)]
    }
}

impl<M> RegisterMigrations<WebsiteTag> for MigrationCapability<M>
where
    M: HList + Clone + CollectMigrations + Send,
{
    type Output = MigrationCapability<impl HList + CollectMigrations + Clone + Send>;

    fn register_migrations(self) -> Self::Output {
        self.prepend::<WebsiteTag, _>(Migrator)
    }
}
