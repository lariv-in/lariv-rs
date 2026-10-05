use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmLeads {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    CompanyName,
    FirstName,
    LastName,
    Email,
    Phone,
    Source,
    Notes,
}

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
}

#[derive(DeriveIden)]
enum CrmContacts {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    CompanyId,
    FirstName,
    LastName,
    Email,
    Phone,
    Title,
    IsPrimary,
}

#[derive(DeriveIden)]
enum CrmDeals {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    CompanyId,
    PrimaryContactId,
    Name,
    Amount,
    Stage,
    ExpectedCloseDate,
}

#[derive(DeriveIden)]
enum CrmConvertedLeads {
    Table,
    Id,
    CreatedAt,
    LeadId,
    ConvertedAt,
    CompanyId,
    ContactId,
    DealId,
}

#[derive(DeriveIden)]
enum CrmFailedLeads {
    Table,
    Id,
    CreatedAt,
    LeadId,
    FailedAt,
    Reason,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CrmLeads::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CrmLeads::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CrmLeads::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CrmLeads::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CrmLeads::CompanyName).text())
                    .col(ColumnDef::new(CrmLeads::FirstName).text())
                    .col(ColumnDef::new(CrmLeads::LastName).text())
                    .col(ColumnDef::new(CrmLeads::Email).text())
                    .col(ColumnDef::new(CrmLeads::Phone).text())
                    .col(
                        ColumnDef::new(CrmLeads::Source)
                            .string_len(32)
                            .not_null()
                            .default("web_form"),
                    )
                    .col(ColumnDef::new(CrmLeads::Notes).text())
                    .to_owned(),
            )
            .await?;

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
                    .to_owned(),
            )
            .await?;

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
                    .col(
                        ColumnDef::new(CrmContacts::CompanyId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CrmContacts::FirstName).text().not_null())
                    .col(ColumnDef::new(CrmContacts::LastName).text())
                    .col(ColumnDef::new(CrmContacts::Email).text())
                    .col(ColumnDef::new(CrmContacts::Phone).text())
                    .col(ColumnDef::new(CrmContacts::Title).text())
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
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CrmDeals::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CrmDeals::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CrmDeals::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CrmDeals::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CrmDeals::CompanyId).big_integer().not_null())
                    .col(
                        ColumnDef::new(CrmDeals::PrimaryContactId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CrmDeals::Name).text().not_null())
                    .col(ColumnDef::new(CrmDeals::Amount).decimal_len(19, 4))
                    .col(
                        ColumnDef::new(CrmDeals::Stage)
                            .string_len(32)
                            .not_null()
                            .default("prospecting"),
                    )
                    .col(ColumnDef::new(CrmDeals::ExpectedCloseDate).date())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crm_deals_company_id")
                            .from(CrmDeals::Table, CrmDeals::CompanyId)
                            .to(CrmCompanies::Table, CrmCompanies::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crm_deals_primary_contact_id")
                            .from(CrmDeals::Table, CrmDeals::PrimaryContactId)
                            .to(CrmContacts::Table, CrmContacts::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CrmConvertedLeads::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CrmConvertedLeads::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CrmConvertedLeads::CreatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(CrmConvertedLeads::LeadId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrmConvertedLeads::ConvertedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrmConvertedLeads::CompanyId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrmConvertedLeads::ContactId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CrmConvertedLeads::DealId).big_integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crm_converted_leads_lead_id")
                            .from(CrmConvertedLeads::Table, CrmConvertedLeads::LeadId)
                            .to(CrmLeads::Table, CrmLeads::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crm_converted_leads_company_id")
                            .from(CrmConvertedLeads::Table, CrmConvertedLeads::CompanyId)
                            .to(CrmCompanies::Table, CrmCompanies::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crm_converted_leads_contact_id")
                            .from(CrmConvertedLeads::Table, CrmConvertedLeads::ContactId)
                            .to(CrmContacts::Table, CrmContacts::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crm_converted_leads_deal_id")
                            .from(CrmConvertedLeads::Table, CrmConvertedLeads::DealId)
                            .to(CrmDeals::Table, CrmDeals::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CrmFailedLeads::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CrmFailedLeads::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CrmFailedLeads::CreatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(CrmFailedLeads::LeadId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrmFailedLeads::FailedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CrmFailedLeads::Reason).text())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crm_failed_leads_lead_id")
                            .from(CrmFailedLeads::Table, CrmFailedLeads::LeadId)
                            .to(CrmLeads::Table, CrmLeads::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_crm_converted_leads_lead_id")
                    .table(CrmConvertedLeads::Table)
                    .col(CrmConvertedLeads::LeadId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_crm_failed_leads_lead_id")
                    .table(CrmFailedLeads::Table)
                    .col(CrmFailedLeads::LeadId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_crm_contacts_company_id")
                    .table(CrmContacts::Table)
                    .col(CrmContacts::CompanyId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_crm_deals_company_id")
                    .table(CrmDeals::Table)
                    .col(CrmDeals::CompanyId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CrmFailedLeads::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CrmConvertedLeads::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CrmDeals::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CrmContacts::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CrmCompanies::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CrmLeads::Table).to_owned())
            .await
    }
}
