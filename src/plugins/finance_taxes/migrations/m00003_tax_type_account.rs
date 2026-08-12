use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Taxes {
    Table,
    TaxType,
    AccountId,
}

#[derive(DeriveIden)]
enum TaxKind {
    #[sea_orm(iden = "tax_kind")]
    Enum,
    #[sea_orm(iden = "levied")]
    Levied,
    #[sea_orm(iden = "withholding")]
    Withholding,
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
            .create_type(
                Type::create()
                    .as_enum(TaxKind::Enum)
                    .values([TaxKind::Levied, TaxKind::Withholding])
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Taxes::Table)
                    .add_column(
                        ColumnDef::new(Taxes::TaxType)
                            .custom(TaxKind::Enum)
                            .not_null()
                            .default("levied"),
                    )
                    .add_column(ColumnDef::new(Taxes::AccountId).big_integer())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_taxes_account_id")
                    .table(Taxes::Table)
                    .col(Taxes::AccountId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Taxes::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_taxes_account_id")
                            .from_tbl(Taxes::Table)
                            .from_col(Taxes::AccountId)
                            .to_tbl(Accounts::Table)
                            .to_col(Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_taxes_account_id")
                    .table(Taxes::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Taxes::Table)
                    .drop_foreign_key(Alias::new("fk_taxes_account_id"))
                    .drop_column(Taxes::AccountId)
                    .drop_column(Taxes::TaxType)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_type(Type::drop().name(TaxKind::Enum).to_owned())
            .await
    }
}
