use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum TaskPushSubscriptions {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    UserId,
    Endpoint,
    P256dh,
    Auth,
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
                    .table(TaskPushSubscriptions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TaskPushSubscriptions::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(TaskPushSubscriptions::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(TaskPushSubscriptions::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(TaskPushSubscriptions::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TaskPushSubscriptions::Endpoint)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TaskPushSubscriptions::P256dh)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TaskPushSubscriptions::Auth)
                            .text()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_task_push_subscriptions_user_id")
                            .from(TaskPushSubscriptions::Table, TaskPushSubscriptions::UserId)
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
                    .name("uix_task_push_subscriptions_endpoint")
                    .table(TaskPushSubscriptions::Table)
                    .col(TaskPushSubscriptions::Endpoint)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_task_push_subscriptions_user_id")
                    .table(TaskPushSubscriptions::Table)
                    .col(TaskPushSubscriptions::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TaskPushSubscriptions::Table).to_owned())
            .await
    }
}
