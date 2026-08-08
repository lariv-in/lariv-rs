use sea_orm_migration::prelude::*;

use super::LlmAssistantTag;

mod m20260731_000002_create_llm_assistant_tables;
mod m20260808_000001_llm_assistant_drop_deleted_at;
mod m20260808_000002_create_llm_assistant_preferences;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260731_000002_create_llm_assistant_tables::Migration),
            Box::new(m20260808_000001_llm_assistant_drop_deleted_at::Migration),
            Box::new(m20260808_000002_create_llm_assistant_preferences::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: LlmAssistantTag;
    migrator: Migrator;
}
