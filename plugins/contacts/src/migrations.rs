use sea_orm_migration::prelude::*;

use super::ContactsTag;

mod m00001_create_companies;
mod m00002_create_contacts;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_companies::Migration),
            Box::new(m00002_create_contacts::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: ContactsTag;
    migrator: Migrator;
}
