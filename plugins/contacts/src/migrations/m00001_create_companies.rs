//! Companies belong to contacts. The table name stays `crm_companies`.
//!
//! CRM's `m00001_create_crm` still creates this table with `IF NOT EXISTS` so
//! databases that already applied that migration are unchanged. A deployment
//! that installs contacts without CRM gets the current columns here.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmCompanies {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    City,
    Pincode,
    State,
    Website,
    Gstin,
    Cin,
    Pan,
    Phone,
    Email,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CrmCompanies::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CrmCompanies::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CrmCompanies::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CrmCompanies::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CrmCompanies::Name).text().not_null())
                    .col(ColumnDef::new(Alias::new("address_line_1")).text())
                    .col(ColumnDef::new(Alias::new("address_line_2")).text())
                    .col(ColumnDef::new(CrmCompanies::City).text())
                    .col(ColumnDef::new(CrmCompanies::Pincode).text())
                    .col(ColumnDef::new(CrmCompanies::State).text())
                    .col(ColumnDef::new(CrmCompanies::Website).text())
                    .col(ColumnDef::new(CrmCompanies::Gstin).text())
                    .col(ColumnDef::new(CrmCompanies::Cin).text())
                    .col(ColumnDef::new(CrmCompanies::Pan).text())
                    .col(ColumnDef::new(CrmCompanies::Phone).text())
                    .col(ColumnDef::new(CrmCompanies::Email).text())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CrmCompanies::Table).to_owned())
            .await
    }
}
