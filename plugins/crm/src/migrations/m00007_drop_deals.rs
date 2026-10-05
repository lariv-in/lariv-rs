use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmConvertedLeads {
    Table,
    DealId,
}

#[derive(DeriveIden)]
enum CrmDeals {
    Table,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_crm_converted_leads_deal_id")
                    .table(CrmConvertedLeads::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CrmConvertedLeads::Table)
                    .drop_column(CrmConvertedLeads::DealId)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(CrmDeals::Table).to_owned())
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Irreversible: deals table and converted_lead.deal_id are removed.
        Ok(())
    }
}
