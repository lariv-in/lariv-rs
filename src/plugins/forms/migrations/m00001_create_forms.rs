use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Forms::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Forms::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Forms::CreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Forms::UpdatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Forms::Title).text().not_null())
                    .col(ColumnDef::new(Forms::Questions).json_binary().not_null())
                    .col(ColumnDef::new(Forms::CreatedById).big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_forms_created_by_id")
                            .from(Forms::Table, Forms::CreatedById)
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
                    .name("idx_forms_created_by_id")
                    .table(Forms::Table)
                    .col(Forms::CreatedById)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(FormResponses::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FormResponses::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(FormResponses::FormId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FormResponses::Answers)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FormResponses::SubmittedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(FormResponses::Name).text())
                    .col(ColumnDef::new(FormResponses::Email).text())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_form_responses_form_id")
                            .from(FormResponses::Table, FormResponses::FormId)
                            .to(Forms::Table, Forms::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_form_responses_form_id")
                    .table(FormResponses::Table)
                    .col(FormResponses::FormId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FormResponses::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Forms::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Forms {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Title,
    Questions,
    CreatedById,
}

#[derive(Iden)]
enum FormResponses {
    Table,
    Id,
    FormId,
    Answers,
    SubmittedAt,
    Name,
    Email,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}
