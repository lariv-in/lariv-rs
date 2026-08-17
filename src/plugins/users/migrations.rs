use sea_orm_migration::prelude::*;

use super::UsersTag;

mod m20240729_000001_create_users_roles;
mod m20260808_000001_users_drop_deleted_at;
mod m20260817_000001_users_phone_default;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240729_000001_create_users_roles::Migration),
            Box::new(m20260808_000001_users_drop_deleted_at::Migration),
            Box::new(m20260817_000001_users_phone_default::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: UsersTag;
    migrator: Migrator;
}
