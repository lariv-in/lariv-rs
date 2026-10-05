use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmLeadTags {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
}

#[derive(DeriveIden)]
enum PCrmLeadTags {
    Table,
    LeadId,
    LeadTagId,
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
                    .table(CrmLeadTags::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CrmLeadTags::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CrmLeadTags::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CrmLeadTags::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CrmLeadTags::Name).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_crm_lead_tags_name")
                    .table(CrmLeadTags::Table)
                    .col(CrmLeadTags::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PCrmLeadTags::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PCrmLeadTags::LeadId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PCrmLeadTags::LeadTagId)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(PCrmLeadTags::LeadId)
                            .col(PCrmLeadTags::LeadTagId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_p_crm_lead_tags_lead_id")
                            .from(PCrmLeadTags::Table, PCrmLeadTags::LeadId)
                            .to(CrmLeads::Table, CrmLeads::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_p_crm_lead_tags_lead_tag_id")
                            .from(PCrmLeadTags::Table, PCrmLeadTags::LeadTagId)
                            .to(CrmLeadTags::Table, CrmLeadTags::Id)
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
                    .name("idx_p_crm_lead_tags_lead_tag_id")
                    .table(PCrmLeadTags::Table)
                    .col(PCrmLeadTags::LeadTagId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PCrmLeadTags::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CrmLeadTags::Table).to_owned())
            .await
    }
}
