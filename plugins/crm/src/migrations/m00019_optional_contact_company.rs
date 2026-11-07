use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_connection().get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                exec_sql(
                    manager,
                    "ALTER TABLE crm_contacts DROP CONSTRAINT IF EXISTS fk_crm_contacts_company_id",
                )
                .await?;
                exec_sql(
                    manager,
                    "ALTER TABLE crm_contacts ALTER COLUMN company_id DROP NOT NULL",
                )
                .await?;
                exec_sql(
                    manager,
                    "ALTER TABLE crm_contacts ADD CONSTRAINT fk_crm_contacts_company_id \
                     FOREIGN KEY (company_id) REFERENCES crm_companies(id) \
                     ON DELETE SET NULL ON UPDATE CASCADE",
                )
                .await
            }
            sea_orm::DatabaseBackend::Sqlite => {
                // SQLite cannot DROP NOT NULL. Rebuild with a nullable company_id.
                // `m00012` has already renamed `first_name` to `name` and dropped `last_name`.
                exec_sql(manager, "PRAGMA foreign_keys = OFF").await?;
                exec_sql(
                    manager,
                    r#"
CREATE TABLE "crm_contacts__company" (
    "id" integer NOT NULL PRIMARY KEY AUTOINCREMENT,
    "created_at" timestamp_with_timezone_text,
    "updated_at" timestamp_with_timezone_text,
    "company_id" integer,
    "name" text NOT NULL,
    "email" text,
    "phone" text,
    "is_primary" boolean NOT NULL DEFAULT FALSE,
    FOREIGN KEY ("company_id") REFERENCES "crm_companies" ("id") ON DELETE SET NULL ON UPDATE CASCADE
)"#,
                )
                .await?;
                exec_sql(
                    manager,
                    r#"
INSERT INTO "crm_contacts__company" (
    id, created_at, updated_at, company_id, name, email, phone, is_primary
)
SELECT id, created_at, updated_at, company_id, name, email, phone, is_primary
FROM crm_contacts"#,
                )
                .await?;
                exec_sql(manager, r#"DROP TABLE "crm_contacts""#).await?;
                exec_sql(
                    manager,
                    r#"ALTER TABLE "crm_contacts__company" RENAME TO "crm_contacts""#,
                )
                .await?;
                exec_sql(
                    manager,
                    "CREATE INDEX IF NOT EXISTS idx_crm_contacts_company_id ON crm_contacts (company_id)",
                )
                .await?;
                exec_sql(manager, "PRAGMA foreign_keys = ON").await
            }
            _ => Ok(()),
        }
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_connection().get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                exec_sql(
                    manager,
                    "UPDATE crm_contacts SET company_id = (
                         SELECT id FROM crm_companies ORDER BY id LIMIT 1
                     ) WHERE company_id IS NULL",
                )
                .await?;
                exec_sql(
                    manager,
                    "ALTER TABLE crm_contacts DROP CONSTRAINT IF EXISTS fk_crm_contacts_company_id",
                )
                .await?;
                exec_sql(
                    manager,
                    "ALTER TABLE crm_contacts ALTER COLUMN company_id SET NOT NULL",
                )
                .await?;
                exec_sql(
                    manager,
                    "ALTER TABLE crm_contacts ADD CONSTRAINT fk_crm_contacts_company_id \
                     FOREIGN KEY (company_id) REFERENCES crm_companies(id) \
                     ON DELETE CASCADE ON UPDATE CASCADE",
                )
                .await
            }
            sea_orm::DatabaseBackend::Sqlite => Ok(()),
            _ => Ok(()),
        }
    }
}
