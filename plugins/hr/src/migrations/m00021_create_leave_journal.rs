use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum HrLeaveJournal {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    UserId,
    Datetime,
    LeaveType,
    Amount,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HrLeaveJournal::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrLeaveJournal::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(HrLeaveJournal::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrLeaveJournal::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(HrLeaveJournal::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveJournal::Datetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveJournal::LeaveType)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveJournal::Amount)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hr_leave_journal_user_id")
                            .from(HrLeaveJournal::Table, HrLeaveJournal::UserId)
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
                    .name("idx_hr_leave_journal_user_id_leave_type")
                    .table(HrLeaveJournal::Table)
                    .col(HrLeaveJournal::UserId)
                    .col(HrLeaveJournal::LeaveType)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(HrLeaveJournal::Table).to_owned())
            .await
    }
}
