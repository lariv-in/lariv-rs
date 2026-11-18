//! Users belong to companies through `company_memberships`.
//!
//! `owned_by` marks the company owners. A company can have several owners; a user
//! can belong to several companies.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CompanyMemberships {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    UserId,
    CompanyId,
    OwnedBy,
}

#[derive(DeriveIden)]
enum Companies {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CompanyMemberships::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CompanyMemberships::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CompanyMemberships::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CompanyMemberships::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(CompanyMemberships::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CompanyMemberships::CompanyId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CompanyMemberships::OwnedBy)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_company_memberships_user_id")
                            .from(CompanyMemberships::Table, CompanyMemberships::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_company_memberships_company_id")
                            .from(CompanyMemberships::Table, CompanyMemberships::CompanyId)
                            .to(Companies::Table, Companies::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_company_memberships_user_company")
                    .table(CompanyMemberships::Table)
                    .col(CompanyMemberships::UserId)
                    .col(CompanyMemberships::CompanyId)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_company_memberships_company_id")
                    .table(CompanyMemberships::Table)
                    .col(CompanyMemberships::CompanyId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CompanyMemberships::Table).to_owned())
            .await
    }
}
