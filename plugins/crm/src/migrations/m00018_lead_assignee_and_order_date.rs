use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

const UP_POSTGRES: &str = r#"
DO $$
BEGIN
    IF to_regclass('public.crm_leads') IS NULL THEN
        RETURN;
    END IF;
    ALTER TABLE crm_leads ADD COLUMN IF NOT EXISTS assigned_to_id bigint;
    ALTER TABLE crm_leads ADD COLUMN IF NOT EXISTS order_expected_date date;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'fk_crm_leads_assigned_to_id'
    ) THEN
        ALTER TABLE crm_leads
            ADD CONSTRAINT fk_crm_leads_assigned_to_id
            FOREIGN KEY (assigned_to_id) REFERENCES users(id)
            ON DELETE SET NULL ON UPDATE CASCADE;
    END IF;
    CREATE INDEX IF NOT EXISTS idx_crm_leads_assigned_to_id ON crm_leads (assigned_to_id);
END $$;
"#;

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
        if manager.get_connection().get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            return exec_sql(manager, UP_POSTGRES).await;
        }

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
