use super::HrJobPostingTag;
use sea_orm_migration::prelude::*;
#[derive(Clone, Copy, Default)]
pub struct Migrator;
#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![]
    }
}
lariv_core::define_register_migrations! { plugin: HrJobPostingTag; migrator: Migrator; }
