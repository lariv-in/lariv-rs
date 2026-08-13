use sea_orm::Statement;
use sea_orm_migration::prelude::*;

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
enum CrmContacts {
    Table,
    Id,
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
            .execute(Statement::from_string(
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
        clear_leads(manager).await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CrmLeads::Table)
                    .add_column(ColumnDef::new(CrmLeads::ContactId).big_integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CrmLeads::Table)
                    .modify_column(ColumnDef::new(CrmLeads::ContactId).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_crm_leads_contact_id")
                    .from(CrmLeads::Table, CrmLeads::ContactId)
                    .to(CrmContacts::Table, CrmContacts::Id)
                    .on_delete(ForeignKeyAction::Restrict)
                    .on_update(ForeignKeyAction::Cascade)
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
                        .drop_column(col)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        clear_leads(manager).await?;

        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_crm_leads_contact_id")
                    .table(CrmLeads::Table)
                    .to_owned(),
            )
            .await?;

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
