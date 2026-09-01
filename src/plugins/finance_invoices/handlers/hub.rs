use std::collections::HashMap;

use axum::{
    extract::Query,
    http::{HeaderMap, Uri},
};
use sea_orm::sea_query::SimpleExpr;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Select};

use crate::{
    components::{ObjectList, SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::users::middleware::RequireAuth,
    template::RenderAppPane,
    web::{Htmx, QueryPageSize, html_built_page_with_slots},
};

use crate::plugins::customer::entities::customer::{self, Entity as CustomerEntity};
use crate::plugins::finance_accounts::scope::{
    CurrencyFormat, load_default_currency_format, load_journal_currency_formats,
    load_journal_entry_currency_formats,
};
use crate::plugins::finance_common::require_superuser;

use crate::plugins::finance_invoices::{
    entities::{
        cancelled_invoice::{self, Entity as CancelledInvoiceEntity},
        draft_invoice::{self, Entity as DraftInvoiceEntity},
        paid_invoice::{self, Entity as PaidInvoiceEntity},
        partially_paid_invoice::{self, Entity as PartiallyPaidInvoiceEntity},
        payment::{self, Entity as PaymentEntity},
        posted_invoice::{self, Entity as PostedInvoiceEntity},
    },
    hub_sort::{
        HubSortKey, expr_ar_amount, expr_customer, expr_line_product_count, expr_line_untaxed,
        expr_open_balance, expr_posted_final_due, expr_settlement_ar_amount,
        expr_settlement_final_due, expr_settlement_payment_datetime,
        expr_settlement_posted_delivery, expr_settlement_posted_number,
        expr_settlement_product_count, expr_settlement_tax_levied_approx, expr_settlement_untaxed,
        expr_tax_levied_approx, parse_hub_sort, sort_order,
    },
    hub_table_addon::enrich_hub_rows,
    keys::InvoiceHubTableKey,
    logic::{
        InvoiceListMetrics, cancelled_invoice_list_metrics, draft_invoice_list_metrics,
        format_delivery_date, format_invoice_date, posted_invoice_list_metrics,
        posted_invoice_list_metrics_map, posted_invoice_open_balance,
    },
    scope::{
        LarivEnvironment, list_fiscal_year_options, parse_filter_datetime,
        resolve_list_fiscal_year, selected_fiscal_year_start_for_ui, sql_draft_not_posted,
        sql_posted_not_cancelled, sql_posted_not_fully_paid, sql_posted_not_partially_paid,
        sql_settlement_posted_not_cancelled,
    },
    state::InvoicesState,
    templates::{InvoiceHubPage, InvoiceRow},
};

fn hub_row_extras_none() -> (String, String, bool) {
    (String::new(), String::new(), false)
}

fn format_hub_delivery_date(d: Option<chrono::NaiveDate>) -> String {
    let s = format_delivery_date(d);
    if s.is_empty() { "—".to_string() } else { s }
}

fn format_metrics(
    metrics: &InvoiceListMetrics,
    fmt: &CurrencyFormat,
) -> (String, String, String, String, String) {
    let final_due = metrics
        .final_due
        .map(crate::datetime::format_date)
        .unwrap_or_else(|| "—".to_string());
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
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: QueryPageSize,
    #[serde(default, rename = "DatetimeFrom")]
    pub datetime_from: Option<String>,
    #[serde(default, rename = "DatetimeTo")]
    pub datetime_to: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
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
        // Metric-sorted keys and posted-only columns fall through to default.
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

fn apply_settlement_sql_sort<E>(
    query: Select<E>,
    settlement_table: &str,
    key: HubSortKey,
    desc: bool,
) -> Select<E>
where
    E: EntityTrait,
{
    match key {
        HubSortKey::Number => {
            order_by_expr(query, expr_settlement_posted_number(settlement_table), desc)
        }
        HubSortKey::Date => order_by_expr(
            query,
            expr_settlement_payment_datetime(settlement_table),
            desc,
        ),
        HubSortKey::DeliveryDate => order_by_expr(
            query,
            expr_settlement_posted_delivery(settlement_table),
            desc,
        ),
        HubSortKey::UntaxedAmount => {
            order_by_expr(query, expr_settlement_untaxed(settlement_table), desc)
        }
        HubSortKey::TotalAmount => {
            order_by_expr(query, expr_settlement_ar_amount(settlement_table), desc)
        }
        HubSortKey::TaxLevied => order_by_expr(
            query,
            expr_settlement_tax_levied_approx(settlement_table),
            desc,
        ),
        HubSortKey::ProductCount => {
            order_by_expr(query, expr_settlement_product_count(settlement_table), desc)
        }
        HubSortKey::FinalDueDate => {
            order_by_expr(query, expr_settlement_final_due(settlement_table), desc)
        }
        // Id handled by callers (entity-specific column).
        _ => query,
    }
}

async fn query_draft_rows(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    env: &LarivEnvironment,
    tz: &str,
) -> (Vec<InvoiceRow>, u32, u64) {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query = DraftInvoiceEntity::find().filter(sql_draft_not_posted());
    if let Some(t) = q.datetime_from.as_deref().and_then(parse_filter_datetime) {
        query = query.filter(draft_invoice::Column::Datetime.gte(t));
    }
    if let Some(t) = q.datetime_to.as_deref().and_then(parse_filter_datetime) {
        query = query.filter(draft_invoice::Column::Datetime.lte(t));
    }
    query = apply_fiscal_year_datetime_filter(query, env, draft_invoice::Column::Datetime);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    if let Some((key, desc)) = parse_hub_sort(sort) {
        if draft_needs_metric_sort(key) {
            return draft_rows_metric_sorted(db, query, page_num, q.page_size.get(), tz, key, desc)
                .await;
        }
        query = apply_draft_sql_sort(query, key, desc);
    } else {
        query = query.order_by_desc(draft_invoice::Column::Id);
    }
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (draft_models_to_rows(db, &models, tz).await, page_num, total)
}

async fn draft_rows_metric_sorted(
    db: &sea_orm::DatabaseConnection,
    query: Select<draft_invoice::Entity>,
    page_num: u32,
    page_size: u32,
    tz: &str,
    key: HubSortKey,
    desc: bool,
) -> (Vec<InvoiceRow>, u32, u64) {
    let mut models = query.all(db).await.unwrap_or_default();
    let total = models.len() as u64;
    let mut keyed = Vec::with_capacity(models.len());
    for m in models.drain(..) {
        let metrics = draft_invoice_list_metrics(db, m.id, tz).await;
        keyed.push((m, metrics));
    }
    keyed.sort_by(|(a, am), (b, bm)| cmp_metrics(am, bm, key, desc).then_with(|| a.id.cmp(&b.id)));
    let start = ((page_num as usize).saturating_sub(1)).saturating_mul(page_size as usize);
    let page_models: Vec<_> = keyed
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .map(|(m, _)| m)
        .collect();
    (
        draft_models_to_rows(db, &page_models, tz).await,
        page_num,
        total,
    )
}

async fn draft_models_to_rows(
    db: &sea_orm::DatabaseConnection,
    models: &[draft_invoice::Model],
    tz: &str,
) -> Vec<InvoiceRow> {
    let currency = load_default_currency_format(db).await;
    let mut rows = Vec::with_capacity(models.len());
    for d in models {
        let (customer_name, open_balance, _) = hub_row_extras_none();
        let metrics = draft_invoice_list_metrics(db, d.id, tz).await;
        let (untaxed_amount, total_amount, tax_levied, product_count, final_due_date) =
            format_metrics(&metrics, &currency);
        rows.push(InvoiceRow {
            id: d.id,
            draft_invoice_id: Some(d.id),
            number: d.number.clone().unwrap_or_else(|| "—".to_string()),
            datetime: format_invoice_date(d.datetime, tz),
            delivery_date: format_hub_delivery_date(d.delivery_date),
            detail_href: format!("/finance-invoices/i/{}/", d.id),
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

async fn query_posted_rows(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    env: &LarivEnvironment,
    tz: &str,
) -> (Vec<InvoiceRow>, u32, u64) {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query = PostedInvoiceEntity::find()
        .filter(sql_posted_not_cancelled())
        .filter(sql_posted_not_fully_paid())
        .filter(sql_posted_not_partially_paid());
    if let Some(t) = q.datetime_from.as_deref().and_then(parse_filter_datetime) {
        query = query.filter(posted_invoice::Column::Datetime.gte(t));
    }
    if let Some(t) = q.datetime_to.as_deref().and_then(parse_filter_datetime) {
        query = query.filter(posted_invoice::Column::Datetime.lte(t));
    }
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
    let customers = if customer_ids.is_empty() {
        HashMap::new()
    } else {
        CustomerEntity::find()
            .filter(customer::Column::Id.is_in(customer_ids))
            .all(db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|c| (c.id, c.name))
            .collect()
    };
    let journal_ids: Vec<i64> = models.iter().map(|p| p.journal_id).collect();
    let currency_fmts = load_journal_currency_formats(db, &journal_ids).await;
    let fallback = CurrencyFormat::fallback();
    let mut rows = Vec::with_capacity(models.len());
    for p in models {
        let open = posted_invoice_open_balance(db, p.id)
            .await
            .unwrap_or(rust_decimal::Decimal::ZERO);
        let fmt = currency_fmts.get(&p.journal_id).unwrap_or(&fallback);
        let metrics = posted_invoice_list_metrics(db, p.id).await;
        let (untaxed_amount, total_amount, tax_levied, product_count, final_due_date) =
            format_metrics(&metrics, fmt);
        rows.push(InvoiceRow {
            id: p.id,
            draft_invoice_id: Some(p.draft_invoice_id),
            number: p.number,
            datetime: format_invoice_date(p.datetime, tz),
            delivery_date: format_hub_delivery_date(p.delivery_date),
            detail_href: format!("/finance-invoices/posted/{}/", p.id),
            customer_name: customers
                .get(&p.customer_id)
                .cloned()
                .unwrap_or_else(|| "—".into()),
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
    q: &HubQuery,
    env: &LarivEnvironment,
    tz: &str,
) -> (Vec<InvoiceRow>, u32, u64) {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query = CancelledInvoiceEntity::find();
    if let Some(t) = q.datetime_from.as_deref().and_then(parse_filter_datetime) {
        query = query.filter(cancelled_invoice::Column::Datetime.gte(t));
    }
    if let Some(t) = q.datetime_to.as_deref().and_then(parse_filter_datetime) {
        query = query.filter(cancelled_invoice::Column::Datetime.lte(t));
    }
    query = apply_fiscal_year_datetime_filter(query, env, cancelled_invoice::Column::Datetime);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    if let Some((key, desc)) = parse_hub_sort(sort) {
        if cancelled_needs_metric_sort(key) {
            return cancelled_rows_metric_sorted(
                db,
                query,
                page_num,
                q.page_size.get(),
                tz,
                key,
                desc,
            )
            .await;
        }
        query = apply_cancelled_sql_sort(query, key, desc);
    } else {
        query = query.order_by_desc(cancelled_invoice::Column::Id);
    }
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (
        cancelled_models_to_rows(db, &models, tz).await,
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
    key: HubSortKey,
    desc: bool,
) -> (Vec<InvoiceRow>, u32, u64) {
    let mut models = query.all(db).await.unwrap_or_default();
    let total = models.len() as u64;
    let mut keyed = Vec::with_capacity(models.len());
    for m in models.drain(..) {
        let metrics = cancelled_invoice_list_metrics(db, m.id).await;
        keyed.push((m, metrics));
    }
    keyed.sort_by(|(a, am), (b, bm)| cmp_metrics(am, bm, key, desc).then_with(|| a.id.cmp(&b.id)));
    let start = ((page_num as usize).saturating_sub(1)).saturating_mul(page_size as usize);
    let page_models: Vec<_> = keyed
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .map(|(m, _)| m)
        .collect();
    (
        cancelled_models_to_rows(db, &page_models, tz).await,
        page_num,
        total,
    )
}

async fn cancelled_models_to_rows(
    db: &sea_orm::DatabaseConnection,
    models: &[cancelled_invoice::Model],
    tz: &str,
) -> Vec<InvoiceRow> {
    let journal_ids: Vec<i64> = models.iter().map(|c| c.journal_id).collect();
    let currency_fmts = load_journal_currency_formats(db, &journal_ids).await;
    let fallback = CurrencyFormat::fallback();
    let posted_ids: Vec<i64> = models.iter().map(|c| c.posted_invoice_id).collect();
    let draft_by_posted = load_posted_draft_invoice_ids(db, &posted_ids).await;
    let mut rows = Vec::with_capacity(models.len());
    for c in models {
        let (customer_name, open_balance, _) = hub_row_extras_none();
        let fmt = currency_fmts.get(&c.journal_id).unwrap_or(&fallback);
        let metrics = cancelled_invoice_list_metrics(db, c.id).await;
        let (untaxed_amount, total_amount, tax_levied, product_count, final_due_date) =
            format_metrics(&metrics, fmt);
        rows.push(InvoiceRow {
            id: c.id,
            draft_invoice_id: draft_by_posted.get(&c.posted_invoice_id).copied(),
            number: c.number.clone(),
            datetime: format_invoice_date(c.datetime, tz),
            delivery_date: format_hub_delivery_date(c.delivery_date),
            detail_href: format!("/finance-invoices/cancelled/{}/", c.id),
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

async fn load_posted_draft_invoice_ids(
    db: &sea_orm::DatabaseConnection,
    posted_ids: &[i64],
) -> HashMap<i64, i64> {
    if posted_ids.is_empty() {
        return HashMap::new();
    }
    PostedInvoiceEntity::find()
        .filter(posted_invoice::Column::Id.is_in(posted_ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|inv| (inv.id, inv.draft_invoice_id))
        .collect()
}

async fn load_posted_invoice_delivery_dates(
    db: &sea_orm::DatabaseConnection,
    posted_ids: &[i64],
) -> HashMap<i64, Option<chrono::NaiveDate>> {
    if posted_ids.is_empty() {
        return HashMap::new();
    }
    PostedInvoiceEntity::find()
        .filter(posted_invoice::Column::Id.is_in(posted_ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|inv| (inv.id, inv.delivery_date))
        .collect()
}

async fn load_posted_invoice_labels(
    db: &sea_orm::DatabaseConnection,
    ids: &[i64],
) -> HashMap<i64, String> {
    if ids.is_empty() {
        return HashMap::new();
    }
    PostedInvoiceEntity::find()
        .filter(posted_invoice::Column::Id.is_in(ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|inv| {
            let label = if inv.number.is_empty() {
                format!("#{}", inv.id)
            } else {
                inv.number.clone()
            };
            (inv.id, label)
        })
        .collect()
}

async fn load_posted_invoice_journals(
    db: &sea_orm::DatabaseConnection,
    ids: &[i64],
) -> HashMap<i64, i64> {
    if ids.is_empty() {
        return HashMap::new();
    }
    PostedInvoiceEntity::find()
        .filter(posted_invoice::Column::Id.is_in(ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|inv| (inv.id, inv.journal_id))
        .collect()
}

async fn query_paid_rows(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    tz: &str,
) -> (Vec<InvoiceRow>, u32, u64) {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query =
        PaidInvoiceEntity::find().filter(sql_settlement_posted_not_cancelled("paid_invoices"));
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match parse_hub_sort(sort) {
        Some((HubSortKey::Id, true)) => query.order_by_desc(paid_invoice::Column::Id),
        Some((HubSortKey::Id, false)) => query.order_by_asc(paid_invoice::Column::Id),
        Some((HubSortKey::Customer | HubSortKey::OpenBalance, _)) | None => {
            query.order_by_desc(paid_invoice::Column::Id)
        }
        Some((key, desc)) => apply_settlement_sql_sort(query, "paid_invoices", key, desc),
    };
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let payment_ids: Vec<i64> = models.iter().map(|p| p.payment_id).collect();
    let posted_ids: Vec<i64> = models.iter().map(|p| p.posted_invoice_id).collect();
    let payments = PaymentEntity::find()
        .filter(payment::Column::Id.is_in(payment_ids))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|p| (p.id, p))
        .collect::<HashMap<_, _>>();
    let je_ids: Vec<i64> = payments.values().map(|p| p.journal_entry_id).collect();
    let currency_fmts = load_journal_entry_currency_formats(db, &je_ids).await;
    let journal_by_posted = load_posted_invoice_journals(db, &posted_ids).await;
    let journal_ids: Vec<i64> = journal_by_posted.values().copied().collect();
    let posted_currency_fmts = load_journal_currency_formats(db, &journal_ids).await;
    let fallback = CurrencyFormat::fallback();
    let invoice_labels = load_posted_invoice_labels(db, &posted_ids).await;
    let metrics_map = posted_invoice_list_metrics_map(db, &posted_ids).await;
    let draft_by_posted = load_posted_draft_invoice_ids(db, &posted_ids).await;
    let delivery_by_posted = load_posted_invoice_delivery_dates(db, &posted_ids).await;
    let mut rows = Vec::with_capacity(models.len());
    for paid in models {
        let inv_label = invoice_labels
            .get(&paid.posted_invoice_id)
            .cloned()
            .unwrap_or_else(|| format!("#{}", paid.posted_invoice_id));
        let datetime = if let Some(pay) = payments.get(&paid.payment_id) {
            crate::datetime::DatetimeLabel::short(pay.datetime, tz).into_string()
        } else {
            "—".to_string()
        };
        let (customer_name, open_balance, _) = hub_row_extras_none();
        let metrics = metrics_map
            .get(&paid.posted_invoice_id)
            .cloned()
            .unwrap_or_default();
        let fmt = journal_by_posted
            .get(&paid.posted_invoice_id)
            .and_then(|jid| posted_currency_fmts.get(jid))
            .or_else(|| {
                payments
                    .get(&paid.payment_id)
                    .and_then(|pay| currency_fmts.get(&pay.journal_entry_id))
            })
            .unwrap_or(&fallback);
        let (untaxed_amount, total_amount, tax_levied, product_count, final_due_date) =
            format_metrics(&metrics, fmt);
        rows.push(InvoiceRow {
            id: paid.id,
            draft_invoice_id: draft_by_posted.get(&paid.posted_invoice_id).copied(),
            number: inv_label,
            datetime,
            delivery_date: format_hub_delivery_date(
                delivery_by_posted
                    .get(&paid.posted_invoice_id)
                    .copied()
                    .flatten(),
            ),
            detail_href: format!("/finance-invoices/paid/{}/", paid.id),
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
    (rows, page_num, total)
}

async fn query_partial_rows(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    tz: &str,
) -> (Vec<InvoiceRow>, u32, u64) {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query = PartiallyPaidInvoiceEntity::find().filter(sql_settlement_posted_not_cancelled(
        "partially_paid_invoices",
    ));
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match parse_hub_sort(sort) {
        Some((HubSortKey::Id, true)) => query.order_by_desc(partially_paid_invoice::Column::Id),
        Some((HubSortKey::Id, false)) => query.order_by_asc(partially_paid_invoice::Column::Id),
        Some((HubSortKey::Customer | HubSortKey::OpenBalance, _)) | None => {
            query.order_by_desc(partially_paid_invoice::Column::Id)
        }
        Some((key, desc)) => apply_settlement_sql_sort(query, "partially_paid_invoices", key, desc),
    };
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let payment_ids: Vec<i64> = models.iter().map(|p| p.payment_id).collect();
    let posted_ids: Vec<i64> = models.iter().map(|p| p.posted_invoice_id).collect();
    let payments = PaymentEntity::find()
        .filter(payment::Column::Id.is_in(payment_ids))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|p| (p.id, p))
        .collect::<HashMap<_, _>>();
    let je_ids: Vec<i64> = payments.values().map(|p| p.journal_entry_id).collect();
    let currency_fmts = load_journal_entry_currency_formats(db, &je_ids).await;
    let journal_by_posted = load_posted_invoice_journals(db, &posted_ids).await;
    let journal_ids: Vec<i64> = journal_by_posted.values().copied().collect();
    let posted_currency_fmts = load_journal_currency_formats(db, &journal_ids).await;
    let fallback = CurrencyFormat::fallback();
    let invoice_labels = load_posted_invoice_labels(db, &posted_ids).await;
    let metrics_map = posted_invoice_list_metrics_map(db, &posted_ids).await;
    let draft_by_posted = load_posted_draft_invoice_ids(db, &posted_ids).await;
    let delivery_by_posted = load_posted_invoice_delivery_dates(db, &posted_ids).await;
    let mut rows = Vec::with_capacity(models.len());
    for partial in models {
        let inv_label = invoice_labels
            .get(&partial.posted_invoice_id)
            .cloned()
            .unwrap_or_else(|| format!("#{}", partial.posted_invoice_id));
        let datetime = if let Some(pay) = payments.get(&partial.payment_id) {
            crate::datetime::DatetimeLabel::short(pay.datetime, tz).into_string()
        } else {
            "—".to_string()
        };
        let (customer_name, open_balance, _) = hub_row_extras_none();
        let metrics = metrics_map
            .get(&partial.posted_invoice_id)
            .cloned()
            .unwrap_or_default();
        let fmt = journal_by_posted
            .get(&partial.posted_invoice_id)
            .and_then(|jid| posted_currency_fmts.get(jid))
            .or_else(|| {
                payments
                    .get(&partial.payment_id)
                    .and_then(|pay| currency_fmts.get(&pay.journal_entry_id))
            })
            .unwrap_or(&fallback);
        let (untaxed_amount, total_amount, tax_levied, product_count, final_due_date) =
            format_metrics(&metrics, fmt);
        rows.push(InvoiceRow {
            id: partial.id,
            draft_invoice_id: draft_by_posted.get(&partial.posted_invoice_id).copied(),
            number: inv_label,
            datetime,
            delivery_date: format_hub_delivery_date(
                delivery_by_posted
                    .get(&partial.posted_invoice_id)
                    .copied()
                    .flatten(),
            ),
            detail_href: format!("/finance-invoices/partial/{}/", partial.id),
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
    (rows, page_num, total)
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
    let tab = q.tab.as_deref().unwrap_or("drafts");
    let env = LarivEnvironment::from_cookie_header(cookie_header(&headers));

    let (mut rows, page_num, total) = match tab {
        "posted" => query_posted_rows(&state.db, &q, &env, &ctx.timezone).await,
        "cancelled" => query_cancelled_rows(&state.db, &q, &env, &ctx.timezone).await,
        "paid" => query_paid_rows(&state.db, &q, &ctx.timezone).await,
        "partial" => query_partial_rows(&state.db, &q, &ctx.timezone).await,
        _ => query_draft_rows(&state.db, &q, &env, &ctx.timezone).await,
    };

    let fiscal_years =
        list_fiscal_year_options()
            .into_iter()
            .map(|(start_year, label)| {
                crate::plugins::finance_invoices::components::FiscalYearOption { start_year, label }
            })
            .collect();
    let selected_fiscal_year_start = selected_fiscal_year_start_for_ui(&env);

    let extra_columns = enrich_hub_rows(&state.db, &mut rows).await;
    let invoices = ObjectList::from_page(rows, page_num, q.page_size.get(), total);
    let page = InvoiceHubPage {
        invoices,
        tab: tab.to_string(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        fiscal_years,
        selected_fiscal_year_start,
        can_edit: require_superuser(&ctx),
        extra_columns,
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<InvoiceHubTableKey>() {
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
