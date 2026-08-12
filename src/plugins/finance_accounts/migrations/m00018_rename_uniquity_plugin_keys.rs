use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn execute(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute_unprepared(sql)
        .await
        .map(|_| ())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_connection().get_database_backend() != sea_orm::DatabaseBackend::Postgres {
            return Ok(());
        }
        execute(
            manager,
            r#"
            UPDATE source_docs
            SET source_doc_type = replace(source_doc_type, 'p_uniquity_finance_', 'p_finance_')
            WHERE source_doc_type LIKE 'p_uniquity_finance_%';

            UPDATE draft_invoices
            SET payment_term_type = replace(payment_term_type, 'p_uniquity_finance_', 'p_finance_')
            WHERE payment_term_type LIKE 'p_uniquity_finance_%';

            UPDATE posted_invoices
            SET payment_term_type = replace(payment_term_type, 'p_uniquity_finance_', 'p_finance_')
            WHERE payment_term_type LIKE 'p_uniquity_finance_%';

            UPDATE cancelled_invoices
            SET payment_term_type = replace(payment_term_type, 'p_uniquity_finance_', 'p_finance_')
            WHERE payment_term_type LIKE 'p_uniquity_finance_%';

            UPDATE payment_terms
            SET type = replace(type, 'p_uniquity_finance_', 'p_finance_')
            WHERE type LIKE 'p_uniquity_finance_%';
            "#,
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_connection().get_database_backend() != sea_orm::DatabaseBackend::Postgres {
            return Ok(());
        }
        execute(
            manager,
            r#"
            UPDATE source_docs
            SET source_doc_type = replace(source_doc_type, 'p_finance_', 'p_uniquity_finance_')
            WHERE source_doc_type LIKE 'p_finance_%';

            UPDATE draft_invoices
            SET payment_term_type = replace(payment_term_type, 'p_finance_', 'p_uniquity_finance_')
            WHERE payment_term_type LIKE 'p_finance_%';

            UPDATE posted_invoices
            SET payment_term_type = replace(payment_term_type, 'p_finance_', 'p_uniquity_finance_')
            WHERE payment_term_type LIKE 'p_finance_%';

            UPDATE cancelled_invoices
            SET payment_term_type = replace(payment_term_type, 'p_finance_', 'p_uniquity_finance_')
            WHERE payment_term_type LIKE 'p_finance_%';

            UPDATE payment_terms
            SET type = replace(type, 'p_finance_', 'p_uniquity_finance_')
            WHERE type LIKE 'p_finance_%';
            "#,
        )
        .await
    }
}
