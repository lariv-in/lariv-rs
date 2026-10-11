use lariv_core::db::migration_sql::is_postgres;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Tasks {
    Table,
    Id,
    ParentId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::ParentId).big_integer().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_tasks_parent_id")
                    .table(Tasks::Table)
                    .col(Tasks::ParentId)
                    .to_owned(),
            )
            .await?;

        // SQLite cannot add a foreign key to an existing table.
        if is_postgres(manager) {
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .name("fk_tasks_parent_id")
                        .from(Tasks::Table, Tasks::ParentId)
                        .to(Tasks::Table, Tasks::Id)
                        .on_delete(ForeignKeyAction::SetNull)
                        .on_update(ForeignKeyAction::Cascade)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if is_postgres(manager) {
            manager
                .drop_foreign_key(
                    ForeignKey::drop()
                        .name("fk_tasks_parent_id")
                        .table(Tasks::Table)
                        .to_owned(),
                )
                .await?;
        }
        manager
            .drop_index(
                Index::drop()
                    .name("idx_tasks_parent_id")
                    .table(Tasks::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::ParentId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
