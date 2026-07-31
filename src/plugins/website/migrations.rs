use sea_orm_migration::prelude::*;

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

crate::define_register_migrations! {
    plugin: WebsiteTag;
    migrator: Migrator;
}
