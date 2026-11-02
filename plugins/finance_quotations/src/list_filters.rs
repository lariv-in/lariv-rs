//! Column filters for the quotations list.

use rust_decimal::Decimal;
use sea_orm::sea_query::{Condition, Expr, ExprTrait, Func, SimpleExpr};
use sea_orm::{DatabaseBackend, EntityTrait};
use std::str::FromStr;

use lariv_core::components::FilterClauseView;
use lariv_core::db::trigram::{WORD_SIMILARITY_THRESHOLD, like_contains_pattern};
use lariv_core::list_filters::{
    CmpOp, FilterField, FilterRowError, FilterWidget, apply_filters, custom, date_column,
    foreign_key_column, parse_list_filters, text_column,
};
use lariv_plugin_crm::routes::CustomerFkSelectRouteTag;
use lariv_plugin_finance_products::routes::ProductFkSelectRouteTag;
use sea_orm::ColumnTrait;

use crate::entities::QuotationStatus;
use crate::entities::quotation;

pub struct QuotationListFilters {
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub condition: Condition,
}

/// Catalog for one request. `FilterField` is not `Clone`; build this once.
pub fn quotation_list_fields() -> Vec<FilterField> {
    vec![
        id_field(),
        text_column("number", "Number", quotation::Column::Number),
        date_column("date", "Date", quotation::Column::Date),
        status_field(),
        foreign_key_column(
            "customer",
            "Customer",
            quotation::Column::CustomerId,
            CustomerFkSelectRouteTag.url(),
            "Select customer…",
        ),
        decimal_expr_field("untaxed", "Untaxed amount", || {
            Expr::cust(
                "(SELECT COALESCE(SUM(pre_tax_amount), 0) FROM quotation_lines \
                  WHERE quotation_lines.quotation_id = quotations.id)",
            )
        }),
        product_field(),
        number_expr_field("product_count", "Number of products", || {
            Expr::cust(
                "(SELECT COUNT(*) FROM quotation_lines \
                  WHERE quotation_lines.quotation_id = quotations.id)",
            )
        }),
        text_column("remarks", "Remarks", quotation::Column::Remarks),
        text_column("bank_account", "Account", quotation::Column::BankAccount),
    ]
}

fn status_field() -> FilterField {
    let choices = QuotationStatus::choices()
        .iter()
        .map(|(key, label)| ((*key).to_string(), (*label).to_string()))
        .collect::<Vec<_>>();
    let listed = choices.clone();
    custom(
        "status",
        "Status",
        FilterWidget::Choice,
        choices,
        false,
        move |clause, _| {
            let raw = clause.value();
            if !listed.iter().any(|(key, _)| key == raw) {
                return Err(format!("unknown value: {raw}"));
            }
            let kind =
                QuotationStatus::parse(raw).ok_or_else(|| format!("unknown value: {raw}"))?;
            Ok(Condition::all().add(quotation::Column::Status.eq(kind)))
        },
    )
}

pub fn prepare(query: &str, backend: DatabaseBackend) -> QuotationListFilters {
    let fields = quotation_list_fields();
    let clauses = parse_list_filters(query);
    let applied = apply_filters(backend, &fields, &clauses);
    let filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    QuotationListFilters {
        fields,
        filters,
        filter_errors: applied.errors,
        condition: applied.condition,
    }
}

pub async fn enrich_fk_displays(
    db: &sea_orm::DatabaseConnection,
    filters: &mut [FilterClauseView],
) {
    for row in filters {
        match row.clause.field.as_str() {
            "product" => {
                row.fk_display = product_display(db, row.clause.value()).await;
            }
            "customer" => {
                row.fk_display = customer_display(db, row.clause.value()).await;
            }
            _ => {}
        }
    }
}

async fn product_display(db: &sea_orm::DatabaseConnection, raw: &str) -> String {
    let Some(id) = raw.trim().parse::<i64>().ok().filter(|id| *id > 0) else {
        return String::new();
    };
    lariv_core::web::opt_or_log(
        lariv_plugin_finance_products::entities::product::Entity::find_by_id(id)
            .one(db)
            .await,
        "find product for quotation list filter",
    )
    .map(|product| product.name)
    .unwrap_or_else(|| format!("#{id}"))
}

async fn customer_display(db: &sea_orm::DatabaseConnection, raw: &str) -> String {
    let Some(id) = raw.trim().parse::<i64>().ok().filter(|id| *id > 0) else {
        return String::new();
    };
    let label = lariv_plugin_customer::logic::display_name(db, id).await;
    if label.is_empty() {
        format!("#{id}")
    } else {
        label
    }
}

fn id_field() -> FilterField {
    text_expr_field("id", "ID", || Expr::cust("CAST(quotations.id AS TEXT)"))
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

fn decimal_expr_field<F>(key: &'static str, label: &'static str, expr: F) -> FilterField
where
    F: Fn() -> SimpleExpr + Send + 'static,
{
    custom(
        key,
        label,
        FilterWidget::Decimal,
        Vec::new(),
        false,
        move |clause, _| {
            let raw = clause.value();
            let parsed = Decimal::from_str(raw.trim())
                .map_err(|_| format!("invalid number: {raw}"))?;
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

fn product_field() -> FilterField {
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
            Ok(one(Expr::cust(format!(
                "EXISTS (SELECT 1 FROM quotation_lines \
                  WHERE quotation_lines.quotation_id = quotations.id \
                    AND quotation_lines.product_id = {id})"
            ))))
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
