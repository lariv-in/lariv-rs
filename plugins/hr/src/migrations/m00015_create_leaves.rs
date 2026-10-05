use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum HrLeaveApplications {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    AppliedById,
    Date,
    Reason,
    LeaveType,
}

#[derive(DeriveIden)]
enum HrApprovedLeaves {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    LeaveApplicationId,
    ApprovedById,
    ApprovedAt,
}

#[derive(DeriveIden)]
enum HrRejectedLeaves {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    LeaveApplicationId,
    RejectedById,
    RejectedAt,
    Reason,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HrLeaveApplications::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrLeaveApplications::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(HrLeaveApplications::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrLeaveApplications::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(HrLeaveApplications::AppliedById)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(HrLeaveApplications::Date).date().not_null())
                    .col(
                        ColumnDef::new(HrLeaveApplications::Reason)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrLeaveApplications::LeaveType)
                            .string_len(32)
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hr_leave_applications_applied_by_id")
                            .from(HrLeaveApplications::Table, HrLeaveApplications::AppliedById)
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
                    .name("idx_hr_leave_applications_applied_by_id")
                    .table(HrLeaveApplications::Table)
                    .col(HrLeaveApplications::AppliedById)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_leave_applications_date")
                    .table(HrLeaveApplications::Table)
                    .col(HrLeaveApplications::Date)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(HrApprovedLeaves::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrApprovedLeaves::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(HrApprovedLeaves::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrApprovedLeaves::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(HrApprovedLeaves::LeaveApplicationId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrApprovedLeaves::ApprovedById)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrApprovedLeaves::ApprovedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hr_approved_leaves_leave_application_id")
                            .from(
                                HrApprovedLeaves::Table,
                                HrApprovedLeaves::LeaveApplicationId,
                            )
                            .to(HrLeaveApplications::Table, HrLeaveApplications::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hr_approved_leaves_approved_by_id")
                            .from(HrApprovedLeaves::Table, HrApprovedLeaves::ApprovedById)
                            .to(Users::Table, Users::Id)
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
                    .name("uix_hr_approved_leaves_leave_application_id")
                    .table(HrApprovedLeaves::Table)
                    .col(HrApprovedLeaves::LeaveApplicationId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_approved_leaves_approved_by_id")
                    .table(HrApprovedLeaves::Table)
                    .col(HrApprovedLeaves::ApprovedById)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(HrRejectedLeaves::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrRejectedLeaves::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(HrRejectedLeaves::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrRejectedLeaves::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(HrRejectedLeaves::LeaveApplicationId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrRejectedLeaves::RejectedById)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrRejectedLeaves::RejectedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(HrRejectedLeaves::Reason).text())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hr_rejected_leaves_leave_application_id")
                            .from(
                                HrRejectedLeaves::Table,
                                HrRejectedLeaves::LeaveApplicationId,
                            )
                            .to(HrLeaveApplications::Table, HrLeaveApplications::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hr_rejected_leaves_rejected_by_id")
                            .from(HrRejectedLeaves::Table, HrRejectedLeaves::RejectedById)
                            .to(Users::Table, Users::Id)
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
                    .name("uix_hr_rejected_leaves_leave_application_id")
                    .table(HrRejectedLeaves::Table)
                    .col(HrRejectedLeaves::LeaveApplicationId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_rejected_leaves_rejected_by_id")
                    .table(HrRejectedLeaves::Table)
                    .col(HrRejectedLeaves::RejectedById)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(HrRejectedLeaves::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(HrApprovedLeaves::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(HrLeaveApplications::Table).to_owned())
            .await
    }
}
