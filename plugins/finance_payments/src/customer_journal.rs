//! Customer journal table for payments.

use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::{Path, Query};
use axum::http::Uri;
use axum::response::{IntoResponse, Response};
use maud::{Markup, html};

use lariv_core::components::SharedChromeFolder;
use lariv_core::http::Cap;
use lariv_core::web::Htmx;
use lariv_plugin_finance_customers::handlers::{customer_page_name, respond_customer_page};
use lariv_plugin_finance_customers::journal::{
    CustomerJournalRegistrar, CustomerJournalRegistry, CustomerJournalRender,
    CustomerJournalSection, CustomerSidebarLink, CustomerSidebarRegistry,
};
use lariv_plugin_finance_customers::routes::CustomerListRouteTag;
use lariv_plugin_finance_customers::templates::CustomerSectionPage;
use lariv_plugin_users::middleware::RequireAuth;

use crate::handlers::payments::{CustomerPaymentsTable, ListQuery, customer_payments_table};
use crate::routes::CustomerJournalPaymentsRouteTag;
use crate::state::PaymentsState;

#[derive(Clone, Copy, Default)]
pub struct Hook;

fn payments_url(customer_id: i64) -> String {
    CustomerJournalPaymentsRouteTag::new(customer_id).url()
}

impl CustomerJournalRegistrar for Hook {
    fn register_customer_journal(
        self,
        registry: CustomerJournalRegistry,
    ) -> CustomerJournalRegistry {
        registry.push(Arc::new(PaymentsSection))
    }

    fn register_customer_sidebar(
        self,
        registry: CustomerSidebarRegistry,
    ) -> CustomerSidebarRegistry {
        registry.push(CustomerSidebarLink {
            key: "payments",
            title: "Payments",
            url_fn: payments_url,
            order: 80,
        })
    }
}

struct PaymentsSection;

#[async_trait]
impl CustomerJournalSection for PaymentsSection {
    fn order(&self) -> u16 {
        80
    }

    async fn render(&self, ctx: &CustomerJournalRender) -> Markup {
        let filter_url = CustomerJournalPaymentsRouteTag::new(ctx.customer_id).path();
        let table = customer_payments_table(
            &ctx.db,
            CustomerPaymentsTable {
                customer_id: ctx.customer_id,
                path_and_query: &filter_url,
                filter_url: &filter_url,
                timezone: &ctx.timezone,
                q: &ListQuery::default(),
            },
        )
        .await;
        html! {
            (table)
        }
    }
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

pub async fn payments(
    Cap(state): Cap<PaymentsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<ListQuery>,
) -> Response {
    let partial = htmx.wants_partial() && !htmx.wants_app_layout() && !htmx.wants_main_content();
    let filter_url = CustomerJournalPaymentsRouteTag::new(id).path();
    let table = customer_payments_table(
        &state.db,
        CustomerPaymentsTable {
            customer_id: id,
            path_and_query: &path_and_query(&uri),
            filter_url: &filter_url,
            timezone: &ctx.timezone,
            q: &q,
        },
    )
    .await;
    if partial {
        return table.into_response();
    }
    let Some(name) = customer_page_name(&state.db, id).await else {
        return htmx.redirect(&CustomerListRouteTag.url()).into_response();
    };
    let page = CustomerSectionPage {
        id,
        name,
        active: "payments",
        action: "Payments",
        body: table,
    };
    respond_customer_page(&page, &chrome, &ctx, &htmx)
}
