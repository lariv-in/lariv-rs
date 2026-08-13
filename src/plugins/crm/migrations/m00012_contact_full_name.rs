use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum CrmContacts {
    Table,
    FirstName,
    LastName,
    Name,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                UPDATE crm_contacts
                SET first_name = TRIM(first_name || ' ' || COALESCE(last_name, ''))
                "#,
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CrmContacts::Table)
                    .rename_column(CrmContacts::FirstName, CrmContacts::Name)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CrmContacts::Table)
                    .drop_column(CrmContacts::LastName)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CrmContacts::Table)
                    .rename_column(CrmContacts::Name, CrmContacts::FirstName)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CrmContacts::Table)
                    .add_column(ColumnDef::new(CrmContacts::LastName).text())
                    .to_owned(),
            )
            .await
    }
}
