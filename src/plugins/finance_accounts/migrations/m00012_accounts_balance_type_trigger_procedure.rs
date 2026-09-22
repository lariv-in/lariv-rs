use crate::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const DROP_TRIGGER: &str =
    "DROP TRIGGER IF EXISTS accounts_enforce_parent_balance_type_biud ON accounts";

const CREATE_TRIGGER: &str = r#"
CREATE TRIGGER accounts_enforce_parent_balance_type_biud
  BEFORE INSERT OR UPDATE ON accounts
  FOR EACH ROW EXECUTE PROCEDURE accounts_enforce_parent_balance_type()
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, DROP_TRIGGER).await?;
        exec_sql(manager, CREATE_TRIGGER).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_sql(manager, DROP_TRIGGER).await?;
        exec_sql(manager, CREATE_TRIGGER).await
    }
}
