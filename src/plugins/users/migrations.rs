use sea_orm_migration::prelude::*;

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

crate::define_register_migrations! {
    plugin: UsersTag;
    migrator: Migrator;
}
