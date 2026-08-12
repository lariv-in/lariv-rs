use sea_orm::Statement;
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

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute(manager, DROP_TRIGGER).await?;
        execute(manager, CREATE_TRIGGER).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute(manager, DROP_TRIGGER).await?;
        execute(manager, CREATE_TRIGGER).await
    }
}
