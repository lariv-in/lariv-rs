//! Invoice hub table sort keys and SQL expressions.

use sea_orm::sea_query::{Expr, Order, SimpleExpr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HubSortKey {
    Id,
    Number,
    Date,
    DeliveryDate,
    Customer,
    OpenBalance,
    UntaxedAmount,
    TotalAmount,
    TaxLevied,
    ProductCount,
    FinalDueDate,
}

/// Parse `sort` query (`"Column ASC"` / `"Column DESC"` / `"Column"`).
pub fn parse_hub_sort(sort: &str) -> Option<(HubSortKey, bool)> {
    let sort = sort.trim();
    if sort.is_empty() {
        return None;
    }
    let mut parts = sort.split_whitespace();
    let key = parts.next()?;
    let desc = parts.next().is_some_and(|d| d.eq_ignore_ascii_case("DESC"));
    let key = if key.eq_ignore_ascii_case("ID") {
        HubSortKey::Id
    } else if key.eq_ignore_ascii_case("Number") {
        HubSortKey::Number
    } else if key.eq_ignore_ascii_case("Date") {
        HubSortKey::Date
    } else if key.eq_ignore_ascii_case("DeliveryDate") {
        HubSortKey::DeliveryDate
    } else if key.eq_ignore_ascii_case("Customer") {
        HubSortKey::Customer
    } else if key.eq_ignore_ascii_case("OpenBalance") {
        HubSortKey::OpenBalance
    } else if key.eq_ignore_ascii_case("UntaxedAmount") {
        HubSortKey::UntaxedAmount
    } else if key.eq_ignore_ascii_case("TotalAmount") {
        HubSortKey::TotalAmount
    } else if key.eq_ignore_ascii_case("TaxLevied") {
        HubSortKey::TaxLevied
    } else if key.eq_ignore_ascii_case("ProductCount") {
        HubSortKey::ProductCount
    } else if key.eq_ignore_ascii_case("FinalDueDate") {
        HubSortKey::FinalDueDate
    } else {
        return None;
    };
    Some((key, desc))
}

pub fn sort_order(desc: bool) -> Order {
    if desc { Order::Desc } else { Order::Asc }
}

pub fn expr_customer(invoice_table: &str) -> SimpleExpr {
    Expr::cust(format!(
        "(SELECT name FROM customers WHERE customers.id = {invoice_table}.customer_id)"
    ))
}

pub fn expr_line_untaxed(lines_table: &str, fk: &str, invoice_table: &str) -> SimpleExpr {
    Expr::cust(format!(
        "(SELECT COALESCE(SUM(quantity * rate), 0) FROM {lines_table} \
          WHERE {lines_table}.{fk} = {invoice_table}.id)"
    ))
}

pub fn expr_line_product_count(lines_table: &str, fk: &str, invoice_table: &str) -> SimpleExpr {
    Expr::cust(format!(
        "(SELECT COUNT(*) FROM {lines_table} WHERE {lines_table}.{fk} = {invoice_table}.id)"
    ))
}

pub fn expr_posted_final_due(invoice_table: &str) -> SimpleExpr {
    Expr::cust(format!(
        "(SELECT MAX(due_date) FROM posted_payment_term_lines \
          WHERE posted_payment_term_lines.posted_payment_term_id = {invoice_table}.posted_payment_term_id)"
    ))
}

pub fn expr_ar_amount(invoice_table: &str) -> SimpleExpr {
    Expr::cust(format!(
        "(SELECT amount FROM journal_entry_items \
          WHERE journal_entry_items.journal_entry_id = {invoice_table}.journal_entry_id \
            AND journal_entry_items.account_id = {invoice_table}.account_receivable_id \
          LIMIT 1)"
    ))
}

/// Tax levied ≈ receivable − untaxed (ignores withholding differences for sort order).
pub fn expr_tax_levied_approx(invoice_table: &str, lines_table: &str, fk: &str) -> SimpleExpr {
    Expr::cust(format!(
        "((SELECT amount FROM journal_entry_items \
           WHERE journal_entry_items.journal_entry_id = {invoice_table}.journal_entry_id \
             AND journal_entry_items.account_id = {invoice_table}.account_receivable_id \
           LIMIT 1) \
          - (SELECT COALESCE(SUM(quantity * rate), 0) FROM {lines_table} \
             WHERE {lines_table}.{fk} = {invoice_table}.id))"
    ))
}

pub fn expr_open_balance(invoice_table: &str) -> SimpleExpr {
    Expr::cust(format!(
        "((SELECT amount FROM journal_entry_items \
           WHERE journal_entry_items.journal_entry_id = {invoice_table}.journal_entry_id \
             AND journal_entry_items.account_id = {invoice_table}.account_receivable_id \
           LIMIT 1) \
          - (SELECT COALESCE(SUM(amount), 0) FROM payments \
             WHERE payments.posted_invoice_id = {invoice_table}.id))"
    ))
}

pub fn expr_settlement_posted_number(settlement_table: &str) -> SimpleExpr {
    Expr::cust(format!(
        "(SELECT number FROM posted_invoices \
          WHERE posted_invoices.id = {settlement_table}.posted_invoice_id)"
    ))
}

pub fn expr_settlement_payment_datetime(settlement_table: &str) -> SimpleExpr {
    Expr::cust(format!(
        "(SELECT datetime FROM payments WHERE payments.id = {settlement_table}.payment_id)"
    ))
}

pub fn expr_settlement_posted_delivery(settlement_table: &str) -> SimpleExpr {
    Expr::cust(format!(
        "(SELECT delivery_date FROM posted_invoices \
          WHERE posted_invoices.id = {settlement_table}.posted_invoice_id)"
    ))
}

pub fn expr_settlement_untaxed(settlement_table: &str) -> SimpleExpr {
    Expr::cust(format!(
        "(SELECT COALESCE(SUM(quantity * rate), 0) FROM posted_invoice_lines \
          WHERE posted_invoice_lines.posted_invoice_id = {settlement_table}.posted_invoice_id)"
    ))
}

pub fn expr_settlement_product_count(settlement_table: &str) -> SimpleExpr {
    Expr::cust(format!(
        "(SELECT COUNT(*) FROM posted_invoice_lines \
          WHERE posted_invoice_lines.posted_invoice_id = {settlement_table}.posted_invoice_id)"
    ))
}

pub fn expr_settlement_final_due(settlement_table: &str) -> SimpleExpr {
    Expr::cust(format!(
        "(SELECT MAX(ptl.due_date) FROM posted_payment_term_lines ptl \
          INNER JOIN posted_invoices pi ON pi.posted_payment_term_id = ptl.posted_payment_term_id \
          WHERE pi.id = {settlement_table}.posted_invoice_id)"
    ))
}

pub fn expr_settlement_ar_amount(settlement_table: &str) -> SimpleExpr {
    Expr::cust(format!(
        "(SELECT jei.amount FROM journal_entry_items jei \
          INNER JOIN posted_invoices pi ON pi.journal_entry_id = jei.journal_entry_id \
            AND jei.account_id = pi.account_receivable_id \
          WHERE pi.id = {settlement_table}.posted_invoice_id LIMIT 1)"
    ))
}

pub fn expr_settlement_tax_levied_approx(settlement_table: &str) -> SimpleExpr {
    Expr::cust(format!(
        "((SELECT jei.amount FROM journal_entry_items jei \
           INNER JOIN posted_invoices pi ON pi.journal_entry_id = jei.journal_entry_id \
             AND jei.account_id = pi.account_receivable_id \
           WHERE pi.id = {settlement_table}.posted_invoice_id LIMIT 1) \
          - (SELECT COALESCE(SUM(quantity * rate), 0) FROM posted_invoice_lines \
             WHERE posted_invoice_lines.posted_invoice_id = {settlement_table}.posted_invoice_id))"
    ))
}
