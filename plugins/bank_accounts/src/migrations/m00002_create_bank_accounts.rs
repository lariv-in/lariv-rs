//! User-managed bank accounts. `ifsc_code` references `banks.ifsc`.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_POSTGRES: &str = r#"
CREATE TABLE IF NOT EXISTS bank_accounts (
    id BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ,
    name TEXT COLLATE "C" NOT NULL,
    account_number TEXT COLLATE "C" NOT NULL,
    ifsc_code TEXT COLLATE "C" NOT NULL,
    micr_code TEXT COLLATE "C",
    CONSTRAINT fk_bank_accounts_ifsc_code
        FOREIGN KEY (ifsc_code) REFERENCES banks (ifsc)
        ON DELETE RESTRICT ON UPDATE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_bank_accounts_ifsc_code ON bank_accounts (ifsc_code);
"#;

const DOWN_POSTGRES: &str = r#"
DROP TABLE IF EXISTS bank_accounts;
"#;

#[derive(DeriveIden)]
enum BankAccounts {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Name,
    AccountNumber,
    IfscCode,
    MicrCode,
}

#[derive(DeriveIden)]
enum Banks {
    Table,
    Ifsc,
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
                            .table(BankAccounts::Table)
                            .if_not_exists()
                            .col(
                                ColumnDef::new(BankAccounts::Id)
                                    .big_integer()
                                    .not_null()
                                    .auto_increment()
                                    .primary_key(),
                            )
                            .col(ColumnDef::new(BankAccounts::CreatedAt).timestamp_with_time_zone())
                            .col(ColumnDef::new(BankAccounts::UpdatedAt).timestamp_with_time_zone())
                            .col(ColumnDef::new(BankAccounts::Name).text().not_null())
                            .col(
                                ColumnDef::new(BankAccounts::AccountNumber)
                                    .text()
                                    .not_null(),
                            )
                            .col(ColumnDef::new(BankAccounts::IfscCode).text().not_null())
                            .col(ColumnDef::new(BankAccounts::MicrCode).text())
                            .foreign_key(
                                ForeignKey::create()
                                    .name("fk_bank_accounts_ifsc_code")
                                    .from(BankAccounts::Table, BankAccounts::IfscCode)
                                    .to(Banks::Table, Banks::Ifsc)
                                    .on_delete(ForeignKeyAction::Restrict)
                                    .on_update(ForeignKeyAction::Cascade),
                            )
                            .to_owned(),
                    )
                    .await?;
                manager
                    .create_index(
                        Index::create()
                            .name("idx_bank_accounts_ifsc_code")
                            .table(BankAccounts::Table)
                            .col(BankAccounts::IfscCode)
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
                    .drop_table(Table::drop().table(BankAccounts::Table).to_owned())
                    .await
            }
        }
    }
}
