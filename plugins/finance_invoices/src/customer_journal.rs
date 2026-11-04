//! Customer journal tables: quotations, posted invoices, and cancelled invoices.

use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::{Path, Query};
use axum::http::HeaderMap;
use axum::http::Uri;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use maud::{Markup, html};
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

use lariv_core::components::{FieldText, SharedChromeFolder, field_text, label};
use lariv_core::http::Cap;
use lariv_core::web::Htmx;
use lariv_plugin_finance_accounts::scope::{CurrencyFormat, load_default_currency_format};
use lariv_plugin_finance_common::decimal::dec_sub;
use lariv_plugin_finance_customers::handlers::{customer_page_name, respond_customer_page};
use lariv_plugin_finance_customers::journal::{
    CustomerJournalRegistrar, CustomerJournalRegistry, CustomerJournalRender,
    CustomerJournalSection, CustomerSidebarLink, CustomerSidebarRegistry,
};
use lariv_plugin_finance_customers::routes::CustomerListRouteTag;
use lariv_plugin_finance_customers::templates::CustomerSectionPage;
use lariv_plugin_users::middleware::RequireAuth;

use crate::components::fiscal_year_environment_selector;
use crate::handlers::hub::{CustomerHubTable, HubQuery, customer_hub_table};
use crate::keys::{
    CUSTOMER_CANCELLED_INVOICES_TABLE, CUSTOMER_DRAFT_INVOICES_TABLE, CUSTOMER_INVOICES_TABLE,
    CUSTOMER_QUOTATIONS_TABLE,
};
use crate::routes::{
    CustomerJournalCancelledRouteTag, CustomerJournalDraftInvoicesRouteTag,
    CustomerJournalInvoicesRouteTag, CustomerJournalQuotationsRouteTag,
};
use crate::scope::{
    LarivEnvironment, list_fiscal_year_options, resolve_list_fiscal_year,
    selected_fiscal_year_start_for_ui,
};
use crate::state::InvoicesState;

#[derive(Clone, Copy, Default)]
pub struct Hook;

fn quotations_url(customer_id: i64) -> String {
    CustomerJournalQuotationsRouteTag::new(customer_id).url()
}

fn draft_invoices_url(customer_id: i64) -> String {
    CustomerJournalDraftInvoicesRouteTag::new(customer_id).url()
}

fn invoices_url(customer_id: i64) -> String {
    CustomerJournalInvoicesRouteTag::new(customer_id).url()
}

fn cancelled_url(customer_id: i64) -> String {
    CustomerJournalCancelledRouteTag::new(customer_id).url()
}

impl CustomerJournalRegistrar for Hook {
    fn register_customer_journal(
        self,
        registry: CustomerJournalRegistry,
    ) -> CustomerJournalRegistry {
        registry.push(Arc::new(InvoicesSection))
    }

    fn register_customer_sidebar(
        self,
        registry: CustomerSidebarRegistry,
    ) -> CustomerSidebarRegistry {
        registry
            .push(CustomerSidebarLink {
                key: "quotations",
                title: "Quotations",
                url_fn: quotations_url,
                order: 40,
            })
            .push(CustomerSidebarLink {
                key: "draft-invoices",
                title: "Draft invoices",
                url_fn: draft_invoices_url,
                order: 45,
            })
            .push(CustomerSidebarLink {
                key: "invoices",
                title: "Invoices",
                url_fn: invoices_url,
                order: 50,
            })
            .push(CustomerSidebarLink {
                key: "cancelled",
                title: "Cancelled invoices",
                url_fn: cancelled_url,
                order: 60,
            })
    }
}

struct InvoicesSection;

#[async_trait]
impl CustomerJournalSection for InvoicesSection {
    fn order(&self) -> u16 {
        40
    }

    async fn render(&self, ctx: &CustomerJournalRender) -> Markup {
        let env = LarivEnvironment::from_cookie_header(ctx.cookie_header.as_deref());
        let fiscal_years = list_fiscal_year_options()
            .into_iter()
            .map(|(start_year, label)| crate::components::FiscalYearOption { start_year, label })
            .collect::<Vec<_>>();
        let selected = selected_fiscal_year_start_for_ui(&env);
        let totals = journal_totals(ctx, resolve_list_fiscal_year(&env)).await;
        let quotations = table(ctx, "quotations", "Quotations", "").await;
        let drafts = table(ctx, "drafts", "Draft invoices", "").await;
        let invoices = table(ctx, "posted", "Invoices", "").await;
        let cancelled = table(ctx, "cancelled", "Cancelled invoices", "").await;
        html! {
            (fiscal_year_environment_selector(&fiscal_years, selected))
            (journal_totals_fields(&totals))
            div class="flex flex-col gap-8" {
                (quotations)
                (drafts)
                (invoices)
                (cancelled)
            }
        }
    }
}

struct JournalTotals {
    invoiced: String,
    paid: String,
    balance: String,
}

fn journal_totals_fields(totals: &JournalTotals) -> Markup {
    html! {
        div class="mb-4 flex flex-col gap-2 md:flex-row md:gap-6" {
            div class="w-full md:min-w-0 md:flex-1" {
                (label(
                    "Invoiced Amount",
                    field_text(FieldText {
                        value: &totals.invoiced,
                        classes: "",
                    }),
                ))
            }
            div class="w-full md:min-w-0 md:flex-1" {
                (label(
                    "Paid Amount",
                    field_text(FieldText {
                        value: &totals.paid,
                        classes: "",
                    }),
                ))
            }
            div class="w-full md:min-w-0 md:flex-1" {
                (label(
                    "Balance Remaining",
                    field_text(FieldText {
                        value: &totals.balance,
                        classes: "",
                    }),
                ))
            }
        }
    }
}

async fn journal_totals(
    ctx: &CustomerJournalRender,
    fiscal_year: Option<lariv_plugin_finance_common::fiscal_year::FiscalYear>,
) -> JournalTotals {
    let range = fiscal_year.map(|year| year.datetime_range());
    let invoiced = sum_posted_invoice_totals(&ctx.db, ctx.customer_id, range).await;
    let paid = sum_payment_totals(&ctx.db, ctx.customer_id, range).await;
    let fmt = load_default_currency_format(&ctx.db).await;
    format_totals(invoiced, paid, &fmt)
}

fn format_totals(invoiced: Decimal, paid: Decimal, fmt: &CurrencyFormat) -> JournalTotals {
    JournalTotals {
        invoiced: fmt.display(invoiced),
        paid: fmt.display(paid),
        balance: fmt.display(dec_sub(invoiced, paid)),
    }
}

async fn sum_posted_invoice_totals(
    db: &DatabaseConnection,
    customer_id: i64,
    range: Option<(DateTime<Utc>, DateTime<Utc>)>,
) -> Decimal {
    let (sql, values) = match range {
        Some((start, end)) => (
            "SELECT COALESCE(SUM(l.amount), 0) AS total \
             FROM posted_invoices i \
             JOIN posted_payment_term_lines l \
               ON l.posted_payment_term_id = i.posted_payment_term_id \
             WHERE i.customer_id = $1 \
               AND i.datetime >= $2 AND i.datetime < $3 \
               AND NOT EXISTS ( \
                 SELECT 1 FROM cancelled_invoices c WHERE c.posted_invoice_id = i.id \
               )",
            vec![customer_id.into(), start.into(), end.into()],
        ),
        None => (
            "SELECT COALESCE(SUM(l.amount), 0) AS total \
             FROM posted_invoices i \
             JOIN posted_payment_term_lines l \
               ON l.posted_payment_term_id = i.posted_payment_term_id \
             WHERE i.customer_id = $1 \
               AND NOT EXISTS ( \
                 SELECT 1 FROM cancelled_invoices c WHERE c.posted_invoice_id = i.id \
               )",
            vec![customer_id.into()],
        ),
    };
    scalar_decimal(db, sql, values, "posted invoice totals").await
}

async fn sum_payment_totals(
    db: &DatabaseConnection,
    customer_id: i64,
    range: Option<(DateTime<Utc>, DateTime<Utc>)>,
) -> Decimal {
    let (sql, values) = match range {
        Some((start, end)) => (
            "SELECT COALESCE(SUM(amount), 0) AS total FROM payments \
             WHERE customer_id = $1 AND datetime >= $2 AND datetime < $3",
            vec![customer_id.into(), start.into(), end.into()],
        ),
        None => (
            "SELECT COALESCE(SUM(amount), 0) AS total FROM payments WHERE customer_id = $1",
            vec![customer_id.into()],
        ),
    };
    scalar_decimal(db, sql, values, "payment totals").await
}

async fn scalar_decimal(
    db: &DatabaseConnection,
    sql: &str,
    values: Vec<sea_orm::Value>,
    what: &str,
) -> Decimal {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            values,
        ))
        .await;
    match row {
        Ok(Some(row)) => row.try_get("", "total").unwrap_or(Decimal::ZERO),
        Ok(None) => Decimal::ZERO,
        Err(err) => {
            tracing::error!(error = %err, "{what}");
            Decimal::ZERO
        }
    }
}

async fn table(ctx: &CustomerJournalRender, tab: &str, heading: &str, query: &str) -> Markup {
    let (table_key, filter_url) = match tab {
        "posted" => (
            CUSTOMER_INVOICES_TABLE,
            CustomerJournalInvoicesRouteTag::new(ctx.customer_id).path(),
        ),
        "cancelled" => (
            CUSTOMER_CANCELLED_INVOICES_TABLE,
            CustomerJournalCancelledRouteTag::new(ctx.customer_id).path(),
        ),
        "drafts" => (
            CUSTOMER_DRAFT_INVOICES_TABLE,
            CustomerJournalDraftInvoicesRouteTag::new(ctx.customer_id).path(),
        ),
        _ => (
            CUSTOMER_QUOTATIONS_TABLE,
            CustomerJournalQuotationsRouteTag::new(ctx.customer_id).path(),
        ),
    };
    let path_and_query = if query.is_empty() {
        filter_url.clone()
    } else {
        format!("{filter_url}?{query}")
    };
    customer_hub_table(
        &ctx.db,
        CustomerHubTable {
            customer_id: ctx.customer_id,
            tab,
            table_key,
            filter_url: &filter_url,
            heading,
            query,
            path_and_query: &path_and_query,
            cookie_header: ctx.cookie_header.as_deref(),
            timezone: &ctx.timezone,
            embed_selection: query.is_empty(),
        },
        &HubQuery::default(),
    )
    .await
}

fn cookie_header(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn section_key(tab: &str) -> &'static str {
    match tab {
        "posted" => "invoices",
        "cancelled" => "cancelled",
        "drafts" => "draft-invoices",
        _ => "quotations",
    }
}

async fn respond_table(
    state: &InvoicesState,
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    htmx: &Htmx,
    headers: &HeaderMap,
    uri: &Uri,
    id: i64,
    tab: &'static str,
    heading: &'static str,
    q: &HubQuery,
) -> Response {
    let query = uri.query().unwrap_or("");
    let path = path_and_query(uri);
    let (table_key, filter_url) = match tab {
        "posted" => (
            CUSTOMER_INVOICES_TABLE,
            CustomerJournalInvoicesRouteTag::new(id).path(),
        ),
        "cancelled" => (
            CUSTOMER_CANCELLED_INVOICES_TABLE,
            CustomerJournalCancelledRouteTag::new(id).path(),
        ),
        "drafts" => (
            CUSTOMER_DRAFT_INVOICES_TABLE,
            CustomerJournalDraftInvoicesRouteTag::new(id).path(),
        ),
        _ => (
            CUSTOMER_QUOTATIONS_TABLE,
            CustomerJournalQuotationsRouteTag::new(id).path(),
        ),
    };
    let partial = htmx.wants_partial() && !htmx.wants_app_layout() && !htmx.wants_main_content();
    let table = customer_hub_table(
        &state.db,
        CustomerHubTable {
            customer_id: id,
            tab,
            table_key,
            filter_url: &filter_url,
            heading,
            query,
            path_and_query: &path,
            cookie_header: cookie_header(headers).as_deref(),
            timezone: &ctx.timezone,
            embed_selection: !partial,
        },
        q,
    )
    .await;
    if partial {
        return table.into_response();
    }
    let Some(name) = customer_page_name(&state.db, id).await else {
        return htmx.redirect(&CustomerListRouteTag.url()).into_response();
    };
    let env = LarivEnvironment::from_cookie_header(cookie_header(headers).as_deref());
    let fiscal_years = list_fiscal_year_options()
        .into_iter()
        .map(|(start_year, label)| crate::components::FiscalYearOption { start_year, label })
        .collect::<Vec<_>>();
    let selected = selected_fiscal_year_start_for_ui(&env);
    let page = CustomerSectionPage {
        id,
        name,
        active: section_key(tab),
        action: heading,
        body: html! {
            (fiscal_year_environment_selector(&fiscal_years, selected))
            (table)
        },
    };
    respond_customer_page(&page, chrome, ctx, htmx)
}

pub async fn quotations(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    headers: HeaderMap,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<HubQuery>,
) -> Response {
    respond_table(
        &state,
        &chrome,
        &ctx,
        &htmx,
        &headers,
        &uri,
        id,
        "quotations",
        "Quotations",
        &q,
    )
    .await
}

pub async fn draft_invoices(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    headers: HeaderMap,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<HubQuery>,
) -> Response {
    respond_table(
        &state,
        &chrome,
        &ctx,
        &htmx,
        &headers,
        &uri,
        id,
        "drafts",
        "Draft invoices",
        &q,
    )
    .await
}

pub async fn invoices(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    headers: HeaderMap,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<HubQuery>,
) -> Response {
    respond_table(
        &state, &chrome, &ctx, &htmx, &headers, &uri, id, "posted", "Invoices", &q,
    )
    .await
}

pub async fn cancelled(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    headers: HeaderMap,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<HubQuery>,
) -> Response {
    respond_table(
        &state,
        &chrome,
        &ctx,
        &htmx,
        &headers,
        &uri,
        id,
        "cancelled",
        "Cancelled invoices",
        &q,
    )
    .await
}
