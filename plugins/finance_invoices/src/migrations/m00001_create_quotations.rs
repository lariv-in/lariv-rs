use lariv_core::db::migration_sql::{
    self, FK_QUOTATION_GOODS_LINE_PRODUCT, FK_QUOTATION_GOODS_LINE_TAX,
    FK_QUOTATION_SERVICE_LINE_SERVICE, FK_QUOTATION_SERVICE_LINE_TAX, FK_QUOTATIONS_CUSTOMER,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Quotations {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Date,
    CustomerId,
    Number,
    BankAccount,
    Remarks,
}

#[derive(DeriveIden)]
enum QuotationGoodsLines {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    QuotationId,
    ProductId,
    Rate,
    Quantity,
    VariableValues,
    PreTaxAmount,
    Remarks,
}

#[derive(DeriveIden)]
enum QuotationServiceLines {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    QuotationId,
    ServiceId,
    Rate,
    Quantity,
    VariableValues,
    PreTaxAmount,
    Remarks,
}

#[derive(DeriveIden)]
enum QuotationGoodsLineTaxes {
    Table,
    LineId,
    TaxId,
}

#[derive(DeriveIden)]
enum QuotationServiceLineTaxes {
    Table,
    LineId,
    TaxId,
}

#[derive(DeriveIden)]
enum QuotationPreferences {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    QuotationNumberFormat,
    QuotationPdfTemplate,
    QuotationEmailSubject,
    QuotationEmailBody,
    CompanyName,
    CompanyAddress,
    CompanyPhone,
    CompanyGstin,
    PlaceOfSupply,
    DefaultBankAccount,
    CompanyLogoVnodeId,
    CompanySignatureVnodeId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Quotations::Table)
                    .col(
                        ColumnDef::new(Quotations::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Quotations::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Quotations::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Quotations::Date).date().not_null())
                    .col(
                        ColumnDef::new(Quotations::CustomerId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Quotations::Number).text().not_null())
                    .col(ColumnDef::new(Quotations::BankAccount).text())
                    .col(
                        ColumnDef::new(Quotations::Remarks)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_quotations_customer_id")
                    .table(Quotations::Table)
                    .col(Quotations::CustomerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(QuotationGoodsLines::Table)
                    .col(
                        ColumnDef::new(QuotationGoodsLines::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(QuotationGoodsLines::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(QuotationGoodsLines::UpdatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(QuotationGoodsLines::QuotationId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(QuotationGoodsLines::ProductId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(QuotationGoodsLines::Rate)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(QuotationGoodsLines::Quantity)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(QuotationGoodsLines::VariableValues)
                            .text()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(QuotationGoodsLines::PreTaxAmount)
                            .decimal_len(19, 6)
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(QuotationGoodsLines::Remarks).text())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_quotation_goods_lines_quotation_id")
                            .from(QuotationGoodsLines::Table, QuotationGoodsLines::QuotationId)
                            .to(Quotations::Table, Quotations::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(QuotationServiceLines::Table)
                    .col(
                        ColumnDef::new(QuotationServiceLines::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(QuotationServiceLines::CreatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(QuotationServiceLines::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(QuotationServiceLines::QuotationId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(QuotationServiceLines::ServiceId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(QuotationServiceLines::Rate)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(QuotationServiceLines::Quantity)
                            .decimal_len(19, 6)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(QuotationServiceLines::VariableValues)
                            .text()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(QuotationServiceLines::PreTaxAmount)
                            .decimal_len(19, 6)
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(QuotationServiceLines::Remarks).text())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_quotation_service_lines_quotation_id")
                            .from(
                                QuotationServiceLines::Table,
                                QuotationServiceLines::QuotationId,
                            )
                            .to(Quotations::Table, Quotations::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(QuotationGoodsLineTaxes::Table)
                    .col(
                        ColumnDef::new(QuotationGoodsLineTaxes::LineId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(QuotationGoodsLineTaxes::TaxId)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(QuotationGoodsLineTaxes::LineId)
                            .col(QuotationGoodsLineTaxes::TaxId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_quotation_goods_line_taxes_line_id")
                            .from(
                                QuotationGoodsLineTaxes::Table,
                                QuotationGoodsLineTaxes::LineId,
                            )
                            .to(QuotationGoodsLines::Table, QuotationGoodsLines::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(QuotationServiceLineTaxes::Table)
                    .col(
                        ColumnDef::new(QuotationServiceLineTaxes::LineId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(QuotationServiceLineTaxes::TaxId)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(QuotationServiceLineTaxes::LineId)
                            .col(QuotationServiceLineTaxes::TaxId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_quotation_service_line_taxes_line_id")
                            .from(
                                QuotationServiceLineTaxes::Table,
                                QuotationServiceLineTaxes::LineId,
                            )
                            .to(QuotationServiceLines::Table, QuotationServiceLines::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(QuotationPreferences::Table)
                    .col(
                        ColumnDef::new(QuotationPreferences::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(QuotationPreferences::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(QuotationPreferences::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(QuotationPreferences::QuotationNumberFormat).text())
                    .col(ColumnDef::new(QuotationPreferences::QuotationPdfTemplate).text())
                    .col(ColumnDef::new(QuotationPreferences::QuotationEmailSubject).text())
                    .col(ColumnDef::new(QuotationPreferences::QuotationEmailBody).text())
                    .col(ColumnDef::new(QuotationPreferences::CompanyName).text())
                    .col(ColumnDef::new(QuotationPreferences::CompanyAddress).text())
                    .col(ColumnDef::new(QuotationPreferences::CompanyPhone).text())
                    .col(ColumnDef::new(QuotationPreferences::CompanyGstin).text())
                    .col(ColumnDef::new(QuotationPreferences::PlaceOfSupply).text())
                    .col(ColumnDef::new(QuotationPreferences::DefaultBankAccount).text())
                    .col(ColumnDef::new(QuotationPreferences::CompanyLogoVnodeId).big_integer())
                    .col(
                        ColumnDef::new(QuotationPreferences::CompanySignatureVnodeId).big_integer(),
                    )
                    .to_owned(),
            )
            .await?;

        migration_sql::ensure_schema_foreign_keys(
            manager,
            &[
                FK_QUOTATIONS_CUSTOMER,
                FK_QUOTATION_GOODS_LINE_PRODUCT,
                FK_QUOTATION_SERVICE_LINE_SERVICE,
                FK_QUOTATION_GOODS_LINE_TAX,
                FK_QUOTATION_SERVICE_LINE_TAX,
            ],
        )
        .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "INSERT INTO quotation_preferences (id, quotation_number_format) \
                 SELECT 1, 'QT-{{YYYY}}-{{POSTED_SEQ}}' \
                 WHERE NOT EXISTS (SELECT 1 FROM quotation_preferences WHERE id = 1)",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(QuotationServiceLineTaxes::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(QuotationGoodsLineTaxes::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(QuotationServiceLines::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(QuotationGoodsLines::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(QuotationPreferences::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Quotations::Table).to_owned())
            .await?;
        Ok(())
    }
}
