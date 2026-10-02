use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum HrAttendances {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    UserId,
    StartedAt,
    EndedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HrAttendances::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrAttendances::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(HrAttendances::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrAttendances::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(HrAttendances::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrAttendances::StartedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrAttendances::EndedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hr_attendances_user_id")
                            .from(HrAttendances::Table, HrAttendances::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_attendances_user_id")
                    .table(HrAttendances::Table)
                    .col(HrAttendances::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(HrAttendances::Table).to_owned())
            .await
    }
}
