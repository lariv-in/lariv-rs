//! Column filters for the invoice hub list.
//!
//! Text columns match a case-insensitive substring. Dates and amounts use
//! inclusive ranges. A selected product keeps invoices that have a line for
//! that product. Draft totals, tax, and final due dates (and cancelled totals
//! and tax) are applied in memory against the same metrics the table displays.
//! Other columns filter in SQL with the expressions used for sorting.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::sea_query::{Expr, ExprTrait, Func};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Select};
use serde::Deserialize;
use std::str::FromStr;

use crate::datetime::{
    DatetimeLocalInput, parse_date, parse_date_start_in_tz, parse_naive_datetime,
};
use crate::plugins::finance_invoices::entities::{
    cancelled_invoice, draft_invoice, posted_invoice,
};
use crate::plugins::finance_invoices::hub_sort::{
    expr_ar_amount, expr_customer, expr_line_product_count, expr_line_untaxed, expr_open_balance,
    expr_posted_final_due, expr_settlement_ar_amount, expr_settlement_final_due,
    expr_settlement_payment_datetime, expr_settlement_posted_delivery,
    expr_settlement_posted_number, expr_settlement_product_count,
    expr_settlement_tax_levied_approx, expr_settlement_untaxed, expr_tax_levied_approx,
};
use crate::plugins::finance_invoices::logic::InvoiceListMetrics;

/// Raw hub filter query params. HTML names match [`super::forms::InvoiceHubFilterForm`].
#[derive(Debug, Clone, Deserialize, Default)]
pub struct HubFilterInput {
    #[serde(default, rename = "ID", alias = "id")]
    pub id: Option<String>,
    #[serde(default, rename = "Number", alias = "number")]
    pub number: Option<String>,
    #[serde(default, rename = "Customer", alias = "customer")]
    pub customer: Option<String>,
    #[serde(default, rename = "OpenBalanceMin", alias = "open_balance_min")]
    pub open_balance_min: Option<String>,
    #[serde(default, rename = "OpenBalanceMax", alias = "open_balance_max")]
    pub open_balance_max: Option<String>,
    #[serde(default, rename = "DatetimeFrom", alias = "datetime_from")]
    pub datetime_from: Option<String>,
    #[serde(default, rename = "DatetimeTo", alias = "datetime_to")]
    pub datetime_to: Option<String>,
    #[serde(default, rename = "DeliveryDateFrom", alias = "delivery_date_from")]
    pub delivery_date_from: Option<String>,
    #[serde(default, rename = "DeliveryDateTo", alias = "delivery_date_to")]
    pub delivery_date_to: Option<String>,
    #[serde(default, rename = "UntaxedMin", alias = "untaxed_min")]
    pub untaxed_min: Option<String>,
    #[serde(default, rename = "UntaxedMax", alias = "untaxed_max")]
    pub untaxed_max: Option<String>,
    #[serde(default, rename = "TotalMin", alias = "total_min")]
    pub total_min: Option<String>,
    #[serde(default, rename = "TotalMax", alias = "total_max")]
    pub total_max: Option<String>,
    #[serde(default, rename = "TaxMin", alias = "tax_min")]
    pub tax_min: Option<String>,
    #[serde(default, rename = "TaxMax", alias = "tax_max")]
    pub tax_max: Option<String>,
    #[serde(default, rename = "ProductID", alias = "product_id")]
    pub product_id: Option<String>,
    #[serde(default, rename = "ProductCountMin", alias = "product_count_min")]
    pub product_count_min: Option<String>,
    #[serde(default, rename = "ProductCountMax", alias = "product_count_max")]
    pub product_count_max: Option<String>,
    #[serde(default, rename = "FinalDueFrom", alias = "final_due_from")]
    pub final_due_from: Option<String>,
    #[serde(default, rename = "FinalDueTo", alias = "final_due_to")]
    pub final_due_to: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bound<T> {
    Any,
    /// Non-empty input that could not be parsed. Matches no rows.
    Reject,
    Value(T),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DateTimeEnd {
    Inclusive(DateTime<Utc>),
    Exclusive(DateTime<Utc>),
}

#[derive(Clone, Debug)]
pub struct ParsedHubFilters {
    id: Option<String>,
    number: Option<String>,
    customer: Option<String>,
    open_balance_min: Bound<Decimal>,
    open_balance_max: Bound<Decimal>,
    datetime_from: Bound<DateTime<Utc>>,
    datetime_to: Bound<DateTimeEnd>,
    delivery_from: Bound<NaiveDate>,
    delivery_to: Bound<NaiveDate>,
    untaxed_min: Bound<Decimal>,
    untaxed_max: Bound<Decimal>,
    total_min: Bound<Decimal>,
    total_max: Bound<Decimal>,
    tax_min: Bound<Decimal>,
    tax_max: Bound<Decimal>,
    product_id: Bound<i64>,
    product_min: Bound<i64>,
    product_max: Bound<i64>,
    final_due_from: Bound<NaiveDate>,
    final_due_to: Bound<NaiveDate>,
}

impl ParsedHubFilters {
    pub fn from_query(input: &HubFilterInput, tz: &str) -> Self {
        Self {
            id: needle(&input.id),
            number: needle(&input.number),
            customer: needle(&input.customer),
            open_balance_min: parse_decimal(&input.open_balance_min),
            open_balance_max: parse_decimal(&input.open_balance_max),
            datetime_from: parse_datetime_start(&input.datetime_from, tz),
            datetime_to: parse_datetime_end(&input.datetime_to, tz),
            delivery_from: parse_date_bound(&input.delivery_date_from),
            delivery_to: parse_date_bound(&input.delivery_date_to),
            untaxed_min: parse_decimal(&input.untaxed_min),
            untaxed_max: parse_decimal(&input.untaxed_max),
            total_min: parse_decimal(&input.total_min),
            total_max: parse_decimal(&input.total_max),
            tax_min: parse_decimal(&input.tax_min),
            tax_max: parse_decimal(&input.tax_max),
            product_id: parse_i64(&input.product_id),
            product_min: parse_i64(&input.product_count_min),
            product_max: parse_i64(&input.product_count_max),
            final_due_from: parse_date_bound(&input.final_due_from),
            final_due_to: parse_date_bound(&input.final_due_to),
        }
    }

    /// A filled filter value could not be parsed, so the list is empty.
    ///
    /// Open balance is posted-only; see [`Self::rejected_posted`].
    pub fn rejected(&self) -> bool {
        bound_rejected(self.datetime_from)
            || bound_rejected(self.datetime_to)
            || bound_rejected(self.delivery_from)
            || bound_rejected(self.delivery_to)
            || bound_rejected(self.untaxed_min)
            || bound_rejected(self.untaxed_max)
            || bound_rejected(self.total_min)
            || bound_rejected(self.total_max)
            || bound_rejected(self.tax_min)
            || bound_rejected(self.tax_max)
            || bound_rejected(self.product_id)
            || bound_rejected(self.product_min)
            || bound_rejected(self.product_max)
            || bound_rejected(self.final_due_from)
            || bound_rejected(self.final_due_to)
    }

    pub fn rejected_posted(&self) -> bool {
        self.rejected()
            || bound_rejected(self.open_balance_min)
            || bound_rejected(self.open_balance_max)
    }

    /// Draft total, tax, and final due are not exact SQL expressions.
    pub fn needs_draft_metrics(&self) -> bool {
        bound_set(self.total_min)
            || bound_set(self.total_max)
            || bound_set(self.tax_min)
            || bound_set(self.tax_max)
            || bound_set(self.final_due_from)
            || bound_set(self.final_due_to)
    }

    /// Cancelled total and tax are computed from lines, not a journal balance.
    pub fn needs_cancelled_metrics(&self) -> bool {
        bound_set(self.total_min)
            || bound_set(self.total_max)
            || bound_set(self.tax_min)
            || bound_set(self.tax_max)
    }

    pub fn matches_draft_metrics(&self, metrics: &InvoiceListMetrics) -> bool {
        self.matches_amount_metrics(metrics) && self.matches_final_due(metrics.final_due)
    }

    pub fn matches_cancelled_metrics(&self, metrics: &InvoiceListMetrics) -> bool {
        self.matches_amount_metrics(metrics)
    }

    fn matches_amount_metrics(&self, metrics: &InvoiceListMetrics) -> bool {
        decimal_in_range(self.total_min, self.total_max, metrics.total)
            && decimal_in_range(self.tax_min, self.tax_max, metrics.tax_levied)
    }

    fn matches_final_due(&self, due: Option<NaiveDate>) -> bool {
        date_in_range(self.final_due_from, self.final_due_to, due)
    }
}

pub fn apply_draft_sql_filters(
    query: Select<draft_invoice::Entity>,
    filters: &ParsedHubFilters,
) -> Select<draft_invoice::Entity> {
    let query = apply_id_contains(query, "draft_invoices", &filters.id);
    let query = apply_col_contains(query, draft_invoice::Column::Number, &filters.number);
    let query = apply_expr_contains(query, || expr_customer("draft_invoices"), &filters.customer);
    let query = apply_datetime_col(query, draft_invoice::Column::Datetime, filters);
    let query = apply_date_col(
        query,
        draft_invoice::Column::DeliveryDate,
        filters.delivery_from,
        filters.delivery_to,
    );
    let query = apply_decimal_expr(
        query,
        || expr_line_untaxed("draft_invoice_lines", "draft_invoice_id", "draft_invoices"),
        filters.untaxed_min,
        filters.untaxed_max,
    );
    let query = apply_i64_expr(
        query,
        || expr_line_product_count("draft_invoice_lines", "draft_invoice_id", "draft_invoices"),
        filters.product_min,
        filters.product_max,
    );
    apply_invoice_product(
        query,
        "draft_invoice_lines",
        "draft_invoice_id",
        "draft_invoices",
        filters.product_id,
    )
}

pub fn apply_posted_sql_filters(
    query: Select<posted_invoice::Entity>,
    filters: &ParsedHubFilters,
) -> Select<posted_invoice::Entity> {
    let query = apply_id_contains(query, "posted_invoices", &filters.id);
    let query = apply_col_contains(query, posted_invoice::Column::Number, &filters.number);
    let query = apply_expr_contains(
        query,
        || expr_customer("posted_invoices"),
        &filters.customer,
    );
    let query = apply_decimal_expr(
        query,
        || expr_open_balance("posted_invoices"),
        filters.open_balance_min,
        filters.open_balance_max,
    );
    let query = apply_datetime_col(query, posted_invoice::Column::Datetime, filters);
    let query = apply_date_col(
        query,
        posted_invoice::Column::DeliveryDate,
        filters.delivery_from,
        filters.delivery_to,
    );
    let query = apply_decimal_expr(
        query,
        || {
            expr_line_untaxed(
                "posted_invoice_lines",
                "posted_invoice_id",
                "posted_invoices",
            )
        },
        filters.untaxed_min,
        filters.untaxed_max,
    );
    let query = apply_decimal_expr(
        query,
        || expr_ar_amount("posted_invoices"),
        filters.total_min,
        filters.total_max,
    );
    let query = apply_decimal_expr(
        query,
        || {
            expr_tax_levied_approx(
                "posted_invoices",
                "posted_invoice_lines",
                "posted_invoice_id",
            )
        },
        filters.tax_min,
        filters.tax_max,
    );
    let query = apply_i64_expr(
        query,
        || {
            expr_line_product_count(
                "posted_invoice_lines",
                "posted_invoice_id",
                "posted_invoices",
            )
        },
        filters.product_min,
        filters.product_max,
    );
    let query = apply_invoice_product(
        query,
        "posted_invoice_lines",
        "posted_invoice_id",
        "posted_invoices",
        filters.product_id,
    );
    apply_date_expr(
        query,
        || expr_posted_final_due("posted_invoices"),
        filters.final_due_from,
        filters.final_due_to,
    )
}

pub fn apply_cancelled_sql_filters(
    query: Select<cancelled_invoice::Entity>,
    filters: &ParsedHubFilters,
) -> Select<cancelled_invoice::Entity> {
    let query = apply_id_contains(query, "cancelled_invoices", &filters.id);
    let query = apply_col_contains(query, cancelled_invoice::Column::Number, &filters.number);
    let query = apply_expr_contains(
        query,
        || expr_customer("cancelled_invoices"),
        &filters.customer,
    );
    let query = apply_datetime_col(query, cancelled_invoice::Column::Datetime, filters);
    let query = apply_date_col(
        query,
        cancelled_invoice::Column::DeliveryDate,
        filters.delivery_from,
        filters.delivery_to,
    );
    let query = apply_decimal_expr(
        query,
        || {
            expr_line_untaxed(
                "cancelled_invoice_lines",
                "cancelled_invoice_id",
                "cancelled_invoices",
            )
        },
        filters.untaxed_min,
        filters.untaxed_max,
    );
    let query = apply_i64_expr(
        query,
        || {
            expr_line_product_count(
                "cancelled_invoice_lines",
                "cancelled_invoice_id",
                "cancelled_invoices",
            )
        },
        filters.product_min,
        filters.product_max,
    );
    let query = apply_invoice_product(
        query,
        "cancelled_invoice_lines",
        "cancelled_invoice_id",
        "cancelled_invoices",
        filters.product_id,
    );
    apply_date_expr(
        query,
        || expr_posted_final_due("cancelled_invoices"),
        filters.final_due_from,
        filters.final_due_to,
    )
}

pub fn apply_settlement_sql_filters<E: EntityTrait>(
    query: Select<E>,
    table: &str,
    filters: &ParsedHubFilters,
) -> Select<E> {
    let query = apply_id_contains(query, table, &filters.id);
    let query = apply_expr_contains(
        query,
        || expr_settlement_posted_number(table),
        &filters.number,
    );
    let query = apply_expr_contains(query, || settlement_customer_expr(table), &filters.customer);
    let query = apply_datetime_expr(query, || expr_settlement_payment_datetime(table), filters);
    let query = apply_date_expr(
        query,
        || expr_settlement_posted_delivery(table),
        filters.delivery_from,
        filters.delivery_to,
    );
    let query = apply_decimal_expr(
        query,
        || expr_settlement_untaxed(table),
        filters.untaxed_min,
        filters.untaxed_max,
    );
    let query = apply_decimal_expr(
        query,
        || expr_settlement_ar_amount(table),
        filters.total_min,
        filters.total_max,
    );
    let query = apply_decimal_expr(
        query,
        || expr_settlement_tax_levied_approx(table),
        filters.tax_min,
        filters.tax_max,
    );
    let query = apply_i64_expr(
        query,
        || expr_settlement_product_count(table),
        filters.product_min,
        filters.product_max,
    );
    let query = apply_settlement_product(query, table, filters.product_id);
    apply_date_expr(
        query,
        || expr_settlement_final_due(table),
        filters.final_due_from,
        filters.final_due_to,
    )
}

fn apply_invoice_product<E: EntityTrait>(
    query: Select<E>,
    lines: &str,
    fk: &str,
    parent: &str,
    product_id: Bound<i64>,
) -> Select<E> {
    apply_product_exists(query, product_id, |id| {
        format!(
            "EXISTS (SELECT 1 FROM {lines} \
              WHERE {lines}.{fk} = {parent}.id AND {lines}.product_id = {id})"
        )
    })
}

fn apply_settlement_product<E: EntityTrait>(
    query: Select<E>,
    table: &str,
    product_id: Bound<i64>,
) -> Select<E> {
    apply_product_exists(query, product_id, |id| {
        format!(
            "EXISTS (SELECT 1 FROM posted_invoice_lines \
              WHERE posted_invoice_lines.posted_invoice_id = {table}.posted_invoice_id \
                AND posted_invoice_lines.product_id = {id})"
        )
    })
}

fn apply_product_exists<E, F>(query: Select<E>, product_id: Bound<i64>, exists_sql: F) -> Select<E>
where
    E: EntityTrait,
    F: FnOnce(i64) -> String,
{
    match product_id {
        Bound::Any => query,
        Bound::Reject => query.filter(Expr::cust("FALSE")),
        Bound::Value(id) => query.filter(Expr::cust(exists_sql(id))),
    }
}

fn settlement_customer_expr(table: &str) -> Expr {
    Expr::cust(format!(
        "(SELECT name FROM customers WHERE customers.id = (\
           SELECT customer_id FROM posted_invoices \
           WHERE posted_invoices.id = {table}.posted_invoice_id))"
    ))
}

fn nonempty(raw: &Option<String>) -> Option<&str> {
    raw.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

fn needle(raw: &Option<String>) -> Option<String> {
    nonempty(raw).map(str::to_string)
}

fn bound_rejected<T>(bound: Bound<T>) -> bool {
    matches!(bound, Bound::Reject)
}

fn bound_set<T>(bound: Bound<T>) -> bool {
    matches!(bound, Bound::Value(_))
}

fn parse_decimal(raw: &Option<String>) -> Bound<Decimal> {
    let Some(s) = nonempty(raw) else {
        return Bound::Any;
    };
    match Decimal::from_str(&s.replace(',', "")) {
        Ok(value) => Bound::Value(value),
        Err(_) => Bound::Reject,
    }
}

fn parse_i64(raw: &Option<String>) -> Bound<i64> {
    let Some(s) = nonempty(raw) else {
        return Bound::Any;
    };
    match s.parse() {
        Ok(value) => Bound::Value(value),
        Err(_) => Bound::Reject,
    }
}

fn parse_date_bound(raw: &Option<String>) -> Bound<NaiveDate> {
    let Some(s) = nonempty(raw) else {
        return Bound::Any;
    };
    match parse_date(s) {
        Some(date) => Bound::Value(date),
        None => Bound::Reject,
    }
}

fn parse_datetime_start(raw: &Option<String>, tz: &str) -> Bound<DateTime<Utc>> {
    let Some(s) = nonempty(raw) else {
        return Bound::Any;
    };
    if let Some(dt) = DatetimeLocalInput::from_raw(s).to_stored(tz) {
        return Bound::Value(dt);
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Bound::Value(dt.with_timezone(&Utc));
    }
    match parse_date_start_in_tz(s, tz) {
        Some(dt) => Bound::Value(dt),
        None => Bound::Reject,
    }
}

fn parse_datetime_end(raw: &Option<String>, tz: &str) -> Bound<DateTimeEnd> {
    let Some(s) = nonempty(raw) else {
        return Bound::Any;
    };
    if parse_naive_datetime(s).is_some() {
        return match DatetimeLocalInput::from_raw(s).to_stored(tz) {
            Some(dt) => Bound::Value(DateTimeEnd::Inclusive(dt)),
            None => Bound::Reject,
        };
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Bound::Value(DateTimeEnd::Inclusive(dt.with_timezone(&Utc)));
    }
    let Some(date) = parse_date(s) else {
        return Bound::Reject;
    };
    let Some(next) = date.succ_opt() else {
        return Bound::Reject;
    };
    let label = next.format("%d/%m/%Y").to_string();
    match parse_date_start_in_tz(&label, tz) {
        Some(dt) => Bound::Value(DateTimeEnd::Exclusive(dt)),
        None => Bound::Reject,
    }
}

fn decimal_in_range(min: Bound<Decimal>, max: Bound<Decimal>, value: Decimal) -> bool {
    match min {
        Bound::Any => {}
        Bound::Reject => return false,
        Bound::Value(bound) if value < bound => return false,
        Bound::Value(_) => {}
    }
    match max {
        Bound::Any => {}
        Bound::Reject => return false,
        Bound::Value(bound) if value > bound => return false,
        Bound::Value(_) => {}
    }
    true
}

fn date_in_range(min: Bound<NaiveDate>, max: Bound<NaiveDate>, value: Option<NaiveDate>) -> bool {
    match min {
        Bound::Any => {}
        Bound::Reject => return false,
        Bound::Value(bound) => match value {
            Some(date) if date >= bound => {}
            _ => return false,
        },
    }
    match max {
        Bound::Any => {}
        Bound::Reject => return false,
        Bound::Value(bound) => match value {
            Some(date) if date <= bound => {}
            _ => return false,
        },
    }
    true
}

fn like_contains_pattern(needle: &str) -> String {
    let mut out = String::from("%");
    for c in needle.to_lowercase().chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

fn apply_id_contains<E: EntityTrait>(
    query: Select<E>,
    table: &str,
    needle: &Option<String>,
) -> Select<E> {
    let Some(needle) = needle else {
        return query;
    };
    query.filter(
        Func::lower(Expr::cust(format!("CAST({table}.id AS TEXT)")))
            .like(like_contains_pattern(needle)),
    )
}

fn apply_col_contains<E, C>(query: Select<E>, column: C, needle: &Option<String>) -> Select<E>
where
    E: EntityTrait,
    C: ColumnTrait,
{
    let Some(needle) = needle else {
        return query;
    };
    query.filter(Func::lower(Expr::col(column)).like(like_contains_pattern(needle)))
}

fn apply_expr_contains<E, F>(
    mut query: Select<E>,
    mut expr: F,
    needle: &Option<String>,
) -> Select<E>
where
    E: EntityTrait,
    F: FnMut() -> Expr,
{
    let Some(needle) = needle else {
        return query;
    };
    query = query.filter(Func::lower(expr()).like(like_contains_pattern(needle)));
    query
}

fn apply_datetime_col<E, C>(query: Select<E>, column: C, filters: &ParsedHubFilters) -> Select<E>
where
    E: EntityTrait,
    C: ColumnTrait + Copy,
{
    let query = match filters.datetime_from {
        Bound::Any => query,
        Bound::Reject => return query.filter(Expr::cust("FALSE")),
        Bound::Value(start) => query.filter(column.gte(start)),
    };
    match filters.datetime_to {
        Bound::Any => query,
        Bound::Reject => query.filter(Expr::cust("FALSE")),
        Bound::Value(DateTimeEnd::Inclusive(end)) => query.filter(column.lte(end)),
        Bound::Value(DateTimeEnd::Exclusive(end)) => query.filter(column.lt(end)),
    }
}

fn apply_datetime_expr<E, F>(query: Select<E>, mut expr: F, filters: &ParsedHubFilters) -> Select<E>
where
    E: EntityTrait,
    F: FnMut() -> Expr,
{
    let query = match filters.datetime_from {
        Bound::Any => query,
        Bound::Reject => return query.filter(Expr::cust("FALSE")),
        Bound::Value(start) => query.filter(expr().gte(Expr::val(start))),
    };
    match filters.datetime_to {
        Bound::Any => query,
        Bound::Reject => query.filter(Expr::cust("FALSE")),
        Bound::Value(DateTimeEnd::Inclusive(end)) => query.filter(expr().lte(Expr::val(end))),
        Bound::Value(DateTimeEnd::Exclusive(end)) => query.filter(expr().lt(Expr::val(end))),
    }
}

fn apply_date_col<E, C>(
    query: Select<E>,
    column: C,
    from: Bound<NaiveDate>,
    to: Bound<NaiveDate>,
) -> Select<E>
where
    E: EntityTrait,
    C: ColumnTrait + Copy,
{
    let query = match from {
        Bound::Any => query,
        Bound::Reject => return query.filter(Expr::cust("FALSE")),
        Bound::Value(date) => query.filter(column.gte(date)),
    };
    match to {
        Bound::Any => query,
        Bound::Reject => query.filter(Expr::cust("FALSE")),
        Bound::Value(date) => query.filter(column.lte(date)),
    }
}

fn apply_decimal_expr<E, F>(
    query: Select<E>,
    expr: F,
    min: Bound<Decimal>,
    max: Bound<Decimal>,
) -> Select<E>
where
    E: EntityTrait,
    F: FnMut() -> Expr,
{
    apply_expr_range(query, expr, min, max)
}

fn apply_i64_expr<E, F>(query: Select<E>, expr: F, min: Bound<i64>, max: Bound<i64>) -> Select<E>
where
    E: EntityTrait,
    F: FnMut() -> Expr,
{
    apply_expr_range(query, expr, min, max)
}

fn apply_date_expr<E, F>(
    query: Select<E>,
    expr: F,
    min: Bound<NaiveDate>,
    max: Bound<NaiveDate>,
) -> Select<E>
where
    E: EntityTrait,
    F: FnMut() -> Expr,
{
    apply_expr_range(query, expr, min, max)
}

fn apply_expr_range<E, F, T>(
    mut query: Select<E>,
    mut expr: F,
    min: Bound<T>,
    max: Bound<T>,
) -> Select<E>
where
    E: EntityTrait,
    F: FnMut() -> Expr,
    T: Into<sea_orm::Value> + Copy,
{
    query = match min {
        Bound::Any => query,
        Bound::Reject => return query.filter(Expr::cust("FALSE")),
        Bound::Value(value) => query.filter(expr().gte(Expr::val(value))),
    };
    match max {
        Bound::Any => query,
        Bound::Reject => query.filter(Expr::cust("FALSE")),
        Bound::Value(value) => query.filter(expr().lte(Expr::val(value))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rust_decimal::Decimal;

    fn input(datetime_from: &str, datetime_to: &str, total_min: &str) -> HubFilterInput {
        HubFilterInput {
            datetime_from: Some(datetime_from.into()),
            datetime_to: Some(datetime_to.into()),
            total_min: Some(total_min.into()),
            ..HubFilterInput::default()
        }
    }

    #[test]
    fn date_only_range_covers_the_whole_local_day() {
        let parsed = ParsedHubFilters::from_query(&input("01/04/2026", "01/04/2026", ""), "UTC");
        assert!(!parsed.rejected());
        assert_eq!(
            parsed.datetime_from,
            Bound::Value(Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap())
        );
        assert_eq!(
            parsed.datetime_to,
            Bound::Value(DateTimeEnd::Exclusive(
                Utc.with_ymd_and_hms(2026, 4, 2, 0, 0, 0).unwrap()
            ))
        );
    }

    #[test]
    fn datetime_upper_bound_is_inclusive() {
        let parsed = ParsedHubFilters::from_query(&input("", "01/04/2026 15:30:00", ""), "UTC");
        assert_eq!(
            parsed.datetime_to,
            Bound::Value(DateTimeEnd::Inclusive(
                Utc.with_ymd_and_hms(2026, 4, 1, 15, 30, 0).unwrap()
            ))
        );
    }

    #[test]
    fn unparseable_amount_rejects_the_list() {
        let parsed = ParsedHubFilters::from_query(&input("", "", "nope"), "UTC");
        assert!(parsed.rejected());
    }

    #[test]
    fn draft_metrics_use_inclusive_total_and_due_bounds() {
        let mut raw = HubFilterInput::default();
        raw.total_min = Some("10".into());
        raw.total_max = Some("20".into());
        raw.final_due_from = Some("01/04/2026".into());
        raw.final_due_to = Some("30/04/2026".into());
        let parsed = ParsedHubFilters::from_query(&raw, "UTC");
        let mut metrics = InvoiceListMetrics {
            total: Decimal::from(10),
            final_due: Some(NaiveDate::from_ymd_opt(2026, 4, 1).unwrap()),
            ..InvoiceListMetrics::default()
        };
        assert!(parsed.matches_draft_metrics(&metrics));
        metrics.total = Decimal::from(21);
        assert!(!parsed.matches_draft_metrics(&metrics));
        metrics.total = Decimal::from(10);
        metrics.final_due = None;
        assert!(!parsed.matches_draft_metrics(&metrics));
    }

    #[test]
    fn product_id_parses_and_rejects_junk() {
        let mut raw = HubFilterInput::default();
        raw.product_id = Some("12".into());
        let parsed = ParsedHubFilters::from_query(&raw, "UTC");
        assert_eq!(parsed.product_id, Bound::Value(12));
        assert!(!parsed.rejected());

        raw.product_id = Some("nope".into());
        let parsed = ParsedHubFilters::from_query(&raw, "UTC");
        assert!(parsed.rejected());

        raw.product_id = Some("  ".into());
        let parsed = ParsedHubFilters::from_query(&raw, "UTC");
        assert_eq!(parsed.product_id, Bound::Any);
        assert!(!parsed.rejected());
    }

    #[test]
    fn like_pattern_escapes_wildcards() {
        assert_eq!(like_contains_pattern("100%"), "%100\\%%");
        assert_eq!(like_contains_pattern("Inv"), "%inv%");
    }
}
