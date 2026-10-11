use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum TaskVapidKeys {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Singleton,
    PublicKey,
    PrivateKey,
    Subject,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TaskVapidKeys::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TaskVapidKeys::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TaskVapidKeys::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(TaskVapidKeys::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(TaskVapidKeys::Singleton)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(ColumnDef::new(TaskVapidKeys::PublicKey).text().not_null())
                    .col(ColumnDef::new(TaskVapidKeys::PrivateKey).text().not_null())
                    .col(ColumnDef::new(TaskVapidKeys::Subject).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uix_task_vapid_keys_singleton")
                    .table(TaskVapidKeys::Table)
                    .col(TaskVapidKeys::Singleton)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TaskVapidKeys::Table).to_owned())
            .await
    }
}
