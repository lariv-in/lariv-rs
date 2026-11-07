use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

const UP_POSTGRES: &str = r#"
DO $$
BEGIN
    IF to_regclass('public.crm_converted_leads') IS NOT NULL THEN
        ALTER TABLE crm_converted_leads DROP CONSTRAINT IF EXISTS fk_crm_converted_leads_deal_id;
        ALTER TABLE crm_converted_leads DROP COLUMN IF EXISTS deal_id;
    END IF;
    DROP TABLE IF EXISTS crm_deals;
END $$;
"#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_connection().get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            return exec_sql(manager, UP_POSTGRES).await;
        }

        // `deal_id` is declared in a foreign key, so SQLite will not drop the column.
        exec_sql(manager, "PRAGMA foreign_keys = OFF").await?;
        exec_sql(
            manager,
            r#"
CREATE TABLE "crm_converted_leads__nodeal" (
    "id" integer NOT NULL PRIMARY KEY AUTOINCREMENT,
    "created_at" timestamp_with_timezone_text,
    "lead_id" integer NOT NULL,
    "converted_at" timestamp_with_timezone_text NOT NULL,
    "company_id" integer NOT NULL,
    "contact_id" integer NOT NULL,
    FOREIGN KEY ("lead_id") REFERENCES "crm_leads" ("id") ON DELETE RESTRICT ON UPDATE CASCADE,
    FOREIGN KEY ("company_id") REFERENCES "crm_companies" ("id") ON DELETE RESTRICT ON UPDATE CASCADE,
    FOREIGN KEY ("contact_id") REFERENCES "crm_contacts" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
)"#,
        )
        .await?;
        exec_sql(
            manager,
            r#"
INSERT INTO "crm_converted_leads__nodeal" (
    id, created_at, lead_id, converted_at, company_id, contact_id
)
SELECT id, created_at, lead_id, converted_at, company_id, contact_id
FROM crm_converted_leads"#,
        )
        .await?;
        exec_sql(manager, r#"DROP TABLE "crm_converted_leads""#).await?;
        exec_sql(
            manager,
            r#"ALTER TABLE "crm_converted_leads__nodeal" RENAME TO "crm_converted_leads""#,
        )
        .await?;
        exec_sql(manager, "DROP TABLE IF EXISTS crm_deals").await?;
        exec_sql(manager, "PRAGMA foreign_keys = ON").await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Irreversible: deals table and converted_lead.deal_id are removed.
        Ok(())
    }
}
