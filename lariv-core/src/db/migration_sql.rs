//! Canonical SeaORM 2.x helpers for plugin migrations.
//!
//! Prefer these patterns in new migrations:
//! - **SeaQuery DDL/DML:** `manager.create_table(...)`, `conn.execute(&query)`, etc.
//! - **Raw SQL string:** `exec_sql(manager, sql).await?`
//! - **Runtime raw SQL:** `exec_sql_conn(conn, backend, sql).await?`
//! - **Another plugin's table:** create the column, then [`ensure_foreign_key`]. The
//!   constraint is skipped until that table exists. [`CROSS_PLUGIN_FOREIGN_KEYS`] is
//!   applied again after every plugin migrator has run.

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};
use sea_orm_migration::prelude::*;

/// Whether the migration connection targets PostgreSQL.
pub fn is_postgres(manager: &SchemaManager<'_>) -> bool {
    manager.get_database_backend() == DatabaseBackend::Postgres
}

/// Execute a raw SQL string during a migration (`execute_raw` wrapper).
pub async fn exec_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute_raw(Statement::from_string(manager.get_database_backend(), sql))
        .await
        .map(|_| ())
}

/// Execute a raw SQL string on any connection (tests, conditional DDL wrappers).
pub async fn exec_sql_conn<C: ConnectionTrait>(
    conn: &C,
    backend: DatabaseBackend,
    sql: &str,
) -> Result<(), DbErr> {
    conn.execute_raw(Statement::from_string(backend, sql))
        .await
        .map(|_| ())
}

/// `ON UPDATE` / `ON DELETE` action. Only these keywords are interpolated into DDL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferentialAction {
    Cascade,
    Restrict,
    SetNull,
    SetDefault,
    NoAction,
}

/// A foreign key that may point at a table owned by another plugin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForeignKey {
    pub constraint: &'static str,
    pub table: &'static str,
    pub column: &'static str,
    pub ref_table: &'static str,
    pub ref_column: &'static str,
    pub on_update: ReferentialAction,
    pub on_delete: ReferentialAction,
}

const fn fk_restrict(
    constraint: &'static str,
    table: &'static str,
    column: &'static str,
    ref_table: &'static str,
) -> ForeignKey {
    ForeignKey {
        constraint,
        table,
        column,
        ref_table,
        ref_column: "id",
        on_update: ReferentialAction::Cascade,
        on_delete: ReferentialAction::Restrict,
    }
}

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

pub const FK_DRAFT_INVOICE_SERVICE_LINE_SERVICE: ForeignKey = fk_restrict(
    "fk_draft_invoice_service_lines_service_id",
    "draft_invoice_service_lines",
    "service_id",
    "services",
);
pub const FK_POSTED_INVOICE_SERVICE_LINE_SERVICE: ForeignKey = fk_restrict(
    "fk_posted_invoice_service_lines_service_id",
    "posted_invoice_service_lines",
    "service_id",
    "services",
);
pub const FK_CANCELLED_INVOICE_SERVICE_LINE_SERVICE: ForeignKey = fk_restrict(
    "fk_cancelled_invoice_service_lines_service_id",
    "cancelled_invoice_service_lines",
    "service_id",
    "services",
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
pub const FK_DRAFT_PURCHASE_SERVICE_LINE_SERVICE: ForeignKey = fk_restrict(
    "fk_draft_purchase_service_lines_service_id",
    "draft_purchase_service_lines",
    "service_id",
    "services",
);
pub const FK_POSTED_PURCHASE_SERVICE_LINE_SERVICE: ForeignKey = fk_restrict(
    "fk_posted_purchase_service_lines_service_id",
    "posted_purchase_service_lines",
    "service_id",
    "services",
);
pub const FK_CANCELLED_PURCHASE_SERVICE_LINE_SERVICE: ForeignKey = fk_restrict(
    "fk_cancelled_purchase_service_lines_service_id",
    "cancelled_purchase_service_lines",
    "service_id",
    "services",
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

pub const FK_QUOTATIONS_CUSTOMER: ForeignKey = fk_restrict(
    "fk_quotations_customer_id",
    "quotations",
    "customer_id",
    "crm_customers",
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
    "services",
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

/// Forward cross-plugin keys. Purchase goods and service lines are omitted:
/// a later purchase migration points those columns at `purchase_products` and
/// `purchase_services` instead of the shared catalogs.
pub const CROSS_PLUGIN_FOREIGN_KEYS: &[ForeignKey] = &[
    FK_PRODUCTS_STOCK,
    FK_PRODUCT_JOURNAL_ENTRY_TEMPLATES_JOURNAL,
    FK_PRODUCT_JOURNAL_ITEM_ACCOUNT,
    FK_PRODUCT_JOURNAL_ITEM_JOURNAL,
    FK_PRODUCT_PREFERENCE_JOURNAL_JOURNAL,
    FK_PRODUCT_PREFERENCE_JOURNAL_ACCOUNT,
    FK_PRODUCT_PREFERENCES_INVENTORY_ACCOUNT,
    FK_PRODUCT_PREFERENCES_COST_ACCOUNT,
    FK_DRAFT_INVOICE_SERVICE_LINE_SERVICE,
    FK_POSTED_INVOICE_SERVICE_LINE_SERVICE,
    FK_CANCELLED_INVOICE_SERVICE_LINE_SERVICE,
    FK_POSTED_INVOICE_SERVICE_LINE_JOURNAL_ITEM,
    FK_CANCELLED_INVOICE_SERVICE_LINE_JOURNAL_ITEM,
    FK_DRAFT_INVOICE_SERVICE_LINE_TAX,
    FK_POSTED_INVOICE_SERVICE_LINE_TAX,
    FK_CANCELLED_INVOICE_SERVICE_LINE_TAX,
    FK_PAYMENT_JOURNAL_ITEM_JOURNAL,
    FK_PAYMENT_JOURNAL_ITEM_ACCOUNT,
    FK_POSTED_PURCHASE_SERVICE_LINE_JOURNAL_ITEM,
    FK_CANCELLED_PURCHASE_SERVICE_LINE_JOURNAL_ITEM,
    FK_DRAFT_PURCHASE_SERVICE_LINE_TAX,
    FK_POSTED_PURCHASE_SERVICE_LINE_TAX,
    FK_CANCELLED_PURCHASE_SERVICE_LINE_TAX,
    FK_PURCHASE_PRODUCTS_STOCK,
    FK_PURCHASE_PRODUCT_TAX,
    FK_PURCHASE_PRODUCT_PREFERENCE_TAX,
    FK_PURCHASE_PRODUCT_JOURNAL_JOURNAL,
    FK_PURCHASE_PRODUCT_JOURNAL_ACCOUNT,
    FK_PURCHASE_SERVICE_TAX,
    FK_PURCHASE_SERVICE_PREFERENCE_TAX,
    FK_PURCHASE_SERVICE_JOURNAL_JOURNAL,
    FK_PURCHASE_SERVICE_JOURNAL_ACCOUNT,
    FK_PURCHASE_PRODUCT_PREFERENCE_JOURNAL_JOURNAL,
    FK_PURCHASE_PRODUCT_PREFERENCE_JOURNAL_ACCOUNT,
    FK_PURCHASE_SERVICE_PREFERENCE_JOURNAL_JOURNAL,
    FK_PURCHASE_SERVICE_PREFERENCE_JOURNAL_ACCOUNT,
    FK_SERVICE_TAX,
    FK_SERVICE_PREFERENCE_TAX,
    FK_SERVICE_PREFERENCES_COST_ACCOUNT,
    FK_SERVICE_PREFERENCES_CREDIT_ACCOUNT,
    FK_SERVICE_JOURNAL_ITEM_JOURNAL,
    FK_SERVICE_JOURNAL_ITEM_ACCOUNT,
    FK_SERVICE_PREFERENCE_JOURNAL_JOURNAL,
    FK_SERVICE_PREFERENCE_JOURNAL_ACCOUNT,
    FK_TAX_JOURNAL_ITEM_JOURNAL,
    FK_TAX_JOURNAL_ITEM_ACCOUNT,
    FK_TAX_PREFERENCE_JOURNAL_JOURNAL,
    FK_TAX_PREFERENCE_JOURNAL_ACCOUNT,
    FK_TAXES_ACCOUNT,
    FK_QUOTATIONS_CUSTOMER,
    FK_QUOTATION_GOODS_LINE_PRODUCT,
    FK_QUOTATION_SERVICE_LINE_SERVICE,
    FK_QUOTATION_GOODS_LINE_TAX,
    FK_QUOTATION_SERVICE_LINE_TAX,
];

/// Add `key` when both relations and the column exist and no matching foreign key does.
///
/// The `ALTER TABLE` runs through `EXECUTE` and `%I`, so a missing peer table is never parsed.
pub async fn ensure_foreign_key<C: ConnectionTrait>(
    conn: &C,
    backend: DatabaseBackend,
    key: ForeignKey,
) -> Result<(), DbErr> {
    if backend != DatabaseBackend::Postgres {
        return Ok(());
    }
    validate_foreign_key(key)?;
    exec_sql_conn(conn, backend, &foreign_key_sql(key)).await
}

/// Add each key in `keys`. Missing peer tables are skipped.
pub async fn ensure_foreign_keys<C: ConnectionTrait>(
    conn: &C,
    backend: DatabaseBackend,
    keys: &[ForeignKey],
) -> Result<(), DbErr> {
    for key in keys {
        ensure_foreign_key(conn, backend, *key).await?;
    }
    Ok(())
}

/// [`ensure_foreign_keys`] against a migration schema manager.
pub async fn ensure_schema_foreign_keys(
    manager: &SchemaManager<'_>,
    keys: &[ForeignKey],
) -> Result<(), DbErr> {
    ensure_foreign_keys(
        manager.get_connection(),
        manager.get_database_backend(),
        keys,
    )
    .await
}

/// Re-apply [`CROSS_PLUGIN_FOREIGN_KEYS`] after plugin migrations have created their tables.
pub async fn ensure_cross_plugin_foreign_keys(db: &DatabaseConnection) -> Result<(), DbErr> {
    ensure_foreign_keys(db, db.get_database_backend(), CROSS_PLUGIN_FOREIGN_KEYS).await
}

fn validate_foreign_key(key: ForeignKey) -> Result<(), DbErr> {
    for name in [
        key.constraint,
        key.table,
        key.column,
        key.ref_table,
        key.ref_column,
    ] {
        if !valid_ident(name) {
            return Err(DbErr::Custom(format!("invalid sql identifier: {name}")));
        }
    }
    Ok(())
}

fn valid_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_') && name.len() <= 63
}

fn action_sql(action: ReferentialAction) -> &'static str {
    match action {
        ReferentialAction::Cascade => "CASCADE",
        ReferentialAction::Restrict => "RESTRICT",
        ReferentialAction::SetNull => "SET NULL",
        ReferentialAction::SetDefault => "SET DEFAULT",
        ReferentialAction::NoAction => "NO ACTION",
    }
}

fn foreign_key_sql(key: ForeignKey) -> String {
    let on_update = action_sql(key.on_update);
    let on_delete = action_sql(key.on_delete);
    format!(
        r#"DO $fk$
DECLARE
    v_table text := '{table}';
    v_column text := '{column}';
    v_constraint text := '{constraint}';
    v_ref_table text := '{ref_table}';
    v_ref_column text := '{ref_column}';
BEGIN
    IF to_regclass('public.' || v_table) IS NULL
       OR to_regclass('public.' || v_ref_table) IS NULL THEN
        RETURN;
    END IF;
    IF NOT EXISTS (
        SELECT 1
          FROM information_schema.columns
         WHERE table_schema = 'public'
           AND table_name = v_table
           AND column_name = v_column
    ) THEN
        RETURN;
    END IF;
    IF EXISTS (SELECT 1 FROM pg_constraint WHERE conname = v_constraint) THEN
        RETURN;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM pg_constraint AS c
          JOIN pg_class AS src ON src.oid = c.conrelid
          JOIN pg_class AS dst ON dst.oid = c.confrelid
          JOIN pg_namespace AS n ON n.oid = src.relnamespace
          JOIN pg_attribute AS a
            ON a.attrelid = c.conrelid
           AND a.attnum = ANY (c.conkey)
         WHERE c.contype = 'f'
           AND n.nspname = 'public'
           AND src.relname = v_table
           AND dst.relname = v_ref_table
           AND a.attname = v_column
    ) THEN
        RETURN;
    END IF;
    EXECUTE format(
        'ALTER TABLE %I ADD CONSTRAINT %I FOREIGN KEY (%I) REFERENCES %I (%I) ON UPDATE {on_update} ON DELETE {on_delete}',
        v_table, v_constraint, v_column, v_ref_table, v_ref_column
    );
END
$fk$;"#,
        table = key.table,
        column = key.column,
        constraint = key.constraint,
        ref_table = key.ref_table,
        ref_column = key.ref_column,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_plugin_foreign_keys_are_unique_identifiers() {
        let mut names = std::collections::HashSet::new();
        for key in CROSS_PLUGIN_FOREIGN_KEYS {
            for name in [
                key.constraint,
                key.table,
                key.column,
                key.ref_table,
                key.ref_column,
            ] {
                assert!(valid_ident(name), "{name}");
            }
            assert!(
                names.insert(key.constraint),
                "duplicate constraint {}",
                key.constraint
            );
        }
    }
}
