use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Todos::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Todos::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Todos::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Todos::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Todos::UserId).big_integer().not_null())
                    .col(
                        ColumnDef::new(Todos::Checked)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Todos::Title).text().not_null())
                    .col(ColumnDef::new(Todos::Description).text().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_todos_user_id")
                            .from(Todos::Table, Todos::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_todos_user_id")
                    .table(Todos::Table)
                    .col(Todos::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Todos::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Todos {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    UserId,
    Checked,
    Title,
    Description,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}
