use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum HrApplicants {
    Table,
    FormResponseId,
    Age,
    Gender,
    ResumeVnodeId,
    JobFormId,
    Remarks,
    Address,
}

#[derive(DeriveIden)]
enum FormResponses {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum FilesystemNodes {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum HrJobForms {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(HrApplicants::Table)
                    .add_column(
                        ColumnDef::new(HrApplicants::FormResponseId)
                            .big_integer()
                            .null(),
                    )
                    .add_column(ColumnDef::new(HrApplicants::Age).big_integer().null())
                    .add_column(ColumnDef::new(HrApplicants::Gender).text().null())
                    .add_column(
                        ColumnDef::new(HrApplicants::ResumeVnodeId)
                            .big_integer()
                            .null(),
                    )
                    .add_column(ColumnDef::new(HrApplicants::JobFormId).big_integer().null())
                    .add_column(ColumnDef::new(HrApplicants::Remarks).text().null())
                    .add_column(ColumnDef::new(HrApplicants::Address).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_hr_applicants_form_response_id")
                    .from(HrApplicants::Table, HrApplicants::FormResponseId)
                    .to(FormResponses::Table, FormResponses::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_applicants_form_response_id")
                    .table(HrApplicants::Table)
                    .col(HrApplicants::FormResponseId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_hr_applicants_resume_vnode_id")
                    .from(HrApplicants::Table, HrApplicants::ResumeVnodeId)
                    .to(FilesystemNodes::Table, FilesystemNodes::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_applicants_resume_vnode_id")
                    .table(HrApplicants::Table)
                    .col(HrApplicants::ResumeVnodeId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_hr_applicants_job_form_id")
                    .from(HrApplicants::Table, HrApplicants::JobFormId)
                    .to(HrJobForms::Table, HrJobForms::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_applicants_job_form_id")
                    .table(HrApplicants::Table)
                    .col(HrApplicants::JobFormId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_hr_applicants_form_response_id")
                    .table(HrApplicants::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_hr_applicants_form_response_id")
                    .table(HrApplicants::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_hr_applicants_resume_vnode_id")
                    .table(HrApplicants::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_hr_applicants_resume_vnode_id")
                    .table(HrApplicants::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_hr_applicants_job_form_id")
                    .table(HrApplicants::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_hr_applicants_job_form_id")
                    .table(HrApplicants::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(HrApplicants::Table)
                    .drop_column(HrApplicants::FormResponseId)
                    .drop_column(HrApplicants::Age)
                    .drop_column(HrApplicants::Gender)
                    .drop_column(HrApplicants::ResumeVnodeId)
                    .drop_column(HrApplicants::JobFormId)
                    .drop_column(HrApplicants::Remarks)
                    .drop_column(HrApplicants::Address)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
