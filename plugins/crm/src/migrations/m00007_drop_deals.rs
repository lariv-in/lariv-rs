use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

const UP_POSTGRES: &str = r#"
DO $$
BEGIN
    IF to_regclass('public.crm_converted_leads') IS NOT NULL THEN
        ALTER TABLE crm_converted_leads DROP CONSTRAINT IF EXISTS fk_crm_converted_leads_deal_id;
        ALTER TABLE crm_converted_leads DROP COLUMN IF EXISTS deal_id;
    END IF;
    DROP TABLE IF EXISTS crm_deals;
END $$;
"#;

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
        if manager.get_connection().get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            return exec_sql(manager, UP_POSTGRES).await;
        }

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
