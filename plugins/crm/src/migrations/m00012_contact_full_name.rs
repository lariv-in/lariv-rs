use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmContacts {
    Table,
    FirstName,
    LastName,
    Name,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_connection().get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            return exec_sql(
                manager,
                r#"
                DO $$
                BEGIN
                    IF EXISTS (
                        SELECT 1 FROM information_schema.columns
                        WHERE table_schema = 'public'
                          AND table_name = 'crm_contacts'
                          AND column_name = 'first_name'
                    ) AND NOT EXISTS (
                        SELECT 1 FROM information_schema.columns
                        WHERE table_schema = 'public'
                          AND table_name = 'crm_contacts'
                          AND column_name = 'name'
                    ) THEN
                        UPDATE crm_contacts
                        SET first_name = TRIM(first_name || ' ' || COALESCE(last_name, ''));
                        ALTER TABLE crm_contacts RENAME COLUMN first_name TO name;
                    END IF;
                    IF EXISTS (
                        SELECT 1 FROM information_schema.columns
                        WHERE table_schema = 'public'
                          AND table_name = 'crm_contacts'
                          AND column_name = 'last_name'
                    ) THEN
                        ALTER TABLE crm_contacts DROP COLUMN last_name;
                    END IF;
                END $$;
                "#,
            )
            .await;
        }

        exec_sql(
            manager,
            r#"
                UPDATE crm_contacts
                SET first_name = TRIM(first_name || ' ' || COALESCE(last_name, ''))
                "#,
        )
        .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CrmContacts::Table)
                    .rename_column(CrmContacts::FirstName, CrmContacts::Name)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CrmContacts::Table)
                    .drop_column(CrmContacts::LastName)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CrmContacts::Table)
                    .rename_column(CrmContacts::Name, CrmContacts::FirstName)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CrmContacts::Table)
                    .add_column(ColumnDef::new(CrmContacts::LastName).text())
                    .to_owned(),
            )
            .await
    }
}
