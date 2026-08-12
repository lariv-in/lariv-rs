use sea_orm::Statement;
use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Name,
    Code,
    IsGroup,
    BalanceType,
    ParentId,
}

#[derive(DeriveIden)]
enum BalanceTypeKind {
    #[sea_orm(iden = "balance_type")]
    Enum,
    #[sea_orm(iden = "Credit")]
    Credit,
    #[sea_orm(iden = "Debit")]
    Debit,
}

/// Go migrations created `"BalanceType"`; Rust uses `balance_type`. Convert legacy columns
/// before seed inserts cast to `balance_type`.
const NORMALIZE_BALANCE_TYPE: &str = r#"
DO $$
BEGIN
  IF EXISTS (
    SELECT 1
    FROM pg_attribute a
    JOIN pg_class c ON a.attrelid = c.oid
    JOIN pg_type t ON a.atttypid = t.oid
    JOIN pg_namespace n ON c.relnamespace = n.oid
    WHERE n.nspname = current_schema()
      AND c.relname = 'accounts'
      AND a.attname = 'balance_type'
      AND NOT a.attisdropped
      AND t.typname = 'BalanceType'
  ) THEN
    ALTER TABLE accounts ALTER COLUMN balance_type TYPE TEXT USING balance_type::TEXT;
    DROP TYPE IF EXISTS "BalanceType";
    ALTER TABLE accounts
      ALTER COLUMN balance_type TYPE balance_type USING balance_type::balance_type;
  END IF;
END;
$$;
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
        manager
            .create_type(
                Type::create()
                    .as_enum(BalanceTypeKind::Enum)
                    .values([BalanceTypeKind::Credit, BalanceTypeKind::Debit])
                    .to_owned(),
            )
            .await?;

        execute(manager, NORMALIZE_BALANCE_TYPE).await?;

        manager
            .create_table(
                Table::create()
                    .table(Accounts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Accounts::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Accounts::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Accounts::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Accounts::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Accounts::Name).text().not_null())
                    .col(ColumnDef::new(Accounts::Code).integer().not_null())
                    .col(
                        ColumnDef::new(Accounts::IsGroup)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Accounts::BalanceType)
                            .custom(BalanceTypeKind::Enum)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Accounts::ParentId).big_integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_accounts_parent_id")
                            .from(Accounts::Table, Accounts::ParentId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_accounts_deleted_at")
                    .table(Accounts::Table)
                    .col(Accounts::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_accounts_parent_id")
                    .table(Accounts::Table)
                    .col(Accounts::ParentId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Accounts::Table).to_owned())
            .await?;
        manager
            .drop_type(Type::drop().name(BalanceTypeKind::Enum).to_owned())
            .await
    }
}
