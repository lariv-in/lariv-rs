use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum UserSignatures {
    Table,
    UserId,
    CreatedAt,
    UpdatedAt,
    KeyRef,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserSignatures::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserSignatures::UserId)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserSignatures::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(UserSignatures::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(UserSignatures::KeyRef).text().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_signatures_user_id")
                            .from(UserSignatures::Table, UserSignatures::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserSignatures::Table).to_owned())
            .await
    }
}
