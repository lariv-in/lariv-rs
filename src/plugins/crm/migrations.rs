use sea_orm_migration::prelude::*;

use super::CrmTag;

mod m00001_create_crm;
mod m00002_rename_accounts_to_companies;
mod m00003_add_company_address_fields;
mod m00004_consolidate_company_address_columns;
mod m00005_optional_lead_source;
mod m00006_lead_contact_fk;
mod m00007_drop_deals;
mod m00008_drop_contact_title;
mod m00009_create_tasks;
mod m00010_task_completed_at;
mod m00011_completed_tasks;
mod m00012_contact_full_name;
mod m00013_drop_customer_fks;
mod m00014_create_lead_updates;

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
            Box::new(m00005_optional_lead_source::Migration),
            Box::new(m00006_lead_contact_fk::Migration),
            Box::new(m00007_drop_deals::Migration),
            Box::new(m00008_drop_contact_title::Migration),
            Box::new(m00009_create_tasks::Migration),
            Box::new(m00010_task_completed_at::Migration),
            Box::new(m00011_completed_tasks::Migration),
            Box::new(m00012_contact_full_name::Migration),
            Box::new(m00013_drop_customer_fks::Migration),
            Box::new(m00014_create_lead_updates::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: CrmTag;
    migrator: Migrator;
}
