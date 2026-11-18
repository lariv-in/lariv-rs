//! Per-user active company preference used when stamping `app.company_id`.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CompanyUserPrefs {
    Table,
    UserId,
    ActiveCompanyId,
    UpdatedAt,
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
                    .table(CompanyUserPrefs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CompanyUserPrefs::UserId)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        // NULL = no company context (superuser "system" mode).
                        ColumnDef::new(CompanyUserPrefs::ActiveCompanyId)
                            .big_integer()
                            .null(),
                    )
                    .col(ColumnDef::new(CompanyUserPrefs::UpdatedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_company_user_prefs_user_id")
                            .from(CompanyUserPrefs::Table, CompanyUserPrefs::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_company_user_prefs_company_id")
                            .from(CompanyUserPrefs::Table, CompanyUserPrefs::ActiveCompanyId)
                            .to(Companies::Table, Companies::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CompanyUserPrefs::Table).to_owned())
            .await
    }
}
