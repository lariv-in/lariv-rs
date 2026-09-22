use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};
use sea_orm_migration::prelude::*;

use crate::plugins::hr::{
    entities::{applicant, employee, ex_employee, probation},
    logic::{person::PersonInput, user::create_hr_user_with_password},
    roles,
    seed::ensure_hr_roles,
};
use crate::plugins::users::entities::user::{self, Entity as UserEntity};

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
        let conn = manager.get_connection();

        ensure_hr_roles(conn).await?;

        add_nullable_user_id(manager, HrApplicants::Table, HrApplicants::UserId).await?;
        add_nullable_user_id(manager, HrProbations::Table, HrProbations::UserId).await?;
        add_nullable_user_id(manager, HrEmployees::Table, HrEmployees::UserId).await?;
        add_nullable_user_id(manager, HrExEmployees::Table, HrExEmployees::UserId).await?;

        backfill_applicants(conn).await?;
        backfill_probations(conn).await?;
        backfill_employees(conn).await?;
        backfill_ex_employees(conn).await?;

        set_user_id_not_null(manager, HrApplicants::Table, HrApplicants::UserId).await?;
        set_user_id_not_null(manager, HrProbations::Table, HrProbations::UserId).await?;
        set_user_id_not_null(manager, HrEmployees::Table, HrEmployees::UserId).await?;
        set_user_id_not_null(manager, HrExEmployees::Table, HrExEmployees::UserId).await?;

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

async fn add_nullable_user_id(
    manager: &SchemaManager<'_>,
    table: impl IntoIden,
    user_id: impl IntoIden,
) -> Result<(), DbErr> {
    manager
        .alter_table(
            Table::alter()
                .table(table)
                .add_column_if_not_exists(ColumnDef::new(user_id).big_integer().null())
                .to_owned(),
        )
        .await
}

async fn set_user_id_not_null(
    manager: &SchemaManager<'_>,
    table: impl IntoIden,
    user_id: impl IntoIden,
) -> Result<(), DbErr> {
    manager
        .alter_table(
            Table::alter()
                .table(table)
                .modify_column(ColumnDef::new(user_id).big_integer().not_null())
                .to_owned(),
        )
        .await
}

async fn backfill_applicants<C: ConnectionTrait>(conn: &C) -> Result<(), DbErr> {
    backfill_people::<_, applicant::Entity>(
        conn,
        applicant::Column::Id,
        applicant::Column::Name,
        applicant::Column::Email,
        applicant::Column::Mobile,
        applicant::Column::UserId,
        roles::APPLICANT,
    )
    .await
}

async fn backfill_probations<C: ConnectionTrait>(conn: &C) -> Result<(), DbErr> {
    backfill_people::<_, probation::Entity>(
        conn,
        probation::Column::Id,
        probation::Column::Name,
        probation::Column::Email,
        probation::Column::Mobile,
        probation::Column::UserId,
        roles::PROBATION,
    )
    .await
}

async fn backfill_employees<C: ConnectionTrait>(conn: &C) -> Result<(), DbErr> {
    backfill_people::<_, employee::Entity>(
        conn,
        employee::Column::Id,
        employee::Column::Name,
        employee::Column::Email,
        employee::Column::Mobile,
        employee::Column::UserId,
        roles::EMPLOYEE,
    )
    .await
}

async fn backfill_ex_employees<C: ConnectionTrait>(conn: &C) -> Result<(), DbErr> {
    backfill_people::<_, ex_employee::Entity>(
        conn,
        ex_employee::Column::Id,
        ex_employee::Column::Name,
        ex_employee::Column::Email,
        ex_employee::Column::Mobile,
        ex_employee::Column::UserId,
        roles::EX_EMPLOYEE,
    )
    .await
}

async fn backfill_people<C, E>(
    conn: &C,
    id_col: E::Column,
    name_col: E::Column,
    email_col: E::Column,
    mobile_col: E::Column,
    user_id_col: E::Column,
    role_name: &str,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
    E: EntityTrait,
{
    let rows = E::find()
        .select_only()
        .column(id_col)
        .column(name_col)
        .column(email_col)
        .column(mobile_col)
        .filter(Expr::col(user_id_col).is_null())
        .into_tuple::<(i64, String, String, String)>()
        .all(conn)
        .await?;

    for (row_id, name, email, mobile) in rows {
        let person = migration_person(row_id, role_name, &name, &email, &mobile);
        let user_id = resolve_or_create_user_id(conn, &person, role_name).await?;

        E::update_many()
            .col_expr(user_id_col, Expr::value(user_id))
            .filter(id_col.eq(row_id))
            .exec(conn)
            .await?;
    }

    Ok(())
}

async fn resolve_or_create_user_id<C: ConnectionTrait>(
    conn: &C,
    person: &PersonInput,
    role_name: &str,
) -> Result<i64, DbErr> {
    if let Some(user_id) = find_user_id(conn, &person.email, &person.mobile).await? {
        return Ok(user_id);
    }

    create_hr_user_with_password(conn, person, role_name, "")
        .await
        .map_err(DbErr::Custom)
}

async fn find_user_id<C: ConnectionTrait>(
    conn: &C,
    email: &str,
    phone: &str,
) -> Result<Option<i64>, DbErr> {
    if let Some(user) = UserEntity::find()
        .filter(user::Column::Email.eq(email))
        .one(conn)
        .await?
    {
        return Ok(Some(user.id));
    }

    if let Some(user) = UserEntity::find()
        .filter(user::Column::Phone.eq(phone))
        .one(conn)
        .await?
    {
        return Ok(Some(user.id));
    }

    Ok(None)
}

fn migration_person(
    row_id: i64,
    role_name: &str,
    name: &str,
    email: &str,
    mobile: &str,
) -> PersonInput {
    PersonInput {
        name: name.trim().to_string(),
        email: if email.trim().is_empty() {
            format!("hr-{role_name}-{row_id}@migration.local")
        } else {
            email.trim().to_string()
        },
        mobile: if mobile.trim().is_empty() {
            format!("hr-{role_name}-{row_id}")
        } else {
            mobile.trim().to_string()
        },
    }
}
