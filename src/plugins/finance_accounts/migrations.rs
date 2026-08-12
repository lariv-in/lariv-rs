use sea_orm_migration::prelude::*;

mod m00001_create_finance_accounts;
mod m00002_seed_chart_of_accounts;
mod m00003_accounts_parent_balance_type;
mod m00004_create_currencies;
mod m00005_seed_currencies;
mod m00006_create_journals;
mod m00007_create_source_docs;
mod m00008_create_journal_entries;
mod m00009_create_journal_entry_items;
mod m00010_create_accounting_preferences;
mod m00011_accounting_preferences_default_journal;
mod m00012_accounts_balance_type_trigger_procedure;
mod m00013_accounting_preferences_invoice_pdf_template;
mod m00014_remove_accounting_preferences_default_journal;
mod m00015_journal_type_credit_debit;
mod m00016_accounts_drop_deleted_at;
mod m00017_journals_is_mutable;

use super::FinanceAccountsTag;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_finance_accounts::Migration),
            Box::new(m00002_seed_chart_of_accounts::Migration),
            Box::new(m00003_accounts_parent_balance_type::Migration),
            Box::new(m00004_create_currencies::Migration),
            Box::new(m00005_seed_currencies::Migration),
            Box::new(m00006_create_journals::Migration),
            Box::new(m00007_create_source_docs::Migration),
            Box::new(m00008_create_journal_entries::Migration),
            Box::new(m00009_create_journal_entry_items::Migration),
            Box::new(m00010_create_accounting_preferences::Migration),
            Box::new(m00011_accounting_preferences_default_journal::Migration),
            Box::new(m00012_accounts_balance_type_trigger_procedure::Migration),
            Box::new(m00013_accounting_preferences_invoice_pdf_template::Migration),
            Box::new(m00014_remove_accounting_preferences_default_journal::Migration),
            Box::new(m00015_journal_type_credit_debit::Migration),
            Box::new(m00016_accounts_drop_deleted_at::Migration),
            Box::new(m00017_journals_is_mutable::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: FinanceAccountsTag;
    migrator: Migrator;
}
