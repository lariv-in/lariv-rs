use axum::{
    extract::Query,
    http::{HeaderMap, Uri},
};
use sea_orm::sea_query::{Expr, ExprTrait, SimpleExpr};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Select};

use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder, SlotCtx, SwapKey};
use lariv_core::http::Cap;
use lariv_core::list_filters::{
    AppliedFilters, FilterClause, FilterRowError, apply_filters, parse_list_filters,
};
use lariv_core::template::RenderAppPane;
use lariv_core::web::{Htmx, QueryPage, QueryPageSize, html_built_page_with_slots};
use lariv_plugin_users::middleware::RequireAuth;

use crate::logic::bill_to::PartyLabels;
use lariv_plugin_finance_accounts::scope::{
    CurrencyFormat, load_default_currency_format, load_journal_currency_formats,
};

use crate::{
    entities::{
        cancelled_invoice::{self, Entity as CancelledInvoiceEntity},
        draft_invoice::{self, Entity as DraftInvoiceEntity},
        posted_invoice::{self, Entity as PostedInvoiceEntity},
        quotation_invoice::{self, Entity as QuotationInvoiceEntity},
    },
    hub_filter::{HubFilterKind, hub_filter_fields, matches_in_memory},
    hub_filter_addon::{HubQueryParams, apply_extra_sql_filters, render_extra_filter_inputs},
    hub_sort::{
        HubSortKey, expr_ar_amount, expr_customer, expr_line_product_count, expr_line_untaxed,
        expr_open_balance, expr_posted_final_due, expr_tax_levied_approx, parse_hub_sort,
        sort_order,
    },
    hub_table_addon::enrich_hub_rows,
    invoice_addon_key::InvoiceAddonKey,
    keys::INVOICE_HUB_TABLE,
    logic::{
        InvoiceDateFormats, InvoiceListMetrics, cancelled_invoice_list_metrics,
        draft_invoice_list_metrics, load_invoice_date_formats, posted_invoice_list_metrics,
        posted_invoice_open_balance, quotation_invoice_list_metrics,
    },
    routes::{
        CancelledInvoiceDetailRouteTag, DraftInvoiceDetailRouteTag, PostedInvoiceDetailRouteTag,
        QuotationInvoiceDetailRouteTag,
    },
    scope::{
        LarivEnvironment, list_fiscal_year_options, resolve_list_fiscal_year,
        selected_fiscal_year_start_for_ui, sql_posted_not_cancelled,
    },
    state::InvoicesState,
    templates::{InvoiceHubPage, InvoiceRow},
};

fn hub_row_extras_none() -> (String, String, bool) {
    (String::new(), String::new(), false)
}

fn format_hub_delivery_date(d: Option<chrono::NaiveDate>, dates: &InvoiceDateFormats) -> String {
    dates.calendar_or_dash(d)
}

fn format_metrics(
    metrics: &InvoiceListMetrics,
    fmt: &CurrencyFormat,
    dates: &InvoiceDateFormats,
) -> (String, String, String, String, String) {
    let final_due = dates.calendar_or_dash(metrics.final_due);
    (
        fmt.display(metrics.untaxed),
        fmt.display(metrics.total),
        fmt.display(metrics.tax_levied),
        metrics.product_count.to_string(),
        final_due,
    )
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct HubQuery {
    #[serde(default)]
    pub tab: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
    #[serde(default)]
    pub sort: Option<String>,
}

pub(crate) async fn filter_clause_views(
    db: &sea_orm::DatabaseConnection,
    clauses: &[FilterClause],
    errors: &[FilterRowError],
) -> Vec<FilterClauseView> {
    let mut filters = FilterClauseView::from_clauses(clauses, errors);
    for row in &mut filters {
        match row.clause.field.as_str() {
            "product" => {
                row.fk_display = hub_product_display(db, row.clause.value()).await;
            }
            "customer" => {
                row.fk_display = hub_customer_display(db, row.clause.value()).await;
            }
            _ => {}
        }
    }
    filters
}

async fn hub_product_display(db: &sea_orm::DatabaseConnection, raw: &str) -> String {
    let Some(id) = raw.trim().parse::<i64>().ok().filter(|id| *id > 0) else {
        return String::new();
    };
    lariv_core::web::opt_or_log(
        lariv_plugin_finance_products::entities::product::Entity::find_by_id(id)
            .one(db)
            .await,
        "find product for invoice hub filter",
    )
    .map(|product| product.name)
    .unwrap_or_else(|| format!("#{id}"))
}

async fn hub_customer_display(db: &sea_orm::DatabaseConnection, raw: &str) -> String {
    let Some(id) = raw.trim().parse::<i64>().ok().filter(|id| *id > 0) else {
        return String::new();
    };
    let label = crate::logic::bill_to::customer_label(db, id).await;
    if label.is_empty() {
        format!("#{id}")
    } else {
        label
    }
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn cookie_header(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
}

fn apply_fiscal_year_datetime_filter<C, E>(
    query: Select<E>,
    env: &LarivEnvironment,
    column: C,
) -> Select<E>
where
    C: ColumnTrait,
    E: EntityTrait,
{
    apply_fiscal_year_expr_filter(query, env, Expr::col(column))
}

fn apply_fiscal_year_expr_filter<E>(
    mut query: Select<E>,
    env: &LarivEnvironment,
    datetime: impl Into<SimpleExpr>,
) -> Select<E>
where
    E: EntityTrait,
{
    if let Some(fy) = resolve_list_fiscal_year(env) {
        let datetime = datetime.into();
        let (start, end) = fy.datetime_range();
        query = query
            .filter(datetime.clone().gte(start))
            .filter(datetime.lt(end));
    }
    query
}

fn order_by_expr<E>(query: Select<E>, expr: SimpleExpr, desc: bool) -> Select<E>
where
    E: EntityTrait,
{
    query.order_by(expr, sort_order(desc))
}

fn filter_customer<E>(
    query: Select<E>,
    customer_id: Option<i64>,
    column: impl ColumnTrait,
) -> Select<E>
where
    E: EntityTrait,
{
    match customer_id {
        Some(id) => query.filter(column.eq(id)),
        None => query,
    }
}

fn cmp_opt_date(
    a: Option<chrono::NaiveDate>,
    b: Option<chrono::NaiveDate>,
    desc: bool,
) -> std::cmp::Ordering {
    let ord = match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(x), Some(y)) => x.cmp(&y),
    };
    if desc { ord.reverse() } else { ord }
}

fn cmp_decimal(
    a: rust_decimal::Decimal,
    b: rust_decimal::Decimal,
    desc: bool,
) -> std::cmp::Ordering {
    let ord = a.cmp(&b);
    if desc { ord.reverse() } else { ord }
}

fn cmp_u32(a: u32, b: u32, desc: bool) -> std::cmp::Ordering {
    let ord = a.cmp(&b);
    if desc { ord.reverse() } else { ord }
}

fn cmp_metrics(
    a: &InvoiceListMetrics,
    b: &InvoiceListMetrics,
    key: HubSortKey,
    desc: bool,
) -> std::cmp::Ordering {
    match key {
        HubSortKey::UntaxedAmount => cmp_decimal(a.untaxed, b.untaxed, desc),
        HubSortKey::TotalAmount => cmp_decimal(a.total, b.total, desc),
        HubSortKey::TaxLevied => cmp_decimal(a.tax_levied, b.tax_levied, desc),
        HubSortKey::ProductCount => cmp_u32(a.product_count, b.product_count, desc),
        HubSortKey::FinalDueDate => cmp_opt_date(a.final_due, b.final_due, desc),
        _ => std::cmp::Ordering::Equal,
    }
}

fn quotation_needs_metric_sort(key: HubSortKey) -> bool {
    matches!(
        key,
        HubSortKey::TotalAmount | HubSortKey::TaxLevied | HubSortKey::FinalDueDate
    )
}

fn apply_quotation_sql_sort(
    query: Select<quotation_invoice::Entity>,
    key: HubSortKey,
    desc: bool,
) -> Select<quotation_invoice::Entity> {
    match key {
        HubSortKey::Id => {
            if desc {
                query.order_by_desc(quotation_invoice::Column::Id)
            } else {
                query.order_by_asc(quotation_invoice::Column::Id)
            }
        }
        HubSortKey::Number => {
            if desc {
                query.order_by_desc(quotation_invoice::Column::Number)
            } else {
                query.order_by_asc(quotation_invoice::Column::Number)
            }
        }
        HubSortKey::Date => {
            if desc {
                query.order_by_desc(quotation_invoice::Column::Datetime)
            } else {
                query.order_by_asc(quotation_invoice::Column::Datetime)
            }
        }
        HubSortKey::DeliveryDate => {
            if desc {
                query.order_by_desc(quotation_invoice::Column::DeliveryDate)
            } else {
                query.order_by_asc(quotation_invoice::Column::DeliveryDate)
            }
        }
        HubSortKey::Customer => order_by_expr(query, expr_customer("quotation_invoices"), desc),
        HubSortKey::UntaxedAmount => order_by_expr(
            query,
            expr_line_untaxed(
                "quotation_invoice_lines",
                "quotation_invoice_id",
                "quotation_invoices",
            ),
            desc,
        ),
        HubSortKey::ProductCount => order_by_expr(
            query,
            expr_line_product_count(
                "quotation_invoice_lines",
                "quotation_invoice_id",
                "quotation_invoices",
            ),
            desc,
        ),
        // Metric-sorted keys and posted-only columns fall through to date order.
        _ => {
            if desc {
                query.order_by_desc(quotation_invoice::Column::Datetime)
            } else {
                query.order_by_asc(quotation_invoice::Column::Datetime)
            }
        }
    }
}

async fn query_quotation_rows(
    db: &sea_orm::DatabaseConnection,
    customer_id: Option<i64>,
    q: &HubQuery,
    params: &HubQueryParams<'_>,
    env: &LarivEnvironment,
    tz: &str,
    dates: &InvoiceDateFormats,
    applied: &AppliedFilters,
) -> (Vec<InvoiceRow>, u32, u64) {
    let page_num = q.page.get();
    let mut query = QuotationInvoiceEntity::find();
    query = filter_customer(query, customer_id, quotation_invoice::Column::CustomerId);
    query = query.filter(applied.condition.clone());
    query = apply_extra_sql_filters(query, params, "quotation_invoices.id");
    query = apply_fiscal_year_datetime_filter(query, env, quotation_invoice::Column::Datetime);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let parsed_sort = parse_hub_sort(sort);
    let metric_sort = parsed_sort.filter(|(key, _)| quotation_needs_metric_sort(*key));
    if !applied.post.is_empty() || metric_sort.is_some() {
        if metric_sort.is_none() {
            query = match parsed_sort {
                Some((key, desc)) => apply_quotation_sql_sort(query, key, desc),
                None => query.order_by_desc(quotation_invoice::Column::Id),
            };
        }
        return quotation_rows_metric_sorted(
            db,
            query,
            page_num,
            q.page_size.get(),
            tz,
            dates,
            metric_sort,
            &applied.post,
        )
        .await;
    }
    query = match parsed_sort {
        Some((key, desc)) => apply_quotation_sql_sort(query, key, desc),
        None => query.order_by_desc(quotation_invoice::Column::Id),
    };
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (
        quotation_models_to_rows(db, &models, tz, dates).await,
        page_num,
        total,
    )
}

async fn quotation_rows_metric_sorted(
    db: &sea_orm::DatabaseConnection,
    query: Select<quotation_invoice::Entity>,
    page_num: u32,
    page_size: u32,
    tz: &str,
    dates: &InvoiceDateFormats,
    sort: Option<(HubSortKey, bool)>,
    post: &[lariv_core::list_filters::FilterClause],
) -> (Vec<InvoiceRow>, u32, u64) {
    let mut models = query.all(db).await.unwrap_or_default();
    let mut keyed = Vec::with_capacity(models.len());
    for m in models.drain(..) {
        let metrics = quotation_invoice_list_metrics(db, m.id, tz).await;
        if !matches_in_memory(post, &metrics) {
            continue;
        }
        keyed.push((m, metrics));
    }
    if let Some((key, desc)) = sort {
        keyed.sort_by(|(a, am), (b, bm)| {
            cmp_metrics(am, bm, key, desc).then_with(|| a.id.cmp(&b.id))
        });
    }
    let total = keyed.len() as u64;
    let start = ((page_num as usize).saturating_sub(1)).saturating_mul(page_size as usize);
    let page_models: Vec<_> = keyed
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .map(|(m, _)| m)
        .collect();
    (
        quotation_models_to_rows(db, &page_models, tz, dates).await,
        page_num,
        total,
    )
}

async fn quotation_models_to_rows(
    db: &sea_orm::DatabaseConnection,
    models: &[quotation_invoice::Model],
    tz: &str,
    dates: &InvoiceDateFormats,
) -> Vec<InvoiceRow> {
    let currency = load_default_currency_format(db).await;
    let customer_ids: Vec<i64> = models.iter().map(|d| d.customer_id).collect();
    let parties = crate::logic::bill_to::parties_in_order(db, &customer_ids).await;
    let customers = PartyLabels::load(db, &parties).await;
    let mut rows = Vec::with_capacity(models.len());
    for (d, party) in models.iter().zip(parties) {
        let metrics = quotation_invoice_list_metrics(db, d.id, tz).await;
        let (untaxed_amount, total_amount, tax_levied, product_count, final_due_date) =
            format_metrics(&metrics, &currency, dates);
        rows.push(InvoiceRow {
            id: d.id,
            addon_key: Some(InvoiceAddonKey::Quotation(d.id)),
            number: d.number.clone().unwrap_or_else(|| "—".to_string()),
            datetime: dates.datetime(d.datetime, tz),
            delivery_date: format_hub_delivery_date(d.delivery_date, dates),
            detail_href: QuotationInvoiceDetailRouteTag::new(d.id).url(),
            customer_name: customers.name(party),
            open_balance: String::new(),
            selectable: true,
            untaxed_amount,
            total_amount,
            tax_levied,
            product_count,
            final_due_date,
            extra_cells: Vec::new(),
        });
    }
    rows
}

fn draft_needs_metric_sort(key: HubSortKey) -> bool {
    matches!(
        key,
        HubSortKey::TotalAmount | HubSortKey::TaxLevied | HubSortKey::FinalDueDate
    )
}

fn cancelled_needs_metric_sort(key: HubSortKey) -> bool {
    matches!(key, HubSortKey::TotalAmount | HubSortKey::TaxLevied)
}

fn apply_draft_sql_sort(
    query: Select<draft_invoice::Entity>,
    key: HubSortKey,
    desc: bool,
) -> Select<draft_invoice::Entity> {
    match key {
        HubSortKey::Id => {
            if desc {
                query.order_by_desc(draft_invoice::Column::Id)
            } else {
                query.order_by_asc(draft_invoice::Column::Id)
            }
        }
        HubSortKey::Number => {
            if desc {
                query.order_by_desc(draft_invoice::Column::Number)
            } else {
                query.order_by_asc(draft_invoice::Column::Number)
            }
        }
        HubSortKey::Date => {
            if desc {
                query.order_by_desc(draft_invoice::Column::Datetime)
            } else {
                query.order_by_asc(draft_invoice::Column::Datetime)
            }
        }
        HubSortKey::DeliveryDate => {
            if desc {
                query.order_by_desc(draft_invoice::Column::DeliveryDate)
            } else {
                query.order_by_asc(draft_invoice::Column::DeliveryDate)
            }
        }
        HubSortKey::Customer => order_by_expr(query, expr_customer("draft_invoices"), desc),
        HubSortKey::UntaxedAmount => order_by_expr(
            query,
            expr_line_untaxed("draft_invoice_lines", "draft_invoice_id", "draft_invoices"),
            desc,
        ),
        HubSortKey::ProductCount => order_by_expr(
            query,
            expr_line_product_count("draft_invoice_lines", "draft_invoice_id", "draft_invoices"),
            desc,
        ),
        // Metric-sorted keys and posted-only columns fall through to date order.
        _ => {
            if desc {
                query.order_by_desc(draft_invoice::Column::Datetime)
            } else {
                query.order_by_asc(draft_invoice::Column::Datetime)
            }
        }
    }
}

fn apply_posted_sql_sort(
    query: Select<posted_invoice::Entity>,
    key: HubSortKey,
    desc: bool,
) -> Select<posted_invoice::Entity> {
    match key {
        HubSortKey::Id => {
            if desc {
                query.order_by_desc(posted_invoice::Column::Id)
            } else {
                query.order_by_asc(posted_invoice::Column::Id)
            }
        }
        HubSortKey::Number => {
            if desc {
                query.order_by_desc(posted_invoice::Column::Number)
            } else {
                query.order_by_asc(posted_invoice::Column::Number)
            }
        }
        HubSortKey::Date => {
            if desc {
                query.order_by_desc(posted_invoice::Column::Datetime)
            } else {
                query.order_by_asc(posted_invoice::Column::Datetime)
            }
        }
        HubSortKey::DeliveryDate => {
            if desc {
                query.order_by_desc(posted_invoice::Column::DeliveryDate)
            } else {
                query.order_by_asc(posted_invoice::Column::DeliveryDate)
            }
        }
        HubSortKey::Customer => order_by_expr(query, expr_customer("posted_invoices"), desc),
        HubSortKey::OpenBalance => order_by_expr(query, expr_open_balance("posted_invoices"), desc),
        HubSortKey::UntaxedAmount => order_by_expr(
            query,
            expr_line_untaxed(
                "posted_invoice_lines",
                "posted_invoice_id",
                "posted_invoices",
            ),
            desc,
        ),
        HubSortKey::TotalAmount => order_by_expr(query, expr_ar_amount("posted_invoices"), desc),
        HubSortKey::TaxLevied => order_by_expr(
            query,
            expr_tax_levied_approx(
                "posted_invoices",
                "posted_invoice_lines",
                "posted_invoice_id",
            ),
            desc,
        ),
        HubSortKey::ProductCount => order_by_expr(
            query,
            expr_line_product_count(
                "posted_invoice_lines",
                "posted_invoice_id",
                "posted_invoices",
            ),
            desc,
        ),
        HubSortKey::FinalDueDate => {
            order_by_expr(query, expr_posted_final_due("posted_invoices"), desc)
        }
    }
}

fn apply_cancelled_sql_sort(
    query: Select<cancelled_invoice::Entity>,
    key: HubSortKey,
    desc: bool,
) -> Select<cancelled_invoice::Entity> {
    match key {
        HubSortKey::Id => {
            if desc {
                query.order_by_desc(cancelled_invoice::Column::Id)
            } else {
                query.order_by_asc(cancelled_invoice::Column::Id)
            }
        }
        HubSortKey::Number => {
            if desc {
                query.order_by_desc(cancelled_invoice::Column::Number)
            } else {
                query.order_by_asc(cancelled_invoice::Column::Number)
            }
        }
        HubSortKey::Date => {
            if desc {
                query.order_by_desc(cancelled_invoice::Column::Datetime)
            } else {
                query.order_by_asc(cancelled_invoice::Column::Datetime)
            }
        }
        HubSortKey::DeliveryDate => {
            if desc {
                query.order_by_desc(cancelled_invoice::Column::DeliveryDate)
            } else {
                query.order_by_asc(cancelled_invoice::Column::DeliveryDate)
            }
        }
        HubSortKey::UntaxedAmount => order_by_expr(
            query,
            expr_line_untaxed(
                "cancelled_invoice_lines",
                "cancelled_invoice_id",
                "cancelled_invoices",
            ),
            desc,
        ),
        HubSortKey::ProductCount => order_by_expr(
            query,
            expr_line_product_count(
                "cancelled_invoice_lines",
                "cancelled_invoice_id",
                "cancelled_invoices",
            ),
            desc,
        ),
        HubSortKey::FinalDueDate => {
            order_by_expr(query, expr_posted_final_due("cancelled_invoices"), desc)
        }
        _ => {
            if desc {
                query.order_by_desc(cancelled_invoice::Column::Datetime)
            } else {
                query.order_by_asc(cancelled_invoice::Column::Datetime)
            }
        }
    }
}

async fn query_draft_rows(
    db: &sea_orm::DatabaseConnection,
    customer_id: Option<i64>,
    q: &HubQuery,
    params: &HubQueryParams<'_>,
    env: &LarivEnvironment,
    tz: &str,
    dates: &InvoiceDateFormats,
    applied: &AppliedFilters,
) -> (Vec<InvoiceRow>, u32, u64) {
    let page_num = q.page.get();
    let mut query = DraftInvoiceEntity::find();
    query = filter_customer(query, customer_id, draft_invoice::Column::CustomerId);
    query = query.filter(applied.condition.clone());
    query = apply_extra_sql_filters(query, params, "draft_invoices.id");
    query = apply_fiscal_year_datetime_filter(query, env, draft_invoice::Column::Datetime);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let parsed_sort = parse_hub_sort(sort);
    let metric_sort = parsed_sort.filter(|(key, _)| draft_needs_metric_sort(*key));
    if !applied.post.is_empty() || metric_sort.is_some() {
        if metric_sort.is_none() {
            query = match parsed_sort {
                Some((key, desc)) => apply_draft_sql_sort(query, key, desc),
                None => query.order_by_desc(draft_invoice::Column::Id),
            };
        }
        return draft_rows_metric_sorted(
            db,
            query,
            page_num,
            q.page_size.get(),
            tz,
            dates,
            metric_sort,
            &applied.post,
        )
        .await;
    }
    query = match parsed_sort {
        Some((key, desc)) => apply_draft_sql_sort(query, key, desc),
        None => query.order_by_desc(draft_invoice::Column::Id),
    };
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (
        draft_models_to_rows(db, &models, tz, dates).await,
        page_num,
        total,
    )
}

async fn draft_rows_metric_sorted(
    db: &sea_orm::DatabaseConnection,
    query: Select<draft_invoice::Entity>,
    page_num: u32,
    page_size: u32,
    tz: &str,
    dates: &InvoiceDateFormats,
    sort: Option<(HubSortKey, bool)>,
    post: &[lariv_core::list_filters::FilterClause],
) -> (Vec<InvoiceRow>, u32, u64) {
    let mut models = query.all(db).await.unwrap_or_default();
    let mut keyed = Vec::with_capacity(models.len());
    for m in models.drain(..) {
        let metrics = draft_invoice_list_metrics(db, m.id, tz).await;
        if !matches_in_memory(post, &metrics) {
            continue;
        }
        keyed.push((m, metrics));
    }
    if let Some((key, desc)) = sort {
        keyed.sort_by(|(a, am), (b, bm)| {
            cmp_metrics(am, bm, key, desc).then_with(|| a.id.cmp(&b.id))
        });
    }
    let total = keyed.len() as u64;
    let start = ((page_num as usize).saturating_sub(1)).saturating_mul(page_size as usize);
    let page_models: Vec<_> = keyed
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .map(|(m, _)| m)
        .collect();
    (
        draft_models_to_rows(db, &page_models, tz, dates).await,
        page_num,
        total,
    )
}

async fn draft_models_to_rows(
    db: &sea_orm::DatabaseConnection,
    models: &[draft_invoice::Model],
    tz: &str,
    dates: &InvoiceDateFormats,
) -> Vec<InvoiceRow> {
    let currency = load_default_currency_format(db).await;
    let customer_ids: Vec<i64> = models.iter().map(|d| d.customer_id).collect();
    let parties = crate::logic::bill_to::parties_in_order(db, &customer_ids).await;
    let customers = PartyLabels::load(db, &parties).await;
    let mut rows = Vec::with_capacity(models.len());
    for (d, party) in models.iter().zip(parties) {
        let metrics = draft_invoice_list_metrics(db, d.id, tz).await;
        let (untaxed_amount, total_amount, tax_levied, product_count, final_due_date) =
            format_metrics(&metrics, &currency, dates);
        rows.push(InvoiceRow {
            id: d.id,
            addon_key: Some(InvoiceAddonKey::Draft(d.id)),
            number: d.number.clone().unwrap_or_else(|| "—".to_string()),
            datetime: dates.datetime(d.datetime, tz),
            delivery_date: format_hub_delivery_date(d.delivery_date, dates),
            detail_href: DraftInvoiceDetailRouteTag::new(d.id).url(),
            customer_name: customers.name(party),
            open_balance: String::new(),
            selectable: true,
            untaxed_amount,
            total_amount,
            tax_levied,
            product_count,
            final_due_date,
            extra_cells: Vec::new(),
        });
    }
    rows
}

async fn query_posted_rows(
    db: &sea_orm::DatabaseConnection,
    customer_id: Option<i64>,
    q: &HubQuery,
    params: &HubQueryParams<'_>,
    env: &LarivEnvironment,
    tz: &str,
    dates: &InvoiceDateFormats,
    applied: &AppliedFilters,
) -> (Vec<InvoiceRow>, u32, u64) {
    let page_num = q.page.get();
    let mut query = PostedInvoiceEntity::find().filter(sql_posted_not_cancelled());
    query = filter_customer(query, customer_id, posted_invoice::Column::CustomerId);
    query = query.filter(applied.condition.clone());
    query = apply_extra_sql_filters(query, params, "posted_invoices.id");
    query = apply_fiscal_year_datetime_filter(query, env, posted_invoice::Column::Datetime);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match parse_hub_sort(sort) {
        Some((key, desc)) => apply_posted_sql_sort(query, key, desc),
        None => query.order_by_desc(posted_invoice::Column::Id),
    };
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let customer_ids: Vec<i64> = models.iter().map(|p| p.customer_id).collect();
    let parties = crate::logic::bill_to::parties_in_order(db, &customer_ids).await;
    let customers = PartyLabels::load(db, &parties).await;
    let journal_ids: Vec<i64> = models.iter().map(|p| p.journal_id).collect();
    let currency_fmts = load_journal_currency_formats(db, &journal_ids).await;
    let fallback = CurrencyFormat::fallback();
    let mut rows = Vec::with_capacity(models.len());
    for (p, party) in models.into_iter().zip(parties) {
        let open = posted_invoice_open_balance(db, p.id)
            .await
            .unwrap_or(rust_decimal::Decimal::ZERO);
        let fmt = currency_fmts.get(&p.journal_id).unwrap_or(&fallback);
        let metrics = posted_invoice_list_metrics(db, p.id).await;
        let (untaxed_amount, total_amount, tax_levied, product_count, final_due_date) =
            format_metrics(&metrics, fmt, dates);
        rows.push(InvoiceRow {
            id: p.id,
            addon_key: Some(InvoiceAddonKey::Posted(p.id)),
            number: p.number,
            datetime: dates.datetime(p.datetime, tz),
            delivery_date: format_hub_delivery_date(p.delivery_date, dates),
            detail_href: PostedInvoiceDetailRouteTag::new(p.id).url(),
            customer_name: customers.name(party),
            open_balance: fmt.display(open),
            selectable: true,
            untaxed_amount,
            total_amount,
            tax_levied,
            product_count,
            final_due_date,
            extra_cells: Vec::new(),
        });
    }
    (rows, page_num, total)
}

async fn query_cancelled_rows(
    db: &sea_orm::DatabaseConnection,
    customer_id: Option<i64>,
    q: &HubQuery,
    params: &HubQueryParams<'_>,
    env: &LarivEnvironment,
    tz: &str,
    dates: &InvoiceDateFormats,
    applied: &AppliedFilters,
) -> (Vec<InvoiceRow>, u32, u64) {
    let page_num = q.page.get();
    let mut query = CancelledInvoiceEntity::find();
    query = filter_customer(query, customer_id, cancelled_invoice::Column::CustomerId);
    query = query.filter(applied.condition.clone());
    query = apply_extra_sql_filters(query, params, "cancelled_invoices.posted_invoice_id");
    query = apply_fiscal_year_datetime_filter(query, env, cancelled_invoice::Column::Datetime);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let parsed_sort = parse_hub_sort(sort);
    let metric_sort = parsed_sort.filter(|(key, _)| cancelled_needs_metric_sort(*key));
    if !applied.post.is_empty() || metric_sort.is_some() {
        if metric_sort.is_none() {
            query = match parsed_sort {
                Some((key, desc)) => apply_cancelled_sql_sort(query, key, desc),
                None => query.order_by_desc(cancelled_invoice::Column::Id),
            };
        }
        return cancelled_rows_metric_sorted(
            db,
            query,
            page_num,
            q.page_size.get(),
            tz,
            dates,
            metric_sort,
            &applied.post,
        )
        .await;
    }
    query = match parsed_sort {
        Some((key, desc)) => apply_cancelled_sql_sort(query, key, desc),
        None => query.order_by_desc(cancelled_invoice::Column::Id),
    };
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (
        cancelled_models_to_rows(db, &models, tz, dates).await,
        page_num,
        total,
    )
}

async fn cancelled_rows_metric_sorted(
    db: &sea_orm::DatabaseConnection,
    query: Select<cancelled_invoice::Entity>,
    page_num: u32,
    page_size: u32,
    tz: &str,
    dates: &InvoiceDateFormats,
    sort: Option<(HubSortKey, bool)>,
    post: &[lariv_core::list_filters::FilterClause],
) -> (Vec<InvoiceRow>, u32, u64) {
    let mut models = query.all(db).await.unwrap_or_default();
    let mut keyed = Vec::with_capacity(models.len());
    for m in models.drain(..) {
        let metrics = cancelled_invoice_list_metrics(db, m.id).await;
        if !matches_in_memory(post, &metrics) {
            continue;
        }
        keyed.push((m, metrics));
    }
    if let Some((key, desc)) = sort {
        keyed.sort_by(|(a, am), (b, bm)| {
            cmp_metrics(am, bm, key, desc).then_with(|| a.id.cmp(&b.id))
        });
    }
    let total = keyed.len() as u64;
    let start = ((page_num as usize).saturating_sub(1)).saturating_mul(page_size as usize);
    let page_models: Vec<_> = keyed
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .map(|(m, _)| m)
        .collect();
    (
        cancelled_models_to_rows(db, &page_models, tz, dates).await,
        page_num,
        total,
    )
}

async fn cancelled_models_to_rows(
    db: &sea_orm::DatabaseConnection,
    models: &[cancelled_invoice::Model],
    tz: &str,
    dates: &InvoiceDateFormats,
) -> Vec<InvoiceRow> {
    let journal_ids: Vec<i64> = models.iter().map(|c| c.journal_id).collect();
    let currency_fmts = load_journal_currency_formats(db, &journal_ids).await;
    let fallback = CurrencyFormat::fallback();
    let mut rows = Vec::with_capacity(models.len());
    for c in models {
        let (customer_name, open_balance, _) = hub_row_extras_none();
        let fmt = currency_fmts.get(&c.journal_id).unwrap_or(&fallback);
        let metrics = cancelled_invoice_list_metrics(db, c.id).await;
        let (untaxed_amount, total_amount, tax_levied, product_count, final_due_date) =
            format_metrics(&metrics, fmt, dates);
        rows.push(InvoiceRow {
            id: c.id,
            addon_key: Some(InvoiceAddonKey::Posted(c.posted_invoice_id)),
            number: c.number.clone(),
            datetime: dates.datetime(c.datetime, tz),
            delivery_date: format_hub_delivery_date(c.delivery_date, dates),
            detail_href: CancelledInvoiceDetailRouteTag::new(c.id).url(),
            customer_name,
            open_balance,
            selectable: true,
            untaxed_amount,
            total_amount,
            tax_levied,
            product_count,
            final_due_date,
            extra_cells: Vec::new(),
        });
    }
    rows
}

struct HubTableRequest<'a> {
    tab: &'a str,
    customer_id: Option<i64>,
    q: &'a HubQuery,
    query: &'a str,
    cookie_header: Option<&'a str>,
    timezone: &'a str,
    path_and_query: String,
    table_key: SwapKey,
    filter_url: String,
    push_history: bool,
    heading: String,
}

async fn load_hub_page(
    db: &sea_orm::DatabaseConnection,
    req: HubTableRequest<'_>,
) -> InvoiceHubPage {
    let params = HubQueryParams::new(req.query);
    let env = LarivEnvironment::from_cookie_header(req.cookie_header);
    let dates = load_invoice_date_formats(db).await;
    let clauses = parse_list_filters(req.query);
    let fields = hub_filter_fields(HubFilterKind::from_tab(req.tab), req.timezone);
    let applied = apply_filters(db.get_database_backend(), &fields, &clauses);
    let (mut rows, page_num, total) = match req.tab {
        "posted" => {
            query_posted_rows(
                db,
                req.customer_id,
                req.q,
                &params,
                &env,
                req.timezone,
                &dates,
                &applied,
            )
            .await
        }
        "cancelled" => {
            query_cancelled_rows(
                db,
                req.customer_id,
                req.q,
                &params,
                &env,
                req.timezone,
                &dates,
                &applied,
            )
            .await
        }
        "drafts" => {
            query_draft_rows(
                db,
                req.customer_id,
                req.q,
                &params,
                &env,
                req.timezone,
                &dates,
                &applied,
            )
            .await
        }
        _ => {
            query_quotation_rows(
                db,
                req.customer_id,
                req.q,
                &params,
                &env,
                req.timezone,
                &dates,
                &applied,
            )
            .await
        }
    };
    let fiscal_years = list_fiscal_year_options()
        .into_iter()
        .map(|(start_year, label)| crate::components::FiscalYearOption { start_year, label })
        .collect();
    let selected_fiscal_year_start = selected_fiscal_year_start_for_ui(&env);
    let extra_columns = enrich_hub_rows(db, &mut rows).await;
    let filters = filter_clause_views(db, &clauses, &applied.errors).await;
    let extra_filters = render_extra_filter_inputs(db, &params).await.into_string();
    let invoices = ObjectList::from_page(rows, page_num, req.q.page_size.get(), total);
    InvoiceHubPage {
        invoices,
        tab: req.tab.to_string(),
        sort: req.q.sort.clone().unwrap_or_default(),
        path_and_query: req.path_and_query,
        fiscal_years,
        selected_fiscal_year_start,
        can_edit: lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<
                crate::routes::FinanceInvoicesMutate,
            >(),
        ),
        extra_columns,
        page_size: req.q.page_size.get(),
        fields,
        filters,
        filter_errors: applied.errors,
        extra_filters,
        table_key: req.table_key,
        filter_url: req.filter_url,
        push_history: req.push_history,
        heading: req.heading,
        create_customer_id: req.customer_id.filter(|id| *id > 0),
    }
}

/// One invoice-hub table constrained to a customer, with that tab's columns and filters.
pub struct CustomerHubTable<'a> {
    pub customer_id: i64,
    pub tab: &'a str,
    pub table_key: SwapKey,
    pub filter_url: &'a str,
    pub heading: &'a str,
    pub query: &'a str,
    pub path_and_query: &'a str,
    pub cookie_header: Option<&'a str>,
    pub timezone: &'a str,
    /// Alpine selection root. The journal page includes it; later HTMX swaps return the table only.
    pub embed_selection: bool,
}

pub async fn customer_hub_table(
    db: &sea_orm::DatabaseConnection,
    spec: CustomerHubTable<'_>,
    q: &HubQuery,
) -> maud::Markup {
    let page = load_hub_page(
        db,
        HubTableRequest {
            tab: spec.tab,
            customer_id: Some(spec.customer_id),
            q,
            query: spec.query,
            cookie_header: spec.cookie_header,
            timezone: spec.timezone,
            path_and_query: spec.path_and_query.to_string(),
            table_key: spec.table_key,
            filter_url: spec.filter_url.to_string(),
            push_history: false,
            heading: spec.heading.to_string(),
        },
    )
    .await;
    page.render_hub_table(spec.embed_selection)
}

pub async fn hub(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    headers: HeaderMap,
    uri: Uri,
    Query(q): Query<HubQuery>,
) -> maud::Markup {
    let tab = match q.tab.as_deref().unwrap_or("quotations") {
        "paid" | "partial" => "posted",
        other => other,
    };
    let page = load_hub_page(
        &state.db,
        HubTableRequest {
            tab,
            customer_id: None,
            q: &q,
            query: uri.query().unwrap_or(""),
            cookie_header: cookie_header(&headers),
            timezone: &ctx.timezone,
            path_and_query: path_and_query(&uri),
            table_key: INVOICE_HUB_TABLE,
            filter_url: String::new(),
            push_history: true,
            heading: String::new(),
        },
    )
    .await;
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets(INVOICE_HUB_TABLE) {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}
