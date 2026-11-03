//! Customers list, detail, and journal under Accounting.

use axum::extract::{Path, Query};
use axum::http::{HeaderMap, Uri};
use axum::response::{IntoResponse, Response};
use lariv_plugin_customer::entities::customer::Entity as CustomerEntity;
use lariv_plugin_customer::keys::CUSTOMER_TABLE;
use lariv_plugin_customer::logic::{self, ListedCustomer};
use lariv_plugin_customer::state::CustomerState;
use lariv_plugin_customer::templates::{CustomerListPage, CustomerRow};
use lariv_plugin_users::middleware::RequireAuth;
use sea_orm::EntityTrait;
use serde::Deserialize;

use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::UrlencodedFields;
use lariv_core::http::Cap;
use lariv_core::list_filters::{
    CmpOp, FilterClause, FilterField, JoinOp, parse_list_filters, text_column,
};
use lariv_core::template::RenderAppPane;
use lariv_core::web::{Htmx, QueryPage, QueryPageSize, html_built_page_with_slots};

use crate::journal::{CustomerJournalRegistry, CustomerJournalRender};
use crate::routes::{CustomerDetailRouteTag, CustomerListRouteTag};
use crate::templates::{CustomerDetailPage, CustomerJournalPage, CustomersListPage};

#[derive(Debug, Deserialize, Default)]
pub struct ListQuery {
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    page: QueryPage,
    #[serde(default)]
    page_size: QueryPageSize,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn cookie_header(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn customer_fields() -> Vec<FilterField> {
    vec![text_column(
        "name",
        "Name",
        lariv_plugin_customer::entities::customer::Column::Id,
    )]
}

fn clauses_from_query(query: &str) -> Vec<FilterClause> {
    let mut clauses = parse_list_filters(query);
    if !clauses.is_empty() {
        return clauses;
    }
    let Some(name) = UrlencodedFields::parse(query.as_bytes())
        .ok()
        .and_then(|fields| fields.get_first("Name").map(str::to_string))
        .filter(|name| !name.trim().is_empty())
    else {
        return clauses;
    };
    clauses.push(FilterClause {
        index: 0,
        join: JoinOp::And,
        field: "name".into(),
        op: CmpOp::Eq,
        values: vec![name.trim().to_string()],
    });
    clauses
}

fn name_needle(clauses: &[FilterClause]) -> String {
    clauses
        .iter()
        .find(|clause| clause.field == "name")
        .and_then(|clause| clause.values.first())
        .map(|value| value.trim().to_string())
        .unwrap_or_default()
}

fn to_row(row: ListedCustomer) -> CustomerRow {
    CustomerRow {
        id: row.id,
        name: row.name,
        kind: row.kind,
        href: CustomerDetailRouteTag::new(row.id).url(),
    }
}

struct CustomerView {
    name: String,
    kind: &'static str,
    party_href: String,
    party_label: &'static str,
}

async fn load_customer(db: &sea_orm::DatabaseConnection, id: i64) -> Option<CustomerView> {
    let row = CustomerEntity::find_by_id(id).one(db).await.ok()??;
    let mut name = logic::display_name(db, id).await;
    if name.is_empty() {
        name = format!("#{id}");
    }
    let (kind, party_label) = if row.is_individual {
        ("Contact", "Contact")
    } else {
        ("Company", "Company")
    };
    Some(CustomerView {
        name,
        kind,
        party_href: logic::detail_href(&row),
        party_label,
    })
}

/// `GET /finance/customers` — companies and contacts billed as customers.
pub async fn list(
    Cap(state): Cap<CustomerState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ListQuery>,
) -> maud::Markup {
    let clauses = clauses_from_query(uri.query().unwrap_or(""));
    let page_size = q.page_size.get();
    let page_number = q.page.get();
    let offset = (page_number.saturating_sub(1) as u64) * page_size as u64;
    let sort = q.sort.clone().unwrap_or_else(|| "Name ASC".into());
    let (rows, total) = match logic::list_customers(
        &state.db,
        &name_needle(&clauses),
        &sort,
        page_size as u64,
        offset,
    )
    .await
    {
        Ok(value) => value,
        Err(err) => {
            tracing::error!(error = %err, "finance_customers: customers list");
            (Vec::new(), 0)
        }
    };
    let page = CustomersListPage {
        inner: CustomerListPage {
            customers: ObjectList::from_page(
                rows.into_iter().map(to_row).collect(),
                page_number,
                page_size,
                total,
            ),
            fields: customer_fields(),
            filters: FilterClauseView::from_clauses(&clauses, &[]),
            filter_errors: Vec::new(),
            sort,
            path_and_query: path_and_query(&uri),
            page_size,
        },
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets(CUSTOMER_TABLE) {
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

pub async fn detail(
    Cap(state): Cap<CustomerState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(customer) = load_customer(&state.db, id).await else {
        return htmx.redirect(&CustomerListRouteTag.url()).into_response();
    };
    let page = CustomerDetailPage {
        id,
        name: customer.name,
        kind: customer.kind,
        party_href: customer.party_href,
        party_label: customer.party_label,
    };
    respond_page(&page, &chrome, &ctx, &htmx)
}

pub async fn journal(
    Cap(state): Cap<CustomerState>,
    Cap(journal): Cap<CustomerJournalRegistry>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let Some(customer) = load_customer(&state.db, id).await else {
        return htmx.redirect(&CustomerListRouteTag.url()).into_response();
    };
    let render = CustomerJournalRender {
        customer_id: id,
        db: state.db.clone(),
        timezone: ctx.timezone.clone(),
        cookie_header: cookie_header(&headers),
    };
    let sections = journal.render(&render).await.into_string();
    let page = CustomerJournalPage {
        id,
        name: customer.name,
        sections_html: sections,
        empty: journal.is_empty(),
    };
    respond_page(&page, &chrome, &ctx, &htmx)
}

/// Display name for a customer detail page, when the customer exists.
pub async fn customer_page_name(db: &sea_orm::DatabaseConnection, id: i64) -> Option<String> {
    load_customer(db, id).await.map(|customer| customer.name)
}

pub fn respond_customer_page<P>(
    page: &P,
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    htmx: &Htmx,
) -> Response
where
    P: RenderAppPane + lariv_core::template::RenderTemplate,
{
    respond_page(page, chrome, ctx, htmx)
}

fn respond_page<P>(
    page: &P,
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    htmx: &Htmx,
) -> Response
where
    P: RenderAppPane + lariv_core::template::RenderTemplate,
{
    let slot_ctx = SlotCtx::from_auth(ctx);
    if htmx.wants_main_content() {
        let markup: maud::Markup = page.render_main().into();
        return markup.into_response();
    }
    if htmx.wants_app_layout() {
        let markup: maud::Markup = page.render_pane().into();
        return markup.into_response();
    }
    html_built_page_with_slots(page, chrome, &slot_ctx).into_response()
}
