//! Foreign keys owned by the products catalog, including keys whose target table
//! belongs to another plugin. [`DEFERRED_FOREIGN_KEYS`] is retried after every
//! installed plugin has migrated.

use lariv_core::db::migration_sql::{ForeignKey, fk_restrict};

pub const FK_PRODUCTS_STOCK: ForeignKey = fk_restrict(
    "fk_products_stock_id",
    "products",
    "stock_id",
    "inventory_stocks",
);
pub const FK_PRODUCT_JOURNAL_ENTRY_TEMPLATES_JOURNAL: ForeignKey = fk_restrict(
    "fk_product_journal_entry_templates_journal_id",
    "product_journal_entry_templates",
    "journal_id",
    "journals",
);
pub const FK_PRODUCT_JOURNAL_ITEM_ACCOUNT: ForeignKey = fk_restrict(
    "fk_product_journal_entry_item_templates_account_id",
    "product_journal_entry_item_templates",
    "account_id",
    "accounts",
);
pub const FK_PRODUCT_JOURNAL_ITEM_JOURNAL: ForeignKey = fk_restrict(
    "fk_product_journal_entry_item_templates_journal_id",
    "product_journal_entry_item_templates",
    "journal_id",
    "journals",
);
pub const FK_PRODUCT_PREFERENCE_JOURNAL_JOURNAL: ForeignKey = fk_restrict(
    "fk_product_preference_journal_items_journal_id",
    "product_preference_journal_items",
    "journal_id",
    "journals",
);
pub const FK_PRODUCT_PREFERENCE_JOURNAL_ACCOUNT: ForeignKey = fk_restrict(
    "fk_product_preference_journal_items_account_id",
    "product_preference_journal_items",
    "account_id",
    "accounts",
);
pub const FK_PRODUCT_PREFERENCES_INVENTORY_ACCOUNT: ForeignKey = fk_restrict(
    "fk_product_preferences_inventory_account_id",
    "product_preferences",
    "inventory_account_id",
    "accounts",
);
pub const FK_PRODUCT_PREFERENCES_COST_ACCOUNT: ForeignKey = fk_restrict(
    "fk_product_preferences_cost_of_sales_account_id",
    "product_preferences",
    "cost_of_sales_account_id",
    "accounts",
);

pub const FK_SERVICE_TAX: ForeignKey = fk_restrict(
    "fk_service_taxes_tax_id",
    "service_taxes",
    "tax_id",
    "taxes",
);
pub const FK_SERVICE_PREFERENCE_TAX: ForeignKey = fk_restrict(
    "fk_service_preferences_taxes_tax_id",
    "service_preferences_taxes",
    "tax_id",
    "taxes",
);
pub const FK_SERVICE_PREFERENCES_COST_ACCOUNT: ForeignKey = fk_restrict(
    "fk_service_preferences_cost_of_sales_account_id",
    "service_preferences",
    "cost_of_sales_account_id",
    "accounts",
);
pub const FK_SERVICE_PREFERENCES_CREDIT_ACCOUNT: ForeignKey = fk_restrict(
    "fk_service_preferences_cost_credit_account_id",
    "service_preferences",
    "cost_credit_account_id",
    "accounts",
);
pub const FK_SERVICE_JOURNAL_ITEM_JOURNAL: ForeignKey = fk_restrict(
    "fk_service_journal_entry_item_templates_journal_id",
    "service_journal_entry_item_templates",
    "journal_id",
    "journals",
);
pub const FK_SERVICE_JOURNAL_ITEM_ACCOUNT: ForeignKey = fk_restrict(
    "fk_service_journal_entry_item_templates_account_id",
    "service_journal_entry_item_templates",
    "account_id",
    "accounts",
);
pub const FK_SERVICE_PREFERENCE_JOURNAL_JOURNAL: ForeignKey = fk_restrict(
    "fk_service_preference_journal_items_journal_id",
    "service_preference_journal_items",
    "journal_id",
    "journals",
);
pub const FK_SERVICE_PREFERENCE_JOURNAL_ACCOUNT: ForeignKey = fk_restrict(
    "fk_service_preference_journal_items_account_id",
    "service_preference_journal_items",
    "account_id",
    "accounts",
);

/// Retried after the composite migration pass. `service_*` tables are dropped by
/// the invoices catalog unify; the retry no-ops once those tables are gone.
pub const DEFERRED_FOREIGN_KEYS: &[ForeignKey] = &[
    FK_PRODUCTS_STOCK,
    FK_PRODUCT_JOURNAL_ENTRY_TEMPLATES_JOURNAL,
    FK_PRODUCT_JOURNAL_ITEM_ACCOUNT,
    FK_PRODUCT_JOURNAL_ITEM_JOURNAL,
    FK_PRODUCT_PREFERENCE_JOURNAL_JOURNAL,
    FK_PRODUCT_PREFERENCE_JOURNAL_ACCOUNT,
    FK_PRODUCT_PREFERENCES_INVENTORY_ACCOUNT,
    FK_PRODUCT_PREFERENCES_COST_ACCOUNT,
    FK_SERVICE_TAX,
    FK_SERVICE_PREFERENCE_TAX,
    FK_SERVICE_PREFERENCES_COST_ACCOUNT,
    FK_SERVICE_PREFERENCES_CREDIT_ACCOUNT,
    FK_SERVICE_JOURNAL_ITEM_JOURNAL,
    FK_SERVICE_JOURNAL_ITEM_ACCOUNT,
    FK_SERVICE_PREFERENCE_JOURNAL_JOURNAL,
    FK_SERVICE_PREFERENCE_JOURNAL_ACCOUNT,
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
