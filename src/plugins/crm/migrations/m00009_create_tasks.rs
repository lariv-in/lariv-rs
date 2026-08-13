use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmTasks {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Title,
    Description,
    AssignedToId,
    DueDate,
}

#[derive(Iden)]
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
                    .table(CrmTasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CrmTasks::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CrmTasks::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CrmTasks::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(CrmTasks::Title).text().not_null())
                    .col(ColumnDef::new(CrmTasks::Description).text())
                    .col(
                        ColumnDef::new(CrmTasks::AssignedToId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CrmTasks::DueDate).date())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crm_tasks_assigned_to_id")
                            .from(CrmTasks::Table, CrmTasks::AssignedToId)
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
                    .name("idx_crm_tasks_assigned_to_id")
                    .table(CrmTasks::Table)
                    .col(CrmTasks::AssignedToId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CrmTasks::Table).to_owned())
            .await
    }
}
