use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum HrOvertimeApplications {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    UserId,
    StartTime,
    EndTime,
    Reason,
}

#[derive(DeriveIden)]
enum HrApprovedOvertimes {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    UserId,
    StartTime,
    EndTime,
    ApprovedById,
    ApprovedAt,
    OvertimeApplicationId,
}

#[derive(DeriveIden)]
enum HrRejectedOvertimes {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    OvertimeApplicationId,
    RejectedById,
    RejectedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HrOvertimeApplications::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrOvertimeApplications::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HrOvertimeApplications::CreatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(HrOvertimeApplications::UpdatedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(HrOvertimeApplications::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrOvertimeApplications::StartTime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrOvertimeApplications::EndTime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(HrOvertimeApplications::Reason).text())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hr_overtime_applications_user_id")
                            .from(
                                HrOvertimeApplications::Table,
                                HrOvertimeApplications::UserId,
                            )
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
                    .name("idx_hr_overtime_applications_user_id")
                    .table(HrOvertimeApplications::Table)
                    .col(HrOvertimeApplications::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_overtime_applications_start_time")
                    .table(HrOvertimeApplications::Table)
                    .col(HrOvertimeApplications::StartTime)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(HrApprovedOvertimes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrApprovedOvertimes::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(HrApprovedOvertimes::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrApprovedOvertimes::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(HrApprovedOvertimes::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrApprovedOvertimes::StartTime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrApprovedOvertimes::EndTime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrApprovedOvertimes::ApprovedById)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrApprovedOvertimes::ApprovedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(HrApprovedOvertimes::OvertimeApplicationId).big_integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hr_approved_overtimes_user_id")
                            .from(HrApprovedOvertimes::Table, HrApprovedOvertimes::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hr_approved_overtimes_approved_by_id")
                            .from(
                                HrApprovedOvertimes::Table,
                                HrApprovedOvertimes::ApprovedById,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hr_approved_overtimes_application_id")
                            .from(
                                HrApprovedOvertimes::Table,
                                HrApprovedOvertimes::OvertimeApplicationId,
                            )
                            .to(HrOvertimeApplications::Table, HrOvertimeApplications::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_hr_approved_overtimes_application_id")
                    .table(HrApprovedOvertimes::Table)
                    .col(HrApprovedOvertimes::OvertimeApplicationId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_approved_overtimes_user_id")
                    .table(HrApprovedOvertimes::Table)
                    .col(HrApprovedOvertimes::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_approved_overtimes_approved_by_id")
                    .table(HrApprovedOvertimes::Table)
                    .col(HrApprovedOvertimes::ApprovedById)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(HrRejectedOvertimes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HrRejectedOvertimes::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(HrRejectedOvertimes::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(HrRejectedOvertimes::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(HrRejectedOvertimes::OvertimeApplicationId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrRejectedOvertimes::RejectedById)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HrRejectedOvertimes::RejectedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hr_rejected_overtimes_application_id")
                            .from(
                                HrRejectedOvertimes::Table,
                                HrRejectedOvertimes::OvertimeApplicationId,
                            )
                            .to(HrOvertimeApplications::Table, HrOvertimeApplications::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hr_rejected_overtimes_rejected_by_id")
                            .from(
                                HrRejectedOvertimes::Table,
                                HrRejectedOvertimes::RejectedById,
                            )
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
                    .name("uix_hr_rejected_overtimes_application_id")
                    .table(HrRejectedOvertimes::Table)
                    .col(HrRejectedOvertimes::OvertimeApplicationId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_hr_rejected_overtimes_rejected_by_id")
                    .table(HrRejectedOvertimes::Table)
                    .col(HrRejectedOvertimes::RejectedById)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(HrRejectedOvertimes::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(HrApprovedOvertimes::Table).to_owned())
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(HrOvertimeApplications::Table)
                    .to_owned(),
            )
            .await
    }
}
