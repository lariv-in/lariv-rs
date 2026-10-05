use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmLeadTimelines {
    Table,
    Id,
    CreatedAt,
    Content,
    LeadId,
}

#[derive(DeriveIden)]
enum CrmLeads {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CrmLeadTimelines::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CrmLeadTimelines::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CrmLeadTimelines::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CrmLeadTimelines::Content).text().not_null())
                    .col(
                        ColumnDef::new(CrmLeadTimelines::LeadId)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crm_lead_timelines_lead_id")
                            .from(CrmLeadTimelines::Table, CrmLeadTimelines::LeadId)
                            .to(CrmLeads::Table, CrmLeads::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_crm_lead_timelines_lead_id")
                    .table(CrmLeadTimelines::Table)
                    .col(CrmLeadTimelines::LeadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_crm_lead_timelines_created_at")
                    .table(CrmLeadTimelines::Table)
                    .col(CrmLeadTimelines::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CrmLeadTimelines::Table).to_owned())
            .await
    }
}
