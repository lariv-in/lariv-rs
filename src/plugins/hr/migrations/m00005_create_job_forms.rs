use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Forms {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum HrJobForms {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    JobTitle,
    SalaryRange,
    ExperienceRequired,
    Description,
    FormId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HrJobForms::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrJobForms::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(HrJobForms::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrJobForms::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrJobForms::JobTitle).text().not_null())
                    .col(ColumnDef::new(HrJobForms::SalaryRange).text())
                    .col(ColumnDef::new(HrJobForms::ExperienceRequired).text())
                    .col(ColumnDef::new(HrJobForms::Description).text().not_null())
                    .col(ColumnDef::new(HrJobForms::FormId).big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hr_job_forms_form_id")
                            .from(HrJobForms::Table, HrJobForms::FormId)
                            .to(Forms::Table, Forms::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_job_forms_form_id")
                    .table(HrJobForms::Table)
                    .col(HrJobForms::FormId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(HrJobForms::Table).to_owned())
            .await
    }
}
