use std::collections::HashMap;

use axum::{
    extract::Query,
    http::Uri,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use crate::{
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::users::middleware::RequireAuth,
    template::RenderAppPane,
    web::{Htmx, html_built_page_with_slots},
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
    hub_table_addon::enrich_hub_rows,
    keys::InvoiceHubTableKey,
    logic::{
        InvoiceListMetrics, cancelled_invoice_list_metrics, draft_invoice_list_metrics,
        format_delivery_date, format_invoice_date, posted_invoice_list_metrics,
        posted_invoice_list_metrics_map, posted_invoice_open_balance,
    },
    scope::{
        parse_filter_datetime, sql_draft_not_posted, sql_posted_not_cancelled,
        sql_posted_not_fully_paid, sql_posted_not_partially_paid,
        sql_settlement_posted_not_cancelled,
    },
    state::InvoicesState,
    templates::{InvoiceHubPage, InvoiceRow},
};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

fn hub_row_extras_none() -> (String, String, bool) {
    (String::new(), String::new(), false)
}

fn format_hub_delivery_date(d: Option<chrono::NaiveDate>) -> String {
    let s = format_delivery_date(d);
    if s.is_empty() {
        "—".to_string()
    } else {
        s
    }
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

async fn query_draft_rows(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
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
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("ID DESC") => {
            query.order_by_desc(draft_invoice::Column::Id)
        }
        s if s.eq_ignore_ascii_case("ID ASC") || s.eq_ignore_ascii_case("ID") => {
            query.order_by_asc(draft_invoice::Column::Id)
        }
        s if s.eq_ignore_ascii_case("Number DESC") => {
            query.order_by_desc(draft_invoice::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Number ASC") || s.eq_ignore_ascii_case("Number") => {
            query.order_by_asc(draft_invoice::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Date DESC") => {
            query.order_by_desc(draft_invoice::Column::Datetime)
        }
        s if s.eq_ignore_ascii_case("Date ASC") || s.eq_ignore_ascii_case("Date") => {
            query.order_by_asc(draft_invoice::Column::Datetime)
        }
        s if s.eq_ignore_ascii_case("DeliveryDate DESC") => {
            query.order_by_desc(draft_invoice::Column::DeliveryDate)
        }
        s if s.eq_ignore_ascii_case("DeliveryDate ASC")
            || s.eq_ignore_ascii_case("DeliveryDate") =>
        {
            query.order_by_asc(draft_invoice::Column::DeliveryDate)
        }
        _ => query.order_by_desc(draft_invoice::Column::Datetime),
    };
    let paginator = query.paginate(db, PAGE_SIZE as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
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
            number: d.number.unwrap_or_else(|| "—".to_string()),
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
    (rows, page_num, total)
}

async fn query_posted_rows(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
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
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("ID DESC") => {
            query.order_by_desc(posted_invoice::Column::Id)
        }
        s if s.eq_ignore_ascii_case("ID ASC") || s.eq_ignore_ascii_case("ID") => {
            query.order_by_asc(posted_invoice::Column::Id)
        }
        s if s.eq_ignore_ascii_case("Number DESC") => {
            query.order_by_desc(posted_invoice::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Number ASC") || s.eq_ignore_ascii_case("Number") => {
            query.order_by_asc(posted_invoice::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Date DESC") => {
            query.order_by_desc(posted_invoice::Column::Datetime)
        }
        s if s.eq_ignore_ascii_case("Date ASC") || s.eq_ignore_ascii_case("Date") => {
            query.order_by_asc(posted_invoice::Column::Datetime)
        }
        s if s.eq_ignore_ascii_case("DeliveryDate DESC") => {
            query.order_by_desc(posted_invoice::Column::DeliveryDate)
        }
        s if s.eq_ignore_ascii_case("DeliveryDate ASC")
            || s.eq_ignore_ascii_case("DeliveryDate") =>
        {
            query.order_by_asc(posted_invoice::Column::DeliveryDate)
        }
        _ => query.order_by_desc(posted_invoice::Column::Datetime),
    };
    let paginator = query.paginate(db, PAGE_SIZE as u64);
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
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("ID DESC") => {
            query.order_by_desc(cancelled_invoice::Column::Id)
        }
        s if s.eq_ignore_ascii_case("ID ASC") || s.eq_ignore_ascii_case("ID") => {
            query.order_by_asc(cancelled_invoice::Column::Id)
        }
        s if s.eq_ignore_ascii_case("Number DESC") => {
            query.order_by_desc(cancelled_invoice::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Number ASC") || s.eq_ignore_ascii_case("Number") => {
            query.order_by_asc(cancelled_invoice::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Date DESC") => {
            query.order_by_desc(cancelled_invoice::Column::Datetime)
        }
        s if s.eq_ignore_ascii_case("Date ASC") || s.eq_ignore_ascii_case("Date") => {
            query.order_by_asc(cancelled_invoice::Column::Datetime)
        }
        s if s.eq_ignore_ascii_case("DeliveryDate DESC") => {
            query.order_by_desc(cancelled_invoice::Column::DeliveryDate)
        }
        s if s.eq_ignore_ascii_case("DeliveryDate ASC")
            || s.eq_ignore_ascii_case("DeliveryDate") =>
        {
            query.order_by_asc(cancelled_invoice::Column::DeliveryDate)
        }
        _ => query.order_by_desc(cancelled_invoice::Column::Datetime),
    };
    let paginator = query.paginate(db, PAGE_SIZE as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
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
            number: c.number,
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
    (rows, page_num, total)
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
    query = match sort {
        s if s.eq_ignore_ascii_case("ID DESC") => {
            query.order_by_desc(paid_invoice::Column::Id)
        }
        s if s.eq_ignore_ascii_case("ID ASC") || s.eq_ignore_ascii_case("ID") => {
            query.order_by_asc(paid_invoice::Column::Id)
        }
        s if s.eq_ignore_ascii_case("Number DESC") => {
            query.order_by_desc(paid_invoice::Column::PostedInvoiceId)
        }
        s if s.eq_ignore_ascii_case("Number ASC") || s.eq_ignore_ascii_case("Number") => {
            query.order_by_asc(paid_invoice::Column::PostedInvoiceId)
        }
        s if s.eq_ignore_ascii_case("Date DESC") => {
            query.order_by_desc(paid_invoice::Column::PaymentId)
        }
        s if s.eq_ignore_ascii_case("Date ASC") || s.eq_ignore_ascii_case("Date") => {
            query.order_by_asc(paid_invoice::Column::PaymentId)
        }
        _ => query.order_by_desc(paid_invoice::Column::Id),
    };
    let paginator = query.paginate(db, PAGE_SIZE as u64);
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
        let (customer_name, open_balance, selectable) = hub_row_extras_none();
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
            selectable,
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
    query = match sort {
        s if s.eq_ignore_ascii_case("ID DESC") => {
            query.order_by_desc(partially_paid_invoice::Column::Id)
        }
        s if s.eq_ignore_ascii_case("ID ASC") || s.eq_ignore_ascii_case("ID") => {
            query.order_by_asc(partially_paid_invoice::Column::Id)
        }
        s if s.eq_ignore_ascii_case("Number DESC") => {
            query.order_by_desc(partially_paid_invoice::Column::PostedInvoiceId)
        }
        s if s.eq_ignore_ascii_case("Number ASC") || s.eq_ignore_ascii_case("Number") => {
            query.order_by_asc(partially_paid_invoice::Column::PostedInvoiceId)
        }
        s if s.eq_ignore_ascii_case("Date DESC") => {
            query.order_by_desc(partially_paid_invoice::Column::PaymentId)
        }
        s if s.eq_ignore_ascii_case("Date ASC") || s.eq_ignore_ascii_case("Date") => {
            query.order_by_asc(partially_paid_invoice::Column::PaymentId)
        }
        _ => query.order_by_desc(partially_paid_invoice::Column::Id),
    };
    let paginator = query.paginate(db, PAGE_SIZE as u64);
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
        let (customer_name, open_balance, selectable) = hub_row_extras_none();
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
            selectable,
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
    uri: Uri,
    Query(q): Query<HubQuery>,
) -> maud::Markup {
    let tab = q.tab.as_deref().unwrap_or("drafts");

    let (mut rows, page_num, total) = match tab {
        "posted" => query_posted_rows(&state.db, &q, &ctx.timezone).await,
        "cancelled" => query_cancelled_rows(&state.db, &q, &ctx.timezone).await,
        "paid" => query_paid_rows(&state.db, &q, &ctx.timezone).await,
        "partial" => query_partial_rows(&state.db, &q, &ctx.timezone).await,
        _ => query_draft_rows(&state.db, &q, &ctx.timezone).await,
    };

    let extra_columns = enrich_hub_rows(&state.db, &mut rows).await;
    let invoices = ObjectList::from_page(rows, page_num, PAGE_SIZE, total);
    let page = InvoiceHubPage {
        invoices,
        tab: tab.to_string(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: require_superuser(&ctx),
        extra_columns,
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
