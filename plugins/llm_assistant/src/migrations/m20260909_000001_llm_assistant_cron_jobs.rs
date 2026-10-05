use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum LlmAssistantSessions {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum LlmAssistantCronJobs {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Duration,
    Prompt,
}

#[derive(DeriveIden)]
enum LlmAssistantCronJobRuns {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    CronJobId,
    Datetime,
    SessionId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LlmAssistantCronJobs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LlmAssistantCronJobs::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(LlmAssistantCronJobs::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(LlmAssistantCronJobs::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(LlmAssistantCronJobs::Duration)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantCronJobs::Prompt)
                            .text()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(LlmAssistantCronJobRuns::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LlmAssistantCronJobRuns::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantCronJobRuns::CreatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantCronJobRuns::UpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantCronJobRuns::CronJobId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmAssistantCronJobRuns::Datetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(LlmAssistantCronJobRuns::SessionId).big_integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_llm_assistant_cron_job_runs_cron_job_id")
                            .from(
                                LlmAssistantCronJobRuns::Table,
                                LlmAssistantCronJobRuns::CronJobId,
                            )
                            .to(LlmAssistantCronJobs::Table, LlmAssistantCronJobs::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_llm_assistant_cron_job_runs_session_id")
                            .from(
                                LlmAssistantCronJobRuns::Table,
                                LlmAssistantCronJobRuns::SessionId,
                            )
                            .to(LlmAssistantSessions::Table, LlmAssistantSessions::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_llm_assistant_cron_job_runs_cron_job_datetime")
                    .table(LlmAssistantCronJobRuns::Table)
                    .col(LlmAssistantCronJobRuns::CronJobId)
                    .col(LlmAssistantCronJobRuns::Datetime)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(LlmAssistantCronJobRuns::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(LlmAssistantCronJobs::Table).to_owned())
            .await
    }
}
