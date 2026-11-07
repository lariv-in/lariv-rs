use lariv_core::db::migration_sql::exec_sql;
use sea_orm::Statement;
use sea_orm_migration::prelude::*;

/// Databases that already store `contact_id` keep their leads. A fresh column
/// still clears the old denormalized rows, which have no contact to point at.
const UP_POSTGRES: &str = r#"
DO $$
BEGIN
    IF to_regclass('public.crm_leads') IS NULL THEN
        RETURN;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = 'crm_leads' AND column_name = 'contact_id'
    ) THEN
        IF to_regclass('public.crm_converted_leads') IS NOT NULL THEN
            DELETE FROM crm_converted_leads;
        END IF;
        IF to_regclass('public.crm_failed_leads') IS NOT NULL THEN
            DELETE FROM crm_failed_leads;
        END IF;
        DELETE FROM crm_leads;
        ALTER TABLE crm_leads ADD COLUMN contact_id bigint;
        ALTER TABLE crm_leads ALTER COLUMN contact_id SET NOT NULL;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_crm_leads_contact_id') THEN
        ALTER TABLE crm_leads
            ADD CONSTRAINT fk_crm_leads_contact_id
            FOREIGN KEY (contact_id) REFERENCES crm_contacts(id)
            ON DELETE RESTRICT ON UPDATE CASCADE;
    END IF;

    ALTER TABLE crm_leads DROP COLUMN IF EXISTS company_name;
    ALTER TABLE crm_leads DROP COLUMN IF EXISTS first_name;
    ALTER TABLE crm_leads DROP COLUMN IF EXISTS last_name;
    ALTER TABLE crm_leads DROP COLUMN IF EXISTS email;
    ALTER TABLE crm_leads DROP COLUMN IF EXISTS phone;
END $$;
"#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmLeads {
    Table,
    ContactId,
    CompanyName,
    FirstName,
    LastName,
    Email,
    Phone,
}

#[derive(DeriveIden)]
enum CrmConvertedLeads {
    Table,
}

#[derive(DeriveIden)]
enum CrmFailedLeads {
    Table,
}

/// Existing lead rows stored denormalized person/company text. Those columns are
/// removed in favour of `contact_id`, so prior lead rows (and convert/fail links)
/// are cleared — there is no lossless mapping without inventing contacts.
async fn clear_leads(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let backend = manager.get_connection().get_database_backend();
    for table in [
        CrmConvertedLeads::Table.to_string(),
        CrmFailedLeads::Table.to_string(),
        CrmLeads::Table.to_string(),
    ] {
        manager
            .get_connection()
            .execute_raw(Statement::from_string(
                backend,
                format!("DELETE FROM {table}"),
            ))
            .await?;
    }
    Ok(())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_connection().get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            return exec_sql(manager, UP_POSTGRES).await;
        }

        clear_leads(manager).await?;

        // SQLite cannot ALTER a column to NOT NULL, and cannot add a foreign key
        // after the table exists. The table was just cleared, so a default satisfies
        // NOT NULL until a real contact id is written.
        exec_sql(
            manager,
            "ALTER TABLE crm_leads ADD COLUMN contact_id integer NOT NULL DEFAULT 0",
        )
        .await?;

        for col in [
            CrmLeads::CompanyName,
            CrmLeads::FirstName,
            CrmLeads::LastName,
            CrmLeads::Email,
            CrmLeads::Phone,
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(CrmLeads::Table)
                        .drop_column(col)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        clear_leads(manager).await?;

        if manager.get_connection().get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            manager
                .drop_foreign_key(
                    ForeignKey::drop()
                        .name("fk_crm_leads_contact_id")
                        .table(CrmLeads::Table)
                        .to_owned(),
                )
                .await?;
        }

        manager
            .alter_table(
                Table::alter()
                    .table(CrmLeads::Table)
                    .drop_column(CrmLeads::ContactId)
                    .to_owned(),
            )
            .await?;

        for col in [
            CrmLeads::CompanyName,
            CrmLeads::FirstName,
            CrmLeads::LastName,
            CrmLeads::Email,
            CrmLeads::Phone,
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(CrmLeads::Table)
                        .add_column(ColumnDef::new(col).text())
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}
