use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmLeads {
    Table,
    AssignedToId,
    OrderExpectedDate,
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
            .alter_table(
                Table::alter()
                    .table(CrmLeads::Table)
                    .add_column(ColumnDef::new(CrmLeads::AssignedToId).big_integer())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CrmLeads::Table)
                    .add_column(ColumnDef::new(CrmLeads::OrderExpectedDate).date())
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_crm_leads_assigned_to_id")
                    .from(CrmLeads::Table, CrmLeads::AssignedToId)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_crm_leads_assigned_to_id")
                    .table(CrmLeads::Table)
                    .col(CrmLeads::AssignedToId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_crm_leads_assigned_to_id")
                    .table(CrmLeads::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_crm_leads_assigned_to_id")
                    .table(CrmLeads::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CrmLeads::Table)
                    .drop_column(CrmLeads::OrderExpectedDate)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CrmLeads::Table)
                    .drop_column(CrmLeads::AssignedToId)
                    .to_owned(),
            )
            .await
    }
}
