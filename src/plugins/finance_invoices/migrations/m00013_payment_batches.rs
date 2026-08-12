use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum PaymentBatches {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Datetime,
    AccountId,
    JournalEntryId,
    TotalAmount,
}

#[derive(DeriveIden)]
enum Payments {
    Table,
    PaymentBatchId,
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum JournalEntries {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PaymentBatches::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PaymentBatches::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PaymentBatches::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PaymentBatches::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(PaymentBatches::DeletedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(PaymentBatches::Datetime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PaymentBatches::AccountId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PaymentBatches::JournalEntryId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PaymentBatches::TotalAmount)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_payment_batches_account_id")
                            .from(PaymentBatches::Table, PaymentBatches::AccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_payment_batches_journal_entry_id")
                            .from(PaymentBatches::Table, PaymentBatches::JournalEntryId)
                            .to(JournalEntries::Table, JournalEntries::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Payments::Table)
                    .add_column(
                        ColumnDef::new(Payments::PaymentBatchId)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_payments_payment_batch_id")
                    .from(Payments::Table, Payments::PaymentBatchId)
                    .to(PaymentBatches::Table, PaymentBatches::Id)
                    .on_delete(ForeignKeyAction::Restrict)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_payments_payment_batch_id")
                    .table(Payments::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Payments::Table)
                    .drop_column(Payments::PaymentBatchId)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(PaymentBatches::Table).to_owned())
            .await
    }
}
