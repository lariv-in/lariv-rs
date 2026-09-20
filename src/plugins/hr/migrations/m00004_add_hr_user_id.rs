use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum HrApplicants {
    Table,
    UserId,
}

#[derive(DeriveIden)]
enum HrProbations {
    Table,
    UserId,
}

#[derive(DeriveIden)]
enum HrEmployees {
    Table,
    UserId,
}

#[derive(DeriveIden)]
enum HrExEmployees {
    Table,
    UserId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(HrApplicants::Table)
                    .add_column(ColumnDef::new(HrApplicants::UserId).big_integer().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_hr_applicants_user_id")
                    .from(HrApplicants::Table, HrApplicants::UserId)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_applicants_user_id")
                    .table(HrApplicants::Table)
                    .col(HrApplicants::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(HrProbations::Table)
                    .add_column(ColumnDef::new(HrProbations::UserId).big_integer().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_hr_probations_user_id")
                    .from(HrProbations::Table, HrProbations::UserId)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_probations_user_id")
                    .table(HrProbations::Table)
                    .col(HrProbations::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(HrEmployees::Table)
                    .add_column(ColumnDef::new(HrEmployees::UserId).big_integer().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_hr_employees_user_id")
                    .from(HrEmployees::Table, HrEmployees::UserId)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_employees_user_id")
                    .table(HrEmployees::Table)
                    .col(HrEmployees::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(HrExEmployees::Table)
                    .add_column(ColumnDef::new(HrExEmployees::UserId).big_integer().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_hr_ex_employees_user_id")
                    .from(HrExEmployees::Table, HrExEmployees::UserId)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_ex_employees_user_id")
                    .table(HrExEmployees::Table)
                    .col(HrExEmployees::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_hr_ex_employees_user_id")
                    .table(HrExEmployees::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_hr_ex_employees_user_id")
                    .table(HrExEmployees::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(HrExEmployees::Table)
                    .drop_column(HrExEmployees::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_hr_employees_user_id")
                    .table(HrEmployees::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_hr_employees_user_id")
                    .table(HrEmployees::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(HrEmployees::Table)
                    .drop_column(HrEmployees::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_hr_probations_user_id")
                    .table(HrProbations::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_hr_probations_user_id")
                    .table(HrProbations::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(HrProbations::Table)
                    .drop_column(HrProbations::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_hr_applicants_user_id")
                    .table(HrApplicants::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_hr_applicants_user_id")
                    .table(HrApplicants::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(HrApplicants::Table)
                    .drop_column(HrApplicants::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
