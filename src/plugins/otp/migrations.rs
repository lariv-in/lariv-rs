use frunk::hlist::HList;
use sea_orm_migration::prelude::*;

use crate::migration::{CollectMigrations, MigrationCapability, RegisterMigrations};

use super::OtpTag;

mod m20260730_000003_create_otp_preferences;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(
            m20260730_000003_create_otp_preferences::Migration,
        )]
    }
}

impl<M> RegisterMigrations<OtpTag> for MigrationCapability<M>
where
    M: HList + Clone + CollectMigrations + Send,
{
    type Output = MigrationCapability<impl HList + CollectMigrations + Clone + Send>;

    fn register_migrations(self) -> Self::Output {
        self.prepend::<OtpTag, _>(Migrator)
    }
}
