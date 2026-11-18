use sea_orm_migration::prelude::*;

use super::CompaniesTag;

mod m00001_create_companies;
mod m00001_ensure_companies;
mod m00002_create_company_memberships;
mod m00003_create_company_user_prefs;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_companies::Migration),
            Box::new(m00001_ensure_companies::Migration),
            Box::new(m00002_create_company_memberships::Migration),
            Box::new(m00003_create_company_user_prefs::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: CompaniesTag;
    migrator: Migrator;
}
