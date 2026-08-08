use sea_orm_migration::prelude::*;

use super::WebsiteTag;

mod m20260731_000001_create_db_routes;
mod m20260808_000001_website_drop_deleted_at;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260731_000001_create_db_routes::Migration),
            Box::new(m20260808_000001_website_drop_deleted_at::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: WebsiteTag;
    migrator: Migrator;
}
