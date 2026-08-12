use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn execute(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(
            manager.get_connection().get_database_backend(),
            sql.to_string(),
        ))
        .await
        .map(|_| ())
}

const UP_POSTGRES: &str = r#"
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = 'crm_accounts'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = 'crm_companies'
  ) THEN
    ALTER TABLE crm_accounts RENAME TO crm_companies;
    ALTER TABLE crm_contacts RENAME COLUMN account_id TO company_id;
    ALTER TABLE crm_deals RENAME COLUMN account_id TO company_id;
    ALTER TABLE crm_converted_leads RENAME COLUMN account_id TO company_id;
    ALTER INDEX IF EXISTS uix_crm_accounts_customer_id RENAME TO uix_crm_companies_customer_id;
    ALTER INDEX IF EXISTS idx_crm_contacts_account_id RENAME TO idx_crm_contacts_company_id;
    ALTER INDEX IF EXISTS idx_crm_deals_account_id RENAME TO idx_crm_deals_company_id;
  END IF;
END $$;
"#;

const DOWN_POSTGRES: &str = r#"
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = 'crm_companies'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = 'crm_accounts'
  ) THEN
    ALTER INDEX IF EXISTS uix_crm_companies_customer_id RENAME TO uix_crm_accounts_customer_id;
    ALTER INDEX IF EXISTS idx_crm_contacts_company_id RENAME TO idx_crm_contacts_account_id;
    ALTER INDEX IF EXISTS idx_crm_deals_company_id RENAME TO idx_crm_deals_account_id;
    ALTER TABLE crm_converted_leads RENAME COLUMN company_id TO account_id;
    ALTER TABLE crm_deals RENAME COLUMN company_id TO account_id;
    ALTER TABLE crm_contacts RENAME COLUMN company_id TO account_id;
    ALTER TABLE crm_companies RENAME TO crm_accounts;
  END IF;
END $$;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_connection().get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => execute(manager, UP_POSTGRES).await,
            sea_orm::DatabaseBackend::Sqlite => {
                let has_accounts: bool = manager
                    .get_connection()
                    .query_one(Statement::from_string(
                        sea_orm::DatabaseBackend::Sqlite,
                        "SELECT 1 AS ok FROM sqlite_master WHERE type = 'table' AND name = 'crm_accounts'".to_string(),
                    ))
                    .await?
                    .is_some();
                let has_companies: bool = manager
                    .get_connection()
                    .query_one(Statement::from_string(
                        sea_orm::DatabaseBackend::Sqlite,
                        "SELECT 1 AS ok FROM sqlite_master WHERE type = 'table' AND name = 'crm_companies'".to_string(),
                    ))
                    .await?
                    .is_some();
                if has_accounts && !has_companies {
                    execute(manager, "ALTER TABLE crm_accounts RENAME TO crm_companies").await?;
                    execute(
                        manager,
                        "ALTER TABLE crm_contacts RENAME COLUMN account_id TO company_id",
                    )
                    .await?;
                    execute(
                        manager,
                        "ALTER TABLE crm_deals RENAME COLUMN account_id TO company_id",
                    )
                    .await?;
                    execute(
                        manager,
                        "ALTER TABLE crm_converted_leads RENAME COLUMN account_id TO company_id",
                    )
                    .await?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_connection().get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => execute(manager, DOWN_POSTGRES).await,
            sea_orm::DatabaseBackend::Sqlite => {
                let has_companies: bool = manager
                    .get_connection()
                    .query_one(Statement::from_string(
                        sea_orm::DatabaseBackend::Sqlite,
                        "SELECT 1 AS ok FROM sqlite_master WHERE type = 'table' AND name = 'crm_companies'".to_string(),
                    ))
                    .await?
                    .is_some();
                let has_accounts: bool = manager
                    .get_connection()
                    .query_one(Statement::from_string(
                        sea_orm::DatabaseBackend::Sqlite,
                        "SELECT 1 AS ok FROM sqlite_master WHERE type = 'table' AND name = 'crm_accounts'".to_string(),
                    ))
                    .await?
                    .is_some();
                if has_companies && !has_accounts {
                    execute(
                        manager,
                        "ALTER TABLE crm_converted_leads RENAME COLUMN company_id TO account_id",
                    )
                    .await?;
                    execute(
                        manager,
                        "ALTER TABLE crm_deals RENAME COLUMN company_id TO account_id",
                    )
                    .await?;
                    execute(
                        manager,
                        "ALTER TABLE crm_contacts RENAME COLUMN company_id TO account_id",
                    )
                    .await?;
                    execute(manager, "ALTER TABLE crm_companies RENAME TO crm_accounts").await?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
