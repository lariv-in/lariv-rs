use sea_orm_migration::prelude::*;

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

crate::define_register_migrations! {
    plugin: OtpTag;
    migrator: Migrator;
}
