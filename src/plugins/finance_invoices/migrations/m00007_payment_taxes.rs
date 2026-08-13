use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum PaymentTaxes {
    Table,
    PaymentId,
    TaxId,
}

#[derive(DeriveIden)]
enum Payments {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Taxes {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PaymentTaxes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PaymentTaxes::PaymentId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(PaymentTaxes::TaxId).big_integer().not_null())
                    .primary_key(
                        Index::create()
                            .col(PaymentTaxes::PaymentId)
                            .col(PaymentTaxes::TaxId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_payment_taxes_payment_id")
                            .from(PaymentTaxes::Table, PaymentTaxes::PaymentId)
                            .to(Payments::Table, Payments::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_payment_taxes_tax_id")
                            .from(PaymentTaxes::Table, PaymentTaxes::TaxId)
                            .to(Taxes::Table, Taxes::Id)
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
                    .name("idx_payment_taxes_tax_id")
                    .table(PaymentTaxes::Table)
                    .col(PaymentTaxes::TaxId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PaymentTaxes::Table).to_owned())
            .await
    }
}
