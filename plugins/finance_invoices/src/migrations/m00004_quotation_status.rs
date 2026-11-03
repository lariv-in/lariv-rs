use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Quotations {
    Table,
    Status,
}

#[derive(DeriveIden)]
enum QuotationStatus {
    #[sea_orm(iden = "quotation_status")]
    Enum,
    #[sea_orm(iden = "draft")]
    Draft,
    #[sea_orm(iden = "accepted")]
    Accepted,
    #[sea_orm(iden = "cancelled")]
    Cancelled,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(QuotationStatus::Enum)
                    .values([
                        QuotationStatus::Draft,
                        QuotationStatus::Accepted,
                        QuotationStatus::Cancelled,
                    ])
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Quotations::Table)
                    .add_column(
                        ColumnDef::new(Quotations::Status)
                            .custom(QuotationStatus::Enum)
                            .not_null()
                            .default("draft"),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Quotations::Table)
                    .drop_column(Quotations::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_type(Type::drop().name(QuotationStatus::Enum).to_owned())
            .await
    }
}
