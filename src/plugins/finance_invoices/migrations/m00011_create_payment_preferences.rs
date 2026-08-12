use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum PaymentPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    PaymentAccountId,
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PaymentPreferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PaymentPreferences::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(PaymentPreferences::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(PaymentPreferences::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(PaymentPreferences::DeletedAt).timestamp_with_time_zone(),
                    )
                    .col(ColumnDef::new(PaymentPreferences::PaymentAccountId).big_integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_payment_preferences_payment_account_id")
                            .from(PaymentPreferences::Table, PaymentPreferences::PaymentAccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_payment_preferences_deleted_at")
                    .table(PaymentPreferences::Table)
                    .col(PaymentPreferences::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_payment_preferences_payment_account_id")
                    .table(PaymentPreferences::Table)
                    .col(PaymentPreferences::PaymentAccountId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PaymentPreferences::Table).to_owned())
            .await
    }
}
