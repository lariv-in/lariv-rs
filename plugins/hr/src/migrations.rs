use sea_orm_migration::prelude::*;

use super::HrTag;

mod m00001_create_hr;
mod m00002_add_hr_updated_at;
mod m00003_standalone_hr_person_fields;
mod m00004_add_hr_user_id;
mod m00005_create_job_forms;
mod m00006_applicant_profile_fields;
mod m00007_replace_age_with_date_of_birth;
mod m00008_job_posting_composites;
mod m00009_merge_probation_into_employees;
mod m00010_employee_profile;
mod m00011_employee_profile_files;
mod m00012_create_holidays;
mod m00013_create_attendances;
mod m00014_employee_nullable_pay;
mod m00015_create_leaves;
mod m00016_employee_manager;

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
            Box::new(m00008_job_posting_composites::Migration),
            Box::new(m00009_merge_probation_into_employees::Migration),
            Box::new(m00010_employee_profile::Migration),
            Box::new(m00011_employee_profile_files::Migration),
            Box::new(m00012_create_holidays::Migration),
            Box::new(m00013_create_attendances::Migration),
            Box::new(m00014_employee_nullable_pay::Migration),
            Box::new(m00015_create_leaves::Migration),
            Box::new(m00016_employee_manager::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: HrTag;
    migrator: Migrator;
}
