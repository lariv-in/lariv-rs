use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum LlmAssistantPreferences {
    Table,
    AdversarialModel,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager
            .has_column("llm_assistant_preferences", "adversarial_model")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(LlmAssistantPreferences::Table)
                        .drop_column(LlmAssistantPreferences::AdversarialModel)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager
            .has_column("llm_assistant_preferences", "adversarial_model")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(LlmAssistantPreferences::Table)
                        .add_column(
                            ColumnDef::new(LlmAssistantPreferences::AdversarialModel)
                                .text()
                                .not_null()
                                .default(""),
                        )
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}
