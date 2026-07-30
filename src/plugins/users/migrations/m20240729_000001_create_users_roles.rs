use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Roles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Roles::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Roles::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Roles::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Roles::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Roles::Name).text().unique_key())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_roles_deleted_at")
                    .table(Roles::Table)
                    .col(Roles::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Users::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Users::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Users::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Users::Name).text().not_null())
                    .col(ColumnDef::new(Users::Email).text().unique_key())
                    .col(ColumnDef::new(Users::Phone).text().unique_key())
                    .col(
                        ColumnDef::new(Users::IsSuperuser)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Users::RoleId).big_integer().not_null())
                    .col(ColumnDef::new(Users::Password).binary())
                    .col(ColumnDef::new(Users::PasswordSalt).binary())
                    .col(
                        ColumnDef::new(Users::Timezone)
                            .text()
                            .default("Asia/Kolkata"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_users_role_id")
                            .from(Users::Table, Users::RoleId)
                            .to(Roles::Table, Roles::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_users_deleted_at")
                    .table(Users::Table)
                    .col(Users::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_users_role_id")
                    .table(Users::Table)
                    .col(Users::RoleId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Roles::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Roles {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Name,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Name,
    Email,
    Phone,
    IsSuperuser,
    RoleId,
    Password,
    PasswordSalt,
    Timezone,
}
