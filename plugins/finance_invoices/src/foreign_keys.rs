//! Foreign keys owned by invoices and quotations. Keys that point at another
//! plugin's table are listed in [`DEFERRED_FOREIGN_KEYS`] and retried after every
//! installed plugin has migrated. The goods and service line keys are historical
//! and stay unregistered so a later migration can leave those tables behind.

use lariv_core::db::migration_sql::{ForeignKey, fk_restrict, fk_set_null};

pub const FK_DRAFT_INVOICE_SERVICE_LINE_SERVICE: ForeignKey = fk_restrict(
    "fk_draft_invoice_service_lines_service_id",
    "draft_invoice_service_lines",
    "service_id",
    "products",
);
pub const FK_POSTED_INVOICE_SERVICE_LINE_SERVICE: ForeignKey = fk_restrict(
    "fk_posted_invoice_service_lines_service_id",
    "posted_invoice_service_lines",
    "service_id",
    "products",
);
pub const FK_CANCELLED_INVOICE_SERVICE_LINE_SERVICE: ForeignKey = fk_restrict(
    "fk_cancelled_invoice_service_lines_service_id",
    "cancelled_invoice_service_lines",
    "service_id",
    "products",
);
pub const FK_POSTED_INVOICE_SERVICE_LINE_JOURNAL_ITEM: ForeignKey = fk_restrict(
    "fk_posted_invoice_service_lines_journal_entry_item_id",
    "posted_invoice_service_lines",
    "journal_entry_item_id",
    "journal_entry_items",
);
pub const FK_CANCELLED_INVOICE_SERVICE_LINE_JOURNAL_ITEM: ForeignKey = fk_restrict(
    "fk_cancelled_invoice_service_lines_journal_entry_item_id",
    "cancelled_invoice_service_lines",
    "journal_entry_item_id",
    "journal_entry_items",
);
pub const FK_DRAFT_INVOICE_SERVICE_LINE_TAX: ForeignKey = fk_restrict(
    "fk_draft_invoice_service_line_taxes_tax_id",
    "draft_invoice_service_line_taxes",
    "tax_id",
    "taxes",
);
pub const FK_POSTED_INVOICE_SERVICE_LINE_TAX: ForeignKey = fk_restrict(
    "fk_posted_invoice_service_line_taxes_tax_id",
    "posted_invoice_service_line_taxes",
    "tax_id",
    "taxes",
);
pub const FK_CANCELLED_INVOICE_SERVICE_LINE_TAX: ForeignKey = fk_restrict(
    "fk_cancelled_invoice_service_line_taxes_tax_id",
    "cancelled_invoice_service_line_taxes",
    "tax_id",
    "taxes",
);
pub const FK_PAYMENT_JOURNAL_ITEM_JOURNAL: ForeignKey = fk_restrict(
    "fk_payment_journal_entry_item_templates_journal_id",
    "payment_journal_entry_item_templates",
    "journal_id",
    "journals",
);
pub const FK_PAYMENT_JOURNAL_ITEM_ACCOUNT: ForeignKey = fk_restrict(
    "fk_payment_journal_entry_item_templates_account_id",
    "payment_journal_entry_item_templates",
    "account_id",
    "accounts",
);

pub const FK_DRAFT_INVOICE_LINE_PRODUCT: ForeignKey = fk_restrict(
    "fk_draft_invoice_lines_product_id",
    "draft_invoice_lines",
    "product_id",
    "products",
);
pub const FK_POSTED_INVOICE_LINE_PRODUCT: ForeignKey = fk_set_null(
    "fk_posted_invoice_lines_product_id",
    "posted_invoice_lines",
    "product_id",
    "products",
);
pub const FK_CANCELLED_INVOICE_LINE_PRODUCT: ForeignKey = fk_set_null(
    "fk_cancelled_invoice_lines_product_id",
    "cancelled_invoice_lines",
    "product_id",
    "products",
);

pub const FK_QUOTATIONS_CUSTOMER: ForeignKey = fk_restrict(
    "fk_quotations_customer_id",
    "quotations",
    "customer_id",
    "customers",
);
pub const FK_QUOTATION_GOODS_LINE_PRODUCT: ForeignKey = fk_restrict(
    "fk_quotation_goods_lines_product_id",
    "quotation_goods_lines",
    "product_id",
    "products",
);
pub const FK_QUOTATION_SERVICE_LINE_SERVICE: ForeignKey = fk_restrict(
    "fk_quotation_service_lines_service_id",
    "quotation_service_lines",
    "service_id",
    "products",
);
pub const FK_QUOTATION_GOODS_LINE_TAX: ForeignKey = fk_restrict(
    "fk_quotation_goods_line_taxes_tax_id",
    "quotation_goods_line_taxes",
    "tax_id",
    "taxes",
);
pub const FK_QUOTATION_SERVICE_LINE_TAX: ForeignKey = fk_restrict(
    "fk_quotation_service_line_taxes_tax_id",
    "quotation_service_line_taxes",
    "tax_id",
    "taxes",
);
/// Unified quotation lines (after m00003).
pub const FK_QUOTATION_LINE_PRODUCT: ForeignKey = fk_restrict(
    "fk_quotation_lines_product_id",
    "quotation_lines",
    "product_id",
    "products",
);
pub const FK_QUOTATION_LINE_TAX: ForeignKey = fk_restrict(
    "fk_quotation_line_taxes_tax_id",
    "quotation_line_taxes",
    "tax_id",
    "taxes",
);

pub const DEFERRED_FOREIGN_KEYS: &[ForeignKey] = &[
    FK_DRAFT_INVOICE_LINE_PRODUCT,
    FK_POSTED_INVOICE_LINE_PRODUCT,
    FK_CANCELLED_INVOICE_LINE_PRODUCT,
    FK_PAYMENT_JOURNAL_ITEM_JOURNAL,
    FK_PAYMENT_JOURNAL_ITEM_ACCOUNT,
    FK_QUOTATIONS_CUSTOMER,
    FK_QUOTATION_LINE_PRODUCT,
    FK_QUOTATION_LINE_TAX,
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
