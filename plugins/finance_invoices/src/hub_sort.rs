//! Invoice hub table sort keys and SQL expressions.

use lariv_core::components::{sort_desc, sort_key};
use sea_orm::sea_query::{
    Alias, Expr, ExprTrait, Func, Order, Query, SimpleExpr, SubQueryStatement, UnionType,
};

use crate::entities::{paid_invoice, partially_paid_invoice, posted_payment_term_line};
use lariv_plugin_finance_payments::entities::payment;

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
    let key = sort_key(sort);
    let desc = sort_desc(sort);
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
    Expr::cust(crate::logic::bill_to::customer_name_sql(invoice_table))
}

pub fn expr_line_untaxed(lines_table: &str, fk: &str, invoice_table: &str) -> SimpleExpr {
    let lines = Alias::new(lines_table);
    let invoice = Alias::new(invoice_table);
    subquery(
        Query::select()
            .expr(Func::coalesce([
                Expr::expr(Func::sum(Expr::col((
                    lines.clone(),
                    Alias::new("pre_tax_amount"),
                )))),
                Expr::val(0),
            ]))
            .from(lines.clone())
            .and_where(Expr::col((lines, Alias::new(fk))).equals((invoice, Alias::new("id"))))
            .to_owned(),
    )
}

pub fn expr_line_product_count(lines_table: &str, fk: &str, invoice_table: &str) -> SimpleExpr {
    let lines = Alias::new(lines_table);
    let invoice = Alias::new(invoice_table);
    subquery(
        Query::select()
            .expr(Func::count(Expr::col((lines.clone(), Alias::new("id")))))
            .from(lines.clone())
            .and_where(Expr::col((lines, Alias::new(fk))).equals((invoice, Alias::new("id"))))
            .to_owned(),
    )
}

pub fn expr_posted_final_due(invoice_table: &str) -> SimpleExpr {
    let invoice = Alias::new(invoice_table);
    subquery(
        Query::select()
            .expr(Func::max(Expr::col((
                posted_payment_term_line::Entity,
                posted_payment_term_line::Column::DueDate,
            ))))
            .from(posted_payment_term_line::Entity)
            .and_where(
                Expr::col((
                    posted_payment_term_line::Entity,
                    posted_payment_term_line::Column::PostedPaymentTermId,
                ))
                .equals((invoice, Alias::new("posted_payment_term_id"))),
            )
            .to_owned(),
    )
}

fn invoice_total_subquery(invoice: Alias) -> sea_orm::sea_query::SelectStatement {
    Query::select()
        .expr(Func::coalesce([
            Expr::expr(Func::sum(Expr::col((
                posted_payment_term_line::Entity,
                posted_payment_term_line::Column::Amount,
            )))),
            Expr::val(0),
        ]))
        .from(posted_payment_term_line::Entity)
        .and_where(
            Expr::col((
                posted_payment_term_line::Entity,
                posted_payment_term_line::Column::PostedPaymentTermId,
            ))
            .equals((invoice, Alias::new("posted_payment_term_id"))),
        )
        .to_owned()
}

pub fn expr_ar_amount(invoice_table: &str) -> SimpleExpr {
    subquery(invoice_total_subquery(Alias::new(invoice_table)))
}

/// Tax levied ≈ invoice total − untaxed (ignores withholding differences for sort order).
pub fn expr_tax_levied_approx(invoice_table: &str, lines_table: &str, fk: &str) -> SimpleExpr {
    Expr::expr(expr_ar_amount(invoice_table)).sub(expr_line_untaxed(lines_table, fk, invoice_table))
}

pub fn expr_open_balance(invoice_table: &str) -> SimpleExpr {
    let invoice = Alias::new(invoice_table);
    let payment_ids = Query::select()
        .column(paid_invoice::Column::PaymentId)
        .from(paid_invoice::Entity)
        .and_where(
            Expr::col(paid_invoice::Column::PostedInvoiceId)
                .equals((invoice.clone(), Alias::new("id"))),
        )
        .union(
            UnionType::Distinct,
            Query::select()
                .column(partially_paid_invoice::Column::PaymentId)
                .from(partially_paid_invoice::Entity)
                .and_where(
                    Expr::col(partially_paid_invoice::Column::PostedInvoiceId)
                        .equals((invoice.clone(), Alias::new("id"))),
                )
                .to_owned(),
        )
        .to_owned();
    let paid_sum = Query::select()
        .expr(Func::coalesce([
            Expr::expr(Func::sum(Expr::col((
                payment::Entity,
                payment::Column::Amount,
            )))),
            Expr::val(0),
        ]))
        .from(payment::Entity)
        .and_where(Expr::col((payment::Entity, payment::Column::Id)).in_subquery(payment_ids))
        .to_owned();
    Expr::expr(subquery(invoice_total_subquery(invoice))).sub(subquery(paid_sum))
}

fn subquery(sel: sea_orm::sea_query::SelectStatement) -> SimpleExpr {
    Expr::from(SubQueryStatement::from(sel))
}

/// `EXISTS` line row for a product foreign-key filter on the hub.
pub fn expr_product_exists(
    lines_table: &str,
    fk: &str,
    invoice_table: &str,
    product_id: i64,
) -> SimpleExpr {
    let lines = Alias::new(lines_table);
    let invoice = Alias::new(invoice_table);
    Expr::exists(
        Query::select()
            .expr(Expr::val(1))
            .from(lines.clone())
            .and_where(
                Expr::col((lines.clone(), Alias::new(fk))).equals((invoice, Alias::new("id"))),
            )
            .and_where(Expr::col((lines, Alias::new("product_id"))).eq(product_id))
            .to_owned(),
    )
}

/// Cast a hub table's `id` to text for trigram / substring filters.
pub fn expr_id_text(invoice_table: &str) -> SimpleExpr {
    Expr::col((Alias::new(invoice_table), Alias::new("id"))).cast_as(Alias::new("text"))
}
