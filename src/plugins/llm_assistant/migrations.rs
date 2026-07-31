use frunk::hlist::HList;
use sea_orm_migration::prelude::*;

use crate::migration::{CollectMigrations, MigrationCapability, RegisterMigrations};

use super::LlmAssistantTag;

mod m20260731_000002_create_llm_assistant_tables;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(
            m20260731_000002_create_llm_assistant_tables::Migration,
        )]
    }
}

impl<M> RegisterMigrations<LlmAssistantTag> for MigrationCapability<M>
where
    M: HList + Clone + CollectMigrations + Send,
{
    type Output = MigrationCapability<impl HList + CollectMigrations + Clone + Send>;

    fn register_migrations(self) -> Self::Output {
        self.prepend::<LlmAssistantTag, _>(Migrator)
    }
}
