use sea_orm_migration::prelude::*;

use super::HrTag;

mod m00001_create_hr;
mod m00002_add_hr_updated_at;
mod m00003_standalone_hr_person_fields;
mod m00004_add_hr_user_id;
mod m00005_create_job_forms;
mod m00006_applicant_profile_fields;
mod m00007_replace_age_with_date_of_birth;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_hr::Migration),
            Box::new(m00002_add_hr_updated_at::Migration),
            Box::new(m00003_standalone_hr_person_fields::Migration),
            Box::new(m00004_add_hr_user_id::Migration),
            Box::new(m00005_create_job_forms::Migration),
            Box::new(m00006_applicant_profile_fields::Migration),
            Box::new(m00007_replace_age_with_date_of_birth::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: HrTag;
    migrator: Migrator;
}
