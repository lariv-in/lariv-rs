//! Column filters for the purchase hub list.
//!
//! Each catalog field is one comparison (`lt` / `gt` / `eq`), not an inclusive
//! min/max pair. Text uses trigram match. Draft totals, tax, and final due
//! (and cancelled totals and tax) stay in memory against the metrics the table
//! displays, because those values are not the SQL expressions used for sorting.
//! Other columns filter in SQL, including expressions that are not real columns.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::DatabaseBackend;
use sea_orm::sea_query::{Condition, Expr, ExprTrait, Func, SimpleExpr};
use std::str::FromStr;

use crate::entities::{cancelled_purchase, draft_purchase, posted_purchase};
use crate::hub_sort::{
    expr_ar_amount, expr_line_product_count, expr_line_untaxed, expr_open_balance,
    expr_posted_final_due, expr_tax_levied_approx, expr_vendor,
};
use crate::logic::PurchaseListMetrics;
use lariv_core::datetime::parse_date;
use lariv_core::db::trigram::{WORD_SIMILARITY_THRESHOLD, like_contains_pattern};
use lariv_core::list_filters::{
    CmpOp, FilterClause, FilterField, FilterWidget, custom, date_column, datetime_column,
    text_column,
};
use lariv_plugin_finance_products::routes::ProductFkSelectRouteTag;

/// Which purchase hub tab the catalog is built for.
///
/// Vendor and open balance exist only on the posted tab, matching the old
/// form's `when = "posted"` flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HubFilterKind {
    Draft,
    Posted,
    Cancelled,
}

impl HubFilterKind {
    pub fn from_tab(tab: &str) -> Self {
        match tab {
            "posted" => Self::Posted,
            "cancelled" => Self::Cancelled,
            _ => Self::Draft,
        }
    }
}

/// Catalog for one request. `FilterField` is not `Clone`; build this once.
pub fn hub_filter_fields(kind: HubFilterKind, tz: &str) -> Vec<FilterField> {
    let tz = tz.to_string();
    match kind {
        HubFilterKind::Draft => draft_fields(tz),
        HubFilterKind::Posted => posted_fields(tz),
        HubFilterKind::Cancelled => cancelled_fields(tz),
    }
}

/// In-memory totals, tax, and (on drafts) final due. `gt` is `>`, `lt` is `<`, `eq` is `=`.
pub fn matches_in_memory(post: &[FilterClause], metrics: &PurchaseListMetrics) -> bool {
    post.iter().all(|clause| match clause.field.as_str() {
        "total" => cmp_decimal(clause, metrics.total),
        "tax" => cmp_decimal(clause, metrics.tax_levied),
        "final_due" => cmp_date(clause, metrics.final_due),
        _ => true,
    })
}

fn draft_fields(tz: String) -> Vec<FilterField> {
    vec![
        id_field("draft_purchases"),
        text_column("number", "Number", draft_purchase::Column::Number),
        datetime_column("datetime", "Date", draft_purchase::Column::Datetime, tz),
        date_column(
            "delivery_date",
            "Delivery date",
            draft_purchase::Column::DeliveryDate,
        ),
        decimal_expr_field("untaxed", "Untaxed amount", false, || {
            expr_line_untaxed(
                "draft_purchase_goods_lines",
                "draft_purchase_id",
                "draft_purchases",
            )
        }),
        decimal_expr_field("total", "Total amount", true, || Expr::cust("0")),
        decimal_expr_field("tax", "Tax levied", true, || Expr::cust("0")),
        product_field(|id| {
            format!(
                "EXISTS (SELECT 1 FROM draft_purchase_goods_lines \
                  WHERE draft_purchase_goods_lines.draft_purchase_id = draft_purchases.id \
                    AND draft_purchase_goods_lines.product_id = {id})"
            )
        }),
        number_expr_field("product_count", "Number of products", || {
            expr_line_product_count(
                "draft_purchase_goods_lines",
                "draft_purchase_id",
                "draft_purchases",
            )
        }),
        date_expr_field("final_due", "Final due date", true, || Expr::cust("0")),
    ]
}

fn posted_fields(tz: String) -> Vec<FilterField> {
    vec![
        id_field("posted_purchases"),
        text_column("number", "Number", posted_purchase::Column::Number),
        text_expr_field("vendor", "Vendor", || expr_vendor("posted_purchases")),
        decimal_expr_field("open_balance", "Open balance", false, || {
            expr_open_balance("posted_purchases")
        }),
        datetime_column("datetime", "Date", posted_purchase::Column::Datetime, tz),
        date_column(
            "delivery_date",
            "Delivery date",
            posted_purchase::Column::DeliveryDate,
        ),
        decimal_expr_field("untaxed", "Untaxed amount", false, || {
            expr_line_untaxed(
                "posted_purchase_goods_lines",
                "posted_purchase_id",
                "posted_purchases",
            )
        }),
        decimal_expr_field("total", "Total amount", false, || {
            expr_ar_amount("posted_purchases")
        }),
        decimal_expr_field("tax", "Tax levied", false, || {
            expr_tax_levied_approx(
                "posted_purchases",
                "posted_purchase_goods_lines",
                "posted_purchase_id",
            )
        }),
        product_field(|id| {
            format!(
                "EXISTS (SELECT 1 FROM posted_purchase_goods_lines \
                  WHERE posted_purchase_goods_lines.posted_purchase_id = posted_purchases.id \
                    AND posted_purchase_goods_lines.product_id = {id})"
            )
        }),
        number_expr_field("product_count", "Number of products", || {
            expr_line_product_count(
                "posted_purchase_goods_lines",
                "posted_purchase_id",
                "posted_purchases",
            )
        }),
        date_expr_field("final_due", "Final due date", false, || {
            expr_posted_final_due("posted_purchases")
        }),
    ]
}

fn cancelled_fields(tz: String) -> Vec<FilterField> {
    vec![
        id_field("cancelled_purchases"),
        text_column("number", "Number", cancelled_purchase::Column::Number),
        datetime_column("datetime", "Date", cancelled_purchase::Column::Datetime, tz),
        date_column(
            "delivery_date",
            "Delivery date",
            cancelled_purchase::Column::DeliveryDate,
        ),
        decimal_expr_field("untaxed", "Untaxed amount", false, || {
            expr_line_untaxed(
                "cancelled_purchase_goods_lines",
                "cancelled_purchase_id",
                "cancelled_purchases",
            )
        }),
        decimal_expr_field("total", "Total amount", true, || Expr::cust("0")),
        decimal_expr_field("tax", "Tax levied", true, || Expr::cust("0")),
        product_field(|id| {
            format!(
                "EXISTS (SELECT 1 FROM cancelled_purchase_goods_lines \
                  WHERE cancelled_purchase_goods_lines.cancelled_purchase_id = cancelled_purchases.id \
                    AND cancelled_purchase_goods_lines.product_id = {id})"
            )
        }),
        number_expr_field("product_count", "Number of products", || {
            expr_line_product_count(
                "cancelled_purchase_goods_lines",
                "cancelled_purchase_id",
                "cancelled_purchases",
            )
        }),
        date_expr_field("final_due", "Final due date", false, || {
            expr_posted_final_due("cancelled_purchases")
        }),
    ]
}

fn id_field(table: &'static str) -> FilterField {
    text_expr_field("id", "ID", move || {
        Expr::cust(format!("CAST({table}.id AS TEXT)"))
    })
}

fn text_expr_field<F>(key: &'static str, label: &'static str, expr: F) -> FilterField
where
    F: Fn() -> SimpleExpr + Send + 'static,
{
    custom(
        key,
        label,
        FilterWidget::Text,
        Vec::new(),
        false,
        move |clause, backend| Ok(text_expr_condition(backend, expr(), clause.value())),
    )
}

/// Trigram match against a SQL expression.
///
/// [`text_column`] only accepts a column reference. Id is cast to text, and
/// vendor is a subquery, so this follows
/// [`text_match_condition`](lariv_core::db::trigram::text_match_condition):
/// Postgres `word_similarity` plus a case-insensitive substring.
fn text_expr_condition(backend: DatabaseBackend, expr: SimpleExpr, needle: &str) -> Condition {
    let needle = needle.trim();
    if needle.is_empty() {
        return Condition::all();
    }
    let postgres = matches!(backend, DatabaseBackend::Postgres);
    let mut cond = Condition::any();
    let lowered: SimpleExpr = Func::cust("LOWER").arg(expr).into();
    if postgres {
        let similarity: SimpleExpr = Func::cust("word_similarity")
            .arg(needle.to_lowercase())
            .arg(lowered.clone())
            .into();
        cond = cond.add(similarity.gte(WORD_SIMILARITY_THRESHOLD));
    }
    cond = cond.add(Expr::expr(lowered).like(like_contains_pattern(needle)));
    cond
}

fn decimal_expr_field<F>(
    key: &'static str,
    label: &'static str,
    post_filter: bool,
    expr: F,
) -> FilterField
where
    F: Fn() -> SimpleExpr + Send + 'static,
{
    custom(
        key,
        label,
        FilterWidget::Decimal,
        Vec::new(),
        post_filter,
        move |clause, _| {
            let raw = clause.value();
            let parsed = parse_decimal_value(raw)?;
            if post_filter {
                return Ok(Condition::all());
            }
            Ok(one(cmp_expr(expr(), clause.op, parsed)))
        },
    )
}

fn number_expr_field<F>(key: &'static str, label: &'static str, expr: F) -> FilterField
where
    F: Fn() -> SimpleExpr + Send + 'static,
{
    custom(
        key,
        label,
        FilterWidget::Number,
        Vec::new(),
        false,
        move |clause, _| {
            let raw = clause.value();
            let parsed = raw
                .parse::<i64>()
                .map_err(|_| format!("invalid number: {raw}"))?;
            Ok(one(cmp_expr(expr(), clause.op, parsed)))
        },
    )
}

fn date_expr_field<F>(
    key: &'static str,
    label: &'static str,
    post_filter: bool,
    expr: F,
) -> FilterField
where
    F: Fn() -> SimpleExpr + Send + 'static,
{
    custom(
        key,
        label,
        FilterWidget::Date,
        Vec::new(),
        post_filter,
        move |clause, _| {
            let raw = clause.value();
            let parsed = parse_date(raw).ok_or_else(|| format!("invalid date: {raw}"))?;
            if post_filter {
                return Ok(Condition::all());
            }
            Ok(one(cmp_expr(expr(), clause.op, parsed)))
        },
    )
}

fn product_field<F>(exists: F) -> FilterField
where
    F: Fn(i64) -> String + Send + 'static,
{
    custom(
        "product",
        "Product",
        FilterWidget::ForeignKey {
            url: ProductFkSelectRouteTag.url(),
            placeholder: "Select product…".into(),
            search_key: "Name".into(),
        },
        Vec::new(),
        false,
        move |clause, _| {
            let raw = clause.value();
            let id = raw
                .parse::<i64>()
                .map_err(|_| format!("invalid id: {raw}"))?;
            if id <= 0 {
                return Err(format!("invalid id: {raw}"));
            }
            Ok(one(Expr::cust(exists(id))))
        },
    )
}

fn cmp_expr(expr: SimpleExpr, op: CmpOp, value: impl Into<sea_orm::Value>) -> SimpleExpr {
    let value = value.into();
    match op {
        CmpOp::Lt => expr.lt(value),
        CmpOp::Gt => expr.gt(value),
        CmpOp::Eq => expr.eq(value),
    }
}

fn one(expr: impl Into<SimpleExpr>) -> Condition {
    Condition::all().add(expr.into())
}

fn parse_decimal_value(raw: &str) -> Result<Decimal, String> {
    Decimal::from_str(raw.trim()).map_err(|_| format!("invalid number: {raw}"))
}

fn cmp_decimal(clause: &FilterClause, value: Decimal) -> bool {
    let Ok(bound) = parse_decimal_value(clause.value()) else {
        return false;
    };
    match clause.op {
        CmpOp::Gt => value > bound,
        CmpOp::Lt => value < bound,
        CmpOp::Eq => value == bound,
    }
}

fn cmp_date(clause: &FilterClause, value: Option<NaiveDate>) -> bool {
    let Some(bound) = parse_date(clause.value()) else {
        return false;
    };
    let Some(value) = value else {
        return false;
    };
    match clause.op {
        CmpOp::Gt => value > bound,
        CmpOp::Lt => value < bound,
        CmpOp::Eq => value == bound,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lariv_core::components::{HtmlAttrs, ListFilterBuilder, list_filter_builder};
    use lariv_core::list_filters::{apply_filters, parse_list_filters};
    use sea_orm::sea_query::{Alias, PostgresQueryBuilder, Query};

    fn render_catalog(kind: HubFilterKind) -> String {
        let fields = hub_filter_fields(kind, "UTC");
        list_filter_builder(ListFilterBuilder {
            fields: &fields,
            rows: &[],
            page_size: 12,
            sort: "",
            attrs: HtmlAttrs::new(),
            apply_label: "Apply Filters",
            extra: maud::PreEscaped(String::new()),
        })
        .into_string()
    }

    fn sql(condition: Condition) -> String {
        Query::select()
            .expr(Expr::val(1))
            .from(Alias::new("t"))
            .cond_where(condition)
            .to_string(PostgresQueryBuilder)
    }

    #[test]
    fn posted_catalog_includes_vendor_and_open_balance() {
        let html = render_catalog(HubFilterKind::Posted);
        assert!(html.contains(r#"data-filter-widget="vendor""#), "{html}");
        assert!(
            html.contains(r#"data-filter-widget="open_balance""#),
            "{html}"
        );
        assert!(html.contains("\"vendor\""), "{html}");
        assert!(html.contains("\"open_balance\""), "{html}");
    }

    #[test]
    fn draft_catalog_omits_vendor_and_open_balance() {
        let html = render_catalog(HubFilterKind::Draft);
        assert!(!html.contains(r#"data-filter-widget="vendor""#), "{html}");
        assert!(
            !html.contains(r#"data-filter-widget="open_balance""#),
            "{html}"
        );
        assert!(html.contains(r#"data-filter-widget="number""#), "{html}");
        assert!(html.contains(r#"data-filter-widget="product""#), "{html}");
    }

    #[test]
    fn draft_total_and_due_compare_with_gt_lt_eq() {
        let fields = hub_filter_fields(HubFilterKind::Draft, "UTC");
        let bad = parse_list_filters("filter.0.field=total&filter.0.op=gt&filter.0.value=nope");
        let applied = apply_filters(DatabaseBackend::Postgres, &fields, &bad);
        assert!(
            applied
                .errors
                .iter()
                .any(|err| err.message.contains("invalid")),
            "{:?}",
            applied.errors
        );
        assert!(applied.post.is_empty());

        let clauses = parse_list_filters(
            "filter.0.field=total&filter.0.op=gt&filter.0.value=10&filter.1.field=final_due&filter.1.op=eq&filter.1.value=01/04/2026",
        );
        let applied = apply_filters(DatabaseBackend::Postgres, &fields, &clauses);
        assert!(applied.errors.is_empty(), "{:?}", applied.errors);
        assert_eq!(applied.post.len(), 2);
        let mut metrics = PurchaseListMetrics {
            total: Decimal::from(11),
            final_due: Some(NaiveDate::from_ymd_opt(2026, 4, 1).unwrap()),
            ..PurchaseListMetrics::default()
        };
        assert!(matches_in_memory(&applied.post, &metrics));
        metrics.total = Decimal::from(10);
        assert!(!matches_in_memory(&applied.post, &metrics));
        metrics.total = Decimal::from(11);
        metrics.final_due = None;
        assert!(!matches_in_memory(&applied.post, &metrics));

        let lt = parse_list_filters("filter.0.field=total&filter.0.op=lt&filter.0.value=10");
        let applied = apply_filters(DatabaseBackend::Postgres, &fields, &lt);
        metrics.total = Decimal::from(9);
        metrics.final_due = None;
        assert!(matches_in_memory(&applied.post, &metrics));
        metrics.total = Decimal::from(10);
        assert!(!matches_in_memory(&applied.post, &metrics));
    }

    #[test]
    fn product_id_parses_and_rejects_junk() {
        let fields = hub_filter_fields(HubFilterKind::Draft, "UTC");
        let bad = parse_list_filters("filter.0.field=product&filter.0.value=nope");
        let applied = apply_filters(DatabaseBackend::Postgres, &fields, &bad);
        assert!(!applied.errors.is_empty(), "{:?}", applied.errors);

        let good = parse_list_filters("filter.0.field=product&filter.0.value=12");
        let applied = apply_filters(DatabaseBackend::Postgres, &fields, &good);
        assert!(applied.errors.is_empty(), "{:?}", applied.errors);
        let rendered = sql(applied.condition);
        assert!(rendered.contains("product_id = 12"), "{rendered}");
    }

    #[test]
    fn posted_total_is_sql_not_in_memory() {
        let fields = hub_filter_fields(HubFilterKind::Posted, "UTC");
        let clauses = parse_list_filters("filter.0.field=total&filter.0.op=gt&filter.0.value=10");
        let applied = apply_filters(DatabaseBackend::Postgres, &fields, &clauses);
        assert!(applied.errors.is_empty(), "{:?}", applied.errors);
        assert!(applied.post.is_empty());
        let rendered = sql(applied.condition);
        assert!(rendered.contains('>'), "{rendered}");
    }
}
