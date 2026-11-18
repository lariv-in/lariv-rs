//! Branch-level IFSC directory (`banks`).
//!
//! Postgres string columns use `COLLATE "C"`. SQLite uses plain `TEXT`.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_POSTGRES: &str = r#"
CREATE TABLE IF NOT EXISTS banks (
    id BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ,
    bank_name TEXT COLLATE "C",
    bank_code TEXT COLLATE "C" NOT NULL,
    ifsc TEXT COLLATE "C" NOT NULL,
    branch_name TEXT COLLATE "C",
    center TEXT COLLATE "C",
    district TEXT COLLATE "C",
    state TEXT COLLATE "C",
    address TEXT COLLATE "C",
    contact TEXT COLLATE "C",
    imps BOOLEAN NOT NULL DEFAULT FALSE,
    rtgs BOOLEAN NOT NULL DEFAULT FALSE,
    city TEXT COLLATE "C",
    iso3166 TEXT COLLATE "C",
    neft BOOLEAN NOT NULL DEFAULT FALSE,
    micr TEXT COLLATE "C",
    upi BOOLEAN NOT NULL DEFAULT FALSE,
    swift TEXT COLLATE "C",
    ach_credit BOOLEAN NOT NULL DEFAULT FALSE,
    ach_debit BOOLEAN NOT NULL DEFAULT FALSE,
    apbs BOOLEAN NOT NULL DEFAULT FALSE,
    nach_debit BOOLEAN NOT NULL DEFAULT FALSE,
    bank_type TEXT COLLATE "C",
    CONSTRAINT banks_ifsc_key UNIQUE (ifsc)
);
CREATE INDEX IF NOT EXISTS idx_banks_ifsc ON banks (ifsc);
CREATE INDEX IF NOT EXISTS idx_banks_bank_code ON banks (bank_code);
"#;

const DOWN_POSTGRES: &str = r#"
DROP TABLE IF EXISTS banks;
"#;

#[derive(DeriveIden)]
enum Banks {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    BankName,
    BankCode,
    Ifsc,
    BranchName,
    Center,
    District,
    State,
    Address,
    Contact,
    Imps,
    Rtgs,
    City,
    Iso3166,
    Neft,
    Micr,
    Upi,
    Swift,
    AchCredit,
    AchDebit,
    Apbs,
    NachDebit,
    BankType,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_connection().get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => exec_sql(manager, UP_POSTGRES).await,
            _ => {
                manager
                    .create_table(
                        Table::create()
                            .table(Banks::Table)
                            .if_not_exists()
                            .col(
                                ColumnDef::new(Banks::Id)
                                    .big_integer()
                                    .not_null()
                                    .auto_increment()
                                    .primary_key(),
                            )
                            .col(ColumnDef::new(Banks::CreatedAt).timestamp_with_time_zone())
                            .col(ColumnDef::new(Banks::UpdatedAt).timestamp_with_time_zone())
                            .col(ColumnDef::new(Banks::BankName).text())
                            .col(ColumnDef::new(Banks::BankCode).text().not_null())
                            .col(ColumnDef::new(Banks::Ifsc).text().not_null().unique_key())
                            .col(ColumnDef::new(Banks::BranchName).text())
                            .col(ColumnDef::new(Banks::Center).text())
                            .col(ColumnDef::new(Banks::District).text())
                            .col(ColumnDef::new(Banks::State).text())
                            .col(ColumnDef::new(Banks::Address).text())
                            .col(ColumnDef::new(Banks::Contact).text())
                            .col(
                                ColumnDef::new(Banks::Imps)
                                    .boolean()
                                    .not_null()
                                    .default(false),
                            )
                            .col(
                                ColumnDef::new(Banks::Rtgs)
                                    .boolean()
                                    .not_null()
                                    .default(false),
                            )
                            .col(ColumnDef::new(Banks::City).text())
                            .col(ColumnDef::new(Banks::Iso3166).text())
                            .col(
                                ColumnDef::new(Banks::Neft)
                                    .boolean()
                                    .not_null()
                                    .default(false),
                            )
                            .col(ColumnDef::new(Banks::Micr).text())
                            .col(
                                ColumnDef::new(Banks::Upi)
                                    .boolean()
                                    .not_null()
                                    .default(false),
                            )
                            .col(ColumnDef::new(Banks::Swift).text())
                            .col(
                                ColumnDef::new(Banks::AchCredit)
                                    .boolean()
                                    .not_null()
                                    .default(false),
                            )
                            .col(
                                ColumnDef::new(Banks::AchDebit)
                                    .boolean()
                                    .not_null()
                                    .default(false),
                            )
                            .col(
                                ColumnDef::new(Banks::Apbs)
                                    .boolean()
                                    .not_null()
                                    .default(false),
                            )
                            .col(
                                ColumnDef::new(Banks::NachDebit)
                                    .boolean()
                                    .not_null()
                                    .default(false),
                            )
                            .col(ColumnDef::new(Banks::BankType).text())
                            .to_owned(),
                    )
                    .await?;
                manager
                    .create_index(
                        Index::create()
                            .name("idx_banks_ifsc")
                            .table(Banks::Table)
                            .col(Banks::Ifsc)
                            .if_not_exists()
                            .to_owned(),
                    )
                    .await?;
                manager
                    .create_index(
                        Index::create()
                            .name("idx_banks_bank_code")
                            .table(Banks::Table)
                            .col(Banks::BankCode)
                            .if_not_exists()
                            .to_owned(),
                    )
                    .await?;
                Ok(())
            }
        }
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_connection().get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => exec_sql(manager, DOWN_POSTGRES).await,
            _ => {
                manager
                    .drop_table(Table::drop().table(Banks::Table).to_owned())
                    .await
            }
        }
    }
}
