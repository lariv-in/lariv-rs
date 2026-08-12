use sea_orm_migration::prelude::*;

use super::CrmTag;

mod m00001_create_crm;
mod m00002_rename_accounts_to_companies;
mod m00003_add_company_address_fields;
mod m00004_consolidate_company_address_columns;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_crm::Migration),
            Box::new(m00002_rename_accounts_to_companies::Migration),
            Box::new(m00003_add_company_address_fields::Migration),
            Box::new(m00004_consolidate_company_address_columns::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: CrmTag;
    migrator: Migrator;
}
