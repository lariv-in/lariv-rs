use sea_orm_migration::prelude::*;

use super::DocumentsTag;

mod m00001_create_documents;
mod m00002_create_document_preferences;
mod m00003_pan_and_passport;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_documents::Migration),
            Box::new(m00002_create_document_preferences::Migration),
            Box::new(m00003_pan_and_passport::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: DocumentsTag;
    migrator: Migrator;
}
