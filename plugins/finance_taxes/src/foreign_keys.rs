//! Foreign keys owned by taxes. Journal and account targets belong to other
//! plugins, so [`DEFERRED_FOREIGN_KEYS`] is retried after every installed plugin
//! has migrated.

use lariv_core::db::migration_sql::{ForeignKey, fk_restrict};

pub const FK_TAX_JOURNAL_ITEM_JOURNAL: ForeignKey = fk_restrict(
    "fk_tax_journal_entry_item_templates_journal_id",
    "tax_journal_entry_item_templates",
    "journal_id",
    "journals",
);
pub const FK_TAX_JOURNAL_ITEM_ACCOUNT: ForeignKey = fk_restrict(
    "fk_tax_journal_entry_item_templates_account_id",
    "tax_journal_entry_item_templates",
    "account_id",
    "accounts",
);
pub const FK_TAX_PREFERENCE_JOURNAL_JOURNAL: ForeignKey = fk_restrict(
    "fk_tax_preference_journal_items_journal_id",
    "tax_preference_journal_items",
    "journal_id",
    "journals",
);
pub const FK_TAX_PREFERENCE_JOURNAL_ACCOUNT: ForeignKey = fk_restrict(
    "fk_tax_preference_journal_items_account_id",
    "tax_preference_journal_items",
    "account_id",
    "accounts",
);
pub const FK_TAXES_ACCOUNT: ForeignKey =
    fk_restrict("fk_taxes_account_id", "taxes", "account_id", "accounts");

pub const DEFERRED_FOREIGN_KEYS: &[ForeignKey] = &[
    FK_TAX_JOURNAL_ITEM_JOURNAL,
    FK_TAX_JOURNAL_ITEM_ACCOUNT,
    FK_TAX_PREFERENCE_JOURNAL_JOURNAL,
    FK_TAX_PREFERENCE_JOURNAL_ACCOUNT,
    FK_TAXES_ACCOUNT,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deferred_foreign_keys_are_unique_identifiers() {
        assert!(lariv_core::db::migration_sql::foreign_keys_are_unique(
            DEFERRED_FOREIGN_KEYS
        ));
    }
}
