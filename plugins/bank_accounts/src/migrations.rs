use sea_orm_migration::prelude::*;

use super::BankAccountsTag;

mod m00001_create_banks;
mod m00002_create_bank_accounts;
mod m00003_create_bank_directory_syncs;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_banks::Migration),
            Box::new(m00002_create_bank_accounts::Migration),
            Box::new(m00003_create_bank_directory_syncs::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: BankAccountsTag;
    migrator: Migrator;
}
