use sea_orm_migration::prelude::*;

use super::LlmAssistantTag;

mod m20260731_000002_create_llm_assistant_tables;
mod m20260808_000001_llm_assistant_drop_deleted_at;
mod m20260808_000002_create_llm_assistant_preferences;
mod m20260815_000001_llm_assistant_chat_model;
mod m20260815_000002_pg_trgm;
mod m20260815_000003_pg_trgm_lower;
mod m20260823_000001_llm_assistant_cse_prefs;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260731_000002_create_llm_assistant_tables::Migration),
            Box::new(m20260808_000001_llm_assistant_drop_deleted_at::Migration),
            Box::new(m20260808_000002_create_llm_assistant_preferences::Migration),
            Box::new(m20260815_000001_llm_assistant_chat_model::Migration),
            Box::new(m20260815_000002_pg_trgm::Migration),
            Box::new(m20260815_000003_pg_trgm_lower::Migration),
            Box::new(m20260823_000001_llm_assistant_cse_prefs::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: LlmAssistantTag;
    migrator: Migrator;
}
