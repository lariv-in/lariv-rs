//! Foreign keys owned by purchases. Unified purchase-line keys are retried via
//! [`DEFERRED_FOREIGN_KEYS`]. Goods and service line keys are historical and stay
//! unregistered so a later migration can leave those tables behind.

use lariv_core::db::migration_sql::{ForeignKey, fk_restrict, fk_set_null};

pub const FK_POSTED_PURCHASE_SERVICE_LINE_JOURNAL_ITEM: ForeignKey = fk_restrict(
    "fk_posted_purchase_service_lines_journal_entry_item_id",
    "posted_purchase_service_lines",
    "journal_entry_item_id",
    "journal_entry_items",
);
pub const FK_CANCELLED_PURCHASE_SERVICE_LINE_JOURNAL_ITEM: ForeignKey = fk_restrict(
    "fk_cancelled_purchase_service_lines_journal_entry_item_id",
    "cancelled_purchase_service_lines",
    "journal_entry_item_id",
    "journal_entry_items",
);
pub const FK_DRAFT_PURCHASE_SERVICE_LINE_TAX: ForeignKey = fk_restrict(
    "fk_draft_purchase_service_line_taxes_tax_id",
    "draft_purchase_service_line_taxes",
    "tax_id",
    "taxes",
);
pub const FK_POSTED_PURCHASE_SERVICE_LINE_TAX: ForeignKey = fk_restrict(
    "fk_posted_purchase_service_line_taxes_tax_id",
    "posted_purchase_service_line_taxes",
    "tax_id",
    "taxes",
);
pub const FK_CANCELLED_PURCHASE_SERVICE_LINE_TAX: ForeignKey = fk_restrict(
    "fk_cancelled_purchase_service_line_taxes_tax_id",
    "cancelled_purchase_service_line_taxes",
    "tax_id",
    "taxes",
);
pub const FK_PURCHASE_PRODUCTS_STOCK: ForeignKey = fk_restrict(
    "fk_purchase_products_stock_id",
    "purchase_products",
    "stock_id",
    "inventory_stocks",
);
pub const FK_PURCHASE_PRODUCT_TAX: ForeignKey = fk_restrict(
    "fk_purchase_product_taxes_tax_id",
    "purchase_product_taxes",
    "tax_id",
    "taxes",
);
pub const FK_PURCHASE_PRODUCT_PREFERENCE_TAX: ForeignKey = fk_restrict(
    "fk_purchase_product_preferences_taxes_tax_id",
    "purchase_product_preferences_taxes",
    "tax_id",
    "taxes",
);
pub const FK_PURCHASE_PRODUCT_JOURNAL_JOURNAL: ForeignKey = fk_restrict(
    "fk_purchase_product_journal_entry_item_templates_journal_id",
    "purchase_product_journal_entry_item_templates",
    "journal_id",
    "journals",
);
pub const FK_PURCHASE_PRODUCT_JOURNAL_ACCOUNT: ForeignKey = fk_restrict(
    "fk_purchase_product_journal_entry_item_templates_account_id",
    "purchase_product_journal_entry_item_templates",
    "account_id",
    "accounts",
);
pub const FK_PURCHASE_SERVICE_TAX: ForeignKey = fk_restrict(
    "fk_purchase_service_taxes_tax_id",
    "purchase_service_taxes",
    "tax_id",
    "taxes",
);
pub const FK_PURCHASE_SERVICE_PREFERENCE_TAX: ForeignKey = fk_restrict(
    "fk_purchase_service_preferences_taxes_tax_id",
    "purchase_service_preferences_taxes",
    "tax_id",
    "taxes",
);
pub const FK_PURCHASE_SERVICE_JOURNAL_JOURNAL: ForeignKey = fk_restrict(
    "fk_purchase_service_journal_entry_item_templates_journal_id",
    "purchase_service_journal_entry_item_templates",
    "journal_id",
    "journals",
);
pub const FK_PURCHASE_SERVICE_JOURNAL_ACCOUNT: ForeignKey = fk_restrict(
    "fk_purchase_service_journal_entry_item_templates_account_id",
    "purchase_service_journal_entry_item_templates",
    "account_id",
    "accounts",
);
pub const FK_PURCHASE_PRODUCT_PREFERENCE_JOURNAL_JOURNAL: ForeignKey = fk_restrict(
    "fk_purchase_product_preference_journal_items_journal_id",
    "purchase_product_preference_journal_items",
    "journal_id",
    "journals",
);
pub const FK_PURCHASE_PRODUCT_PREFERENCE_JOURNAL_ACCOUNT: ForeignKey = fk_restrict(
    "fk_purchase_product_preference_journal_items_account_id",
    "purchase_product_preference_journal_items",
    "account_id",
    "accounts",
);
pub const FK_PURCHASE_SERVICE_PREFERENCE_JOURNAL_JOURNAL: ForeignKey = fk_restrict(
    "fk_purchase_service_preference_journal_items_journal_id",
    "purchase_service_preference_journal_items",
    "journal_id",
    "journals",
);
pub const FK_PURCHASE_SERVICE_PREFERENCE_JOURNAL_ACCOUNT: ForeignKey = fk_restrict(
    "fk_purchase_service_preference_journal_items_account_id",
    "purchase_service_preference_journal_items",
    "account_id",
    "accounts",
);
pub const FK_DRAFT_PURCHASE_GOODS_LINE_PRODUCT: ForeignKey = fk_restrict(
    "fk_draft_purchase_goods_lines_product_id",
    "draft_purchase_goods_lines",
    "product_id",
    "products",
);
pub const FK_POSTED_PURCHASE_GOODS_LINE_PRODUCT: ForeignKey = fk_restrict(
    "fk_posted_purchase_goods_lines_product_id",
    "posted_purchase_goods_lines",
    "product_id",
    "products",
);
pub const FK_CANCELLED_PURCHASE_GOODS_LINE_PRODUCT: ForeignKey = fk_restrict(
    "fk_cancelled_purchase_goods_lines_product_id",
    "cancelled_purchase_goods_lines",
    "product_id",
    "products",
);
/// Unified purchase lines (after m00013).
pub const FK_DRAFT_PURCHASE_LINE_PRODUCT: ForeignKey = fk_restrict(
    "fk_draft_purchase_lines_product_id",
    "draft_purchase_lines",
    "product_id",
    "products",
);
pub const FK_POSTED_PURCHASE_LINE_PRODUCT: ForeignKey = fk_set_null(
    "fk_posted_purchase_lines_product_id",
    "posted_purchase_lines",
    "product_id",
    "products",
);
pub const FK_CANCELLED_PURCHASE_LINE_PRODUCT: ForeignKey = fk_set_null(
    "fk_cancelled_purchase_lines_product_id",
    "cancelled_purchase_lines",
    "product_id",
    "products",
);
pub const FK_DRAFT_PURCHASE_SERVICE_LINE_SERVICE: ForeignKey = fk_restrict(
    "fk_draft_purchase_service_lines_service_id",
    "draft_purchase_service_lines",
    "service_id",
    "products",
);
pub const FK_POSTED_PURCHASE_SERVICE_LINE_SERVICE: ForeignKey = fk_restrict(
    "fk_posted_purchase_service_lines_service_id",
    "posted_purchase_service_lines",
    "service_id",
    "products",
);
pub const FK_CANCELLED_PURCHASE_SERVICE_LINE_SERVICE: ForeignKey = fk_restrict(
    "fk_cancelled_purchase_service_lines_service_id",
    "cancelled_purchase_service_lines",
    "service_id",
    "products",
);

pub const DEFERRED_FOREIGN_KEYS: &[ForeignKey] = &[
    FK_DRAFT_PURCHASE_LINE_PRODUCT,
    FK_POSTED_PURCHASE_LINE_PRODUCT,
    FK_CANCELLED_PURCHASE_LINE_PRODUCT,
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
