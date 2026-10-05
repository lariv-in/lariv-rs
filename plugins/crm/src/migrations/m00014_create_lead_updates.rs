use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmLeadUpdates {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    LeadId,
    CreatedById,
    Datetime,
    Description,
}

#[derive(DeriveIden)]
enum CrmLeads {
    Table,
    Id,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CrmLeadUpdates::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CrmLeadUpdates::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CrmLeadUpdates::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CrmLeadUpdates::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(CrmLeadUpdates::LeadId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrmLeadUpdates::CreatedById)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrmLeadUpdates::Datetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrmLeadUpdates::Description)
                            .text()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crm_lead_updates_lead_id")
                            .from(CrmLeadUpdates::Table, CrmLeadUpdates::LeadId)
                            .to(CrmLeads::Table, CrmLeads::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crm_lead_updates_created_by_id")
                            .from(CrmLeadUpdates::Table, CrmLeadUpdates::CreatedById)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_crm_lead_updates_lead_id")
                    .table(CrmLeadUpdates::Table)
                    .col(CrmLeadUpdates::LeadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_crm_lead_updates_datetime")
                    .table(CrmLeadUpdates::Table)
                    .col(CrmLeadUpdates::Datetime)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CrmLeadUpdates::Table).to_owned())
            .await
    }
}
