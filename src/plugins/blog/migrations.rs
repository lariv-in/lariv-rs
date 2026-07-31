use sea_orm_migration::prelude::*;

use super::BlogTag;

mod m20260730_000001_create_blog_tables;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260730_000001_create_blog_tables::Migration)]
    }
}

crate::define_register_migrations! {
    plugin: BlogTag;
    migrator: Migrator;
}
