use std::collections::HashMap;

use axum::{
    extract::Query,
    http::{HeaderMap, Uri},
};
use sea_orm::sea_query::SimpleExpr;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Select};

use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::http::Cap;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{Htmx, QueryPage, QueryPageSize, html_built_page_with_slots};
use lariv_plugin_users::middleware::RequireAuth;

use crate::logic::bill_to::{BillTo, PartyLabels};
use lariv_plugin_finance_accounts::scope::{
    CurrencyFormat, load_default_currency_format, load_journal_currency_formats,
};

use crate::{
    entities::{
        cancelled_purchase::{self, Entity as CancelledPurchaseEntity},
        draft_purchase::{self, Entity as DraftPurchaseEntity},
        posted_purchase::{self, Entity as PostedPurchaseEntity},
    },
    hub_filter::{
        ParsedHubFilters, apply_cancelled_sql_filters, apply_draft_sql_filters,
        apply_posted_sql_filters,
    },
    hub_filter_addon::{
        HubQueryParams, apply_extra_sql_filters, draft_purchase_id_sql_via_posted,
        render_extra_filter_inputs,
    },
    hub_sort::{
        HubSortKey, expr_ar_amount, expr_vendor, expr_line_product_count, expr_line_untaxed,
        expr_open_balance, expr_posted_final_due, expr_tax_levied_approx, parse_hub_sort,
        sort_order,
    },
    hub_table_addon::enrich_hub_rows,
    keys::PurchaseHubTableKey,
    logic::{
        PurchaseDateFormats, PurchaseListMetrics, cancelled_purchase_list_metrics,
        draft_purchase_list_metrics, load_purchase_date_formats, posted_purchase_list_metrics,
    },
    routes::{
        CancelledPurchaseDetailRouteTag, DraftPurchaseDetailRouteTag, PostedPurchaseDetailRouteTag,
    },
    scope::{
        LarivEnvironment, list_fiscal_year_options, resolve_list_fiscal_year,
        selected_fiscal_year_start_for_ui, sql_draft_not_posted, sql_posted_not_cancelled,
    },
    state::PurchasesState,
    templates::{PurchaseHubPage, PurchaseRow},
};

fn hub_row_extras_none() -> (String, String, bool) {
    (String::new(), String::new(), false)
}

fn format_hub_delivery_date(d: Option<chrono::NaiveDate>, dates: &PurchaseDateFormats) -> String {
    dates.calendar_or_dash(d)
}

fn format_metrics(
    metrics: &PurchaseListMetrics,
    fmt: &CurrencyFormat,
    dates: &PurchaseDateFormats,
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
    #[serde(flatten)]
    pub filters: crate::hub_filter::HubFilterInput,
}

async fn hub_product_display(db: &sea_orm::DatabaseConnection, raw: &Option<String>) -> String {
    let Some(id) = raw
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|id| *id > 0)
    else {
        return String::new();
    };
    lariv_core::web::opt_or_log(
        lariv_plugin_finance_products::entities::product::Entity::find_by_id(id)
            .one(db)
            .await,
        "find product for purchase hub filter",
    )
    .map(|product| product.name)
    .unwrap_or_else(|| format!("#{id}"))
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
    mut query: Select<E>,
    env: &LarivEnvironment,
    column: C,
) -> Select<E>
where
    C: ColumnTrait,
    E: EntityTrait,
{
    if let Some(fy) = resolve_list_fiscal_year(env) {
        let (start, end) = fy.datetime_range();
        query = query.filter(column.gte(start)).filter(column.lt(end));
    }
    query
}

fn order_by_expr<E>(query: Select<E>, expr: SimpleExpr, desc: bool) -> Select<E>
where
    E: EntityTrait,
{
    query.order_by(expr, sort_order(desc))
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
    a: &PurchaseListMetrics,
    b: &PurchaseListMetrics,
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
    query: Select<draft_purchase::Entity>,
    key: HubSortKey,
    desc: bool,
) -> Select<draft_purchase::Entity> {
    match key {
        HubSortKey::Id => {
            if desc {
                query.order_by_desc(draft_purchase::Column::Id)
            } else {
                query.order_by_asc(draft_purchase::Column::Id)
            }
        }
        HubSortKey::Number => {
            if desc {
                query.order_by_desc(draft_purchase::Column::Number)
            } else {
                query.order_by_asc(draft_purchase::Column::Number)
            }
        }
        HubSortKey::Date => {
            if desc {
                query.order_by_desc(draft_purchase::Column::Datetime)
            } else {
                query.order_by_asc(draft_purchase::Column::Datetime)
            }
        }
        HubSortKey::DeliveryDate => {
            if desc {
                query.order_by_desc(draft_purchase::Column::DeliveryDate)
            } else {
                query.order_by_asc(draft_purchase::Column::DeliveryDate)
            }
        }
        HubSortKey::UntaxedAmount => order_by_expr(
            query,
            expr_line_untaxed("draft_purchase_lines", "draft_purchase_id", "draft_purchases"),
            desc,
        ),
        HubSortKey::ProductCount => order_by_expr(
            query,
            expr_line_product_count("draft_purchase_lines", "draft_purchase_id", "draft_purchases"),
            desc,
        ),
        // Metric-sorted keys and posted-only columns fall through to default.
        _ => {
            if desc {
                query.order_by_desc(draft_purchase::Column::Datetime)
            } else {
                query.order_by_asc(draft_purchase::Column::Datetime)
            }
        }
    }
}

fn apply_posted_sql_sort(
    query: Select<posted_purchase::Entity>,
    key: HubSortKey,
    desc: bool,
) -> Select<posted_purchase::Entity> {
    match key {
        HubSortKey::Id => {
            if desc {
                query.order_by_desc(posted_purchase::Column::Id)
            } else {
                query.order_by_asc(posted_purchase::Column::Id)
            }
        }
        HubSortKey::Number => {
            if desc {
                query.order_by_desc(posted_purchase::Column::Number)
            } else {
                query.order_by_asc(posted_purchase::Column::Number)
            }
        }
        HubSortKey::Date => {
            if desc {
                query.order_by_desc(posted_purchase::Column::Datetime)
            } else {
                query.order_by_asc(posted_purchase::Column::Datetime)
            }
        }
        HubSortKey::DeliveryDate => {
            if desc {
                query.order_by_desc(posted_purchase::Column::DeliveryDate)
            } else {
                query.order_by_asc(posted_purchase::Column::DeliveryDate)
            }
        }
        HubSortKey::Vendor => order_by_expr(query, expr_vendor("posted_purchases"), desc),
        HubSortKey::OpenBalance => order_by_expr(query, expr_open_balance("posted_purchases"), desc),
        HubSortKey::UntaxedAmount => order_by_expr(
            query,
            expr_line_untaxed(
                "posted_purchase_lines",
                "posted_purchase_id",
                "posted_purchases",
            ),
            desc,
        ),
        HubSortKey::TotalAmount => order_by_expr(query, expr_ar_amount("posted_purchases"), desc),
        HubSortKey::TaxLevied => order_by_expr(
            query,
            expr_tax_levied_approx(
                "posted_purchases",
                "posted_purchase_lines",
                "posted_purchase_id",
            ),
            desc,
        ),
        HubSortKey::ProductCount => order_by_expr(
            query,
            expr_line_product_count(
                "posted_purchase_lines",
                "posted_purchase_id",
                "posted_purchases",
            ),
            desc,
        ),
        HubSortKey::FinalDueDate => {
            order_by_expr(query, expr_posted_final_due("posted_purchases"), desc)
        }
    }
}

fn apply_cancelled_sql_sort(
    query: Select<cancelled_purchase::Entity>,
    key: HubSortKey,
    desc: bool,
) -> Select<cancelled_purchase::Entity> {
    match key {
        HubSortKey::Id => {
            if desc {
                query.order_by_desc(cancelled_purchase::Column::Id)
            } else {
                query.order_by_asc(cancelled_purchase::Column::Id)
            }
        }
        HubSortKey::Number => {
            if desc {
                query.order_by_desc(cancelled_purchase::Column::Number)
            } else {
                query.order_by_asc(cancelled_purchase::Column::Number)
            }
        }
        HubSortKey::Date => {
            if desc {
                query.order_by_desc(cancelled_purchase::Column::Datetime)
            } else {
                query.order_by_asc(cancelled_purchase::Column::Datetime)
            }
        }
        HubSortKey::DeliveryDate => {
            if desc {
                query.order_by_desc(cancelled_purchase::Column::DeliveryDate)
            } else {
                query.order_by_asc(cancelled_purchase::Column::DeliveryDate)
            }
        }
        HubSortKey::UntaxedAmount => order_by_expr(
            query,
            expr_line_untaxed(
                "cancelled_purchase_lines",
                "cancelled_purchase_id",
                "cancelled_purchases",
            ),
            desc,
        ),
        HubSortKey::ProductCount => order_by_expr(
            query,
            expr_line_product_count(
                "cancelled_purchase_lines",
                "cancelled_purchase_id",
                "cancelled_purchases",
            ),
            desc,
        ),
        HubSortKey::FinalDueDate => {
            order_by_expr(query, expr_posted_final_due("cancelled_purchases"), desc)
        }
        _ => {
            if desc {
                query.order_by_desc(cancelled_purchase::Column::Datetime)
            } else {
                query.order_by_asc(cancelled_purchase::Column::Datetime)
            }
        }
    }
}

async fn query_draft_rows(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    params: &HubQueryParams<'_>,
    env: &LarivEnvironment,
    tz: &str,
    dates: &PurchaseDateFormats,
) -> (Vec<PurchaseRow>, u32, u64) {
    let page_num = q.page.get();
    let filters = ParsedHubFilters::from_query(&q.filters, tz);
    if filters.rejected() {
        return (Vec::new(), page_num, 0);
    }
    let mut query = DraftPurchaseEntity::find().filter(sql_draft_not_posted());
    query = apply_draft_sql_filters(query, &filters);
    query = apply_extra_sql_filters(query, params, "draft_purchases.id");
    query = apply_fiscal_year_datetime_filter(query, env, draft_purchase::Column::Datetime);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let parsed_sort = parse_hub_sort(sort);
    let metric_sort = parsed_sort.filter(|(key, _)| draft_needs_metric_sort(*key));
    if filters.needs_draft_metrics() || metric_sort.is_some() {
        if metric_sort.is_none() {
            query = match parsed_sort {
                Some((key, desc)) => apply_draft_sql_sort(query, key, desc),
                None => query.order_by_desc(draft_purchase::Column::Id),
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
            &filters,
        )
        .await;
    }
    query = match parsed_sort {
        Some((key, desc)) => apply_draft_sql_sort(query, key, desc),
        None => query.order_by_desc(draft_purchase::Column::Id),
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
    query: Select<draft_purchase::Entity>,
    page_num: u32,
    page_size: u32,
    tz: &str,
    dates: &PurchaseDateFormats,
    sort: Option<(HubSortKey, bool)>,
    filters: &ParsedHubFilters,
) -> (Vec<PurchaseRow>, u32, u64) {
    let mut models = query.all(db).await.unwrap_or_default();
    let mut keyed = Vec::with_capacity(models.len());
    for m in models.drain(..) {
        let metrics = draft_purchase_list_metrics(db, m.id, tz).await;
        if !filters.matches_draft_metrics(&metrics) {
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
    models: &[draft_purchase::Model],
    tz: &str,
    dates: &PurchaseDateFormats,
) -> Vec<PurchaseRow> {
    let currency = load_default_currency_format(db).await;
    let mut rows = Vec::with_capacity(models.len());
    for d in models {
        let (vendor_name, open_balance, _) = hub_row_extras_none();
        let metrics = draft_purchase_list_metrics(db, d.id, tz).await;
        let (untaxed_amount, total_amount, tax_levied, product_count, final_due_date) =
            format_metrics(&metrics, &currency, dates);
        rows.push(PurchaseRow {
            id: d.id,
            draft_purchase_id: Some(d.id),
            number: d.number.clone().unwrap_or_else(|| "—".to_string()),
            datetime: dates.datetime(d.datetime, tz),
            delivery_date: format_hub_delivery_date(d.delivery_date, dates),
            detail_href: DraftPurchaseDetailRouteTag::new(d.id).url(),
            vendor_name,
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

async fn query_posted_rows(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    params: &HubQueryParams<'_>,
    env: &LarivEnvironment,
    tz: &str,
    dates: &PurchaseDateFormats,
) -> (Vec<PurchaseRow>, u32, u64) {
    let page_num = q.page.get();
    let filters = ParsedHubFilters::from_query(&q.filters, tz);
    if filters.rejected_posted() {
        return (Vec::new(), page_num, 0);
    }
    let mut query = PostedPurchaseEntity::find().filter(sql_posted_not_cancelled());
    query = apply_posted_sql_filters(query, &filters);
    query = apply_extra_sql_filters(query, params, "posted_purchases.draft_purchase_id");
    query = apply_fiscal_year_datetime_filter(query, env, posted_purchase::Column::Datetime);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match parse_hub_sort(sort) {
        Some((key, desc)) => apply_posted_sql_sort(query, key, desc),
        None => query.order_by_desc(posted_purchase::Column::Id),
    };
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let parties: Vec<BillTo> = models
        .iter()
        .map(|p| {
            BillTo::new(
                p.vendor_is_individual,
                p.vendor_contact_id,
                p.vendor_company_id,
            )
        })
        .collect();
    let vendors = PartyLabels::load(db, &parties).await;
    let journal_ids: Vec<i64> = models.iter().map(|p| p.journal_id).collect();
    let currency_fmts = load_journal_currency_formats(db, &journal_ids).await;
    let fallback = CurrencyFormat::fallback();
    let mut rows = Vec::with_capacity(models.len());
    for (p, party) in models.into_iter().zip(parties) {
        let fmt = currency_fmts.get(&p.journal_id).unwrap_or(&fallback);
        let metrics = posted_purchase_list_metrics(db, p.id).await;
        let (untaxed_amount, total_amount, tax_levied, product_count, final_due_date) =
            format_metrics(&metrics, fmt, dates);
        rows.push(PurchaseRow {
            id: p.id,
            draft_purchase_id: Some(p.draft_purchase_id),
            number: p.number,
            datetime: dates.datetime(p.datetime, tz),
            delivery_date: format_hub_delivery_date(p.delivery_date, dates),
            detail_href: PostedPurchaseDetailRouteTag::new(p.id).url(),
            vendor_name: vendors.name(party),
            open_balance: total_amount.clone(),
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
    q: &HubQuery,
    params: &HubQueryParams<'_>,
    env: &LarivEnvironment,
    tz: &str,
    dates: &PurchaseDateFormats,
) -> (Vec<PurchaseRow>, u32, u64) {
    let page_num = q.page.get();
    let filters = ParsedHubFilters::from_query(&q.filters, tz);
    if filters.rejected() {
        return (Vec::new(), page_num, 0);
    }
    let mut query = CancelledPurchaseEntity::find();
    query = apply_cancelled_sql_filters(query, &filters);
    query = apply_extra_sql_filters(
        query,
        params,
        &draft_purchase_id_sql_via_posted("cancelled_purchases"),
    );
    query = apply_fiscal_year_datetime_filter(query, env, cancelled_purchase::Column::Datetime);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let parsed_sort = parse_hub_sort(sort);
    let metric_sort = parsed_sort.filter(|(key, _)| cancelled_needs_metric_sort(*key));
    if filters.needs_cancelled_metrics() || metric_sort.is_some() {
        if metric_sort.is_none() {
            query = match parsed_sort {
                Some((key, desc)) => apply_cancelled_sql_sort(query, key, desc),
                None => query.order_by_desc(cancelled_purchase::Column::Id),
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
            &filters,
        )
        .await;
    }
    query = match parsed_sort {
        Some((key, desc)) => apply_cancelled_sql_sort(query, key, desc),
        None => query.order_by_desc(cancelled_purchase::Column::Id),
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
    query: Select<cancelled_purchase::Entity>,
    page_num: u32,
    page_size: u32,
    tz: &str,
    dates: &PurchaseDateFormats,
    sort: Option<(HubSortKey, bool)>,
    filters: &ParsedHubFilters,
) -> (Vec<PurchaseRow>, u32, u64) {
    let mut models = query.all(db).await.unwrap_or_default();
    let mut keyed = Vec::with_capacity(models.len());
    for m in models.drain(..) {
        let metrics = cancelled_purchase_list_metrics(db, m.id).await;
        if !filters.matches_cancelled_metrics(&metrics) {
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
    models: &[cancelled_purchase::Model],
    tz: &str,
    dates: &PurchaseDateFormats,
) -> Vec<PurchaseRow> {
    let journal_ids: Vec<i64> = models.iter().map(|c| c.journal_id).collect();
    let currency_fmts = load_journal_currency_formats(db, &journal_ids).await;
    let fallback = CurrencyFormat::fallback();
    let posted_ids: Vec<i64> = models.iter().map(|c| c.posted_purchase_id).collect();
    let draft_by_posted = load_posted_draft_purchase_ids(db, &posted_ids).await;
    let mut rows = Vec::with_capacity(models.len());
    for c in models {
        let (vendor_name, open_balance, _) = hub_row_extras_none();
        let fmt = currency_fmts.get(&c.journal_id).unwrap_or(&fallback);
        let metrics = cancelled_purchase_list_metrics(db, c.id).await;
        let (untaxed_amount, total_amount, tax_levied, product_count, final_due_date) =
            format_metrics(&metrics, fmt, dates);
        rows.push(PurchaseRow {
            id: c.id,
            draft_purchase_id: draft_by_posted.get(&c.posted_purchase_id).copied(),
            number: c.number.clone(),
            datetime: dates.datetime(c.datetime, tz),
            delivery_date: format_hub_delivery_date(c.delivery_date, dates),
            detail_href: CancelledPurchaseDetailRouteTag::new(c.id).url(),
            vendor_name,
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

async fn load_posted_draft_purchase_ids(
    db: &sea_orm::DatabaseConnection,
    posted_ids: &[i64],
) -> HashMap<i64, i64> {
    if posted_ids.is_empty() {
        return HashMap::new();
    }
    PostedPurchaseEntity::find()
        .filter(posted_purchase::Column::Id.is_in(posted_ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|inv| (inv.id, inv.draft_purchase_id))
        .collect()
}

pub async fn hub(
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    headers: HeaderMap,
    uri: Uri,
    Query(q): Query<HubQuery>,
) -> maud::Markup {
    let tab = q.tab.as_deref().unwrap_or("drafts");
    let params = HubQueryParams::new(uri.query().unwrap_or(""));
    let env = LarivEnvironment::from_cookie_header(cookie_header(&headers));

    let dates = load_purchase_date_formats(&state.db).await;
    let (mut rows, page_num, total) = match tab {
        "posted" => query_posted_rows(&state.db, &q, &params, &env, &ctx.timezone, &dates).await,
        "cancelled" => {
            query_cancelled_rows(&state.db, &q, &params, &env, &ctx.timezone, &dates).await
        }
        _ => query_draft_rows(&state.db, &q, &params, &env, &ctx.timezone, &dates).await,
    };

    let fiscal_years = list_fiscal_year_options()
        .into_iter()
        .map(|(start_year, label)| crate::components::FiscalYearOption { start_year, label })
        .collect();
    let selected_fiscal_year_start = selected_fiscal_year_start_for_ui(&env);

    let extra_columns = enrich_hub_rows(&state.db, &mut rows).await;
    let product_display = hub_product_display(&state.db, &q.filters.product_id).await;
    let extra_filters = render_extra_filter_inputs(&state.db, &params)
        .await
        .into_string();
    let purchases = ObjectList::from_page(rows, page_num, q.page_size.get(), total);
    let page = PurchaseHubPage {
        purchases,
        tab: tab.to_string(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        fiscal_years,
        selected_fiscal_year_start,
        can_edit: lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<
                crate::routes::FinancePurchasesMutate,
            >(),
        ),
        extra_columns,
        page_size: q.page_size.get(),
        filters: q.filters,
        product_display,
        extra_filters,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<PurchaseHubTableKey>() {
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
