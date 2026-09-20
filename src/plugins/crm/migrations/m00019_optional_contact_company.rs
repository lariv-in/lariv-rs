use crate::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmContacts {
    Table,
    CompanyId,
}


#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_connection().get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                exec_sql(manager,
                    "ALTER TABLE crm_contacts DROP CONSTRAINT IF EXISTS fk_crm_contacts_company_id",
                )
                .await?;
                exec_sql(manager,
                    "ALTER TABLE crm_contacts ALTER COLUMN company_id DROP NOT NULL",
                )
                .await?;
                exec_sql(manager,
                    "ALTER TABLE crm_contacts ADD CONSTRAINT fk_crm_contacts_company_id \
                     FOREIGN KEY (company_id) REFERENCES crm_companies(id) \
                     ON DELETE SET NULL ON UPDATE CASCADE",
                )
                .await
            }
            sea_orm::DatabaseBackend::Sqlite => {
                manager
                    .alter_table(
                        Table::alter()
                            .table(CrmContacts::Table)
                            .modify_column(
                                ColumnDef::new(CrmContacts::CompanyId).big_integer().null(),
                            )
                            .to_owned(),
                    )
                    .await
            }
            _ => Ok(()),
        }
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_connection().get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                exec_sql(manager,
                    "UPDATE crm_contacts SET company_id = (
                         SELECT id FROM crm_companies ORDER BY id LIMIT 1
                     ) WHERE company_id IS NULL",
                )
                .await?;
                exec_sql(manager,
                    "ALTER TABLE crm_contacts DROP CONSTRAINT IF EXISTS fk_crm_contacts_company_id",
                )
                .await?;
                exec_sql(manager,
                    "ALTER TABLE crm_contacts ALTER COLUMN company_id SET NOT NULL",
                )
                .await?;
                exec_sql(manager,
                    "ALTER TABLE crm_contacts ADD CONSTRAINT fk_crm_contacts_company_id \
                     FOREIGN KEY (company_id) REFERENCES crm_companies(id) \
                     ON DELETE CASCADE ON UPDATE CASCADE",
                )
                .await
            }
            sea_orm::DatabaseBackend::Sqlite => {
                manager
                    .alter_table(
                        Table::alter()
                            .table(CrmContacts::Table)
                            .modify_column(
                                ColumnDef::new(CrmContacts::CompanyId)
                                    .big_integer()
                                    .not_null(),
                            )
                            .to_owned(),
                    )
                    .await
            }
            _ => Ok(()),
        }
    }
}
