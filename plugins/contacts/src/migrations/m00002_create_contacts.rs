//! People belong to contacts. The table name stays `crm_contacts`.
//!
//! `company_id` is nullable. CRM's later migrations make the same change on
//! databases that already created the older NOT NULL column.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmContacts {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    CompanyId,
    Name,
    Email,
    Phone,
    IsPrimary,
}

#[derive(DeriveIden)]
enum CrmCompanies {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CrmContacts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CrmContacts::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CrmContacts::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CrmContacts::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CrmContacts::CompanyId).big_integer())
                    .col(ColumnDef::new(CrmContacts::Name).text().not_null())
                    .col(ColumnDef::new(CrmContacts::Email).text())
                    .col(ColumnDef::new(CrmContacts::Phone).text())
                    .col(
                        ColumnDef::new(CrmContacts::IsPrimary)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crm_contacts_company_id")
                            .from(CrmContacts::Table, CrmContacts::CompanyId)
                            .to(CrmCompanies::Table, CrmCompanies::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_crm_contacts_company_id")
                    .table(CrmContacts::Table)
                    .col(CrmContacts::CompanyId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CrmContacts::Table).to_owned())
            .await
    }
}
