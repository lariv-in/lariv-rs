use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum HrPreferences {
    Table,
    IdCardSigningKey,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(HrPreferences::Table)
                    .add_column(ColumnDef::new(HrPreferences::IdCardSigningKey).text())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(HrPreferences::Table)
                    .drop_column(HrPreferences::IdCardSigningKey)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
