//! Customers list hosted under Accounting (table from the customer plugin).

use axum::extract::Query;
use axum::http::Uri;
use lariv_plugin_customer::keys::CustomerTableKey;
use lariv_plugin_customer::logic::{self, ListedCustomer};
use lariv_plugin_customer::state::CustomerState;
use lariv_plugin_customer::templates::{CustomerListPage, CustomerRow};
use lariv_plugin_users::middleware::RequireAuth;
use serde::Deserialize;

use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::UrlencodedFields;
use lariv_core::http::Cap;
use lariv_core::list_filters::{
    CmpOp, FilterClause, FilterField, JoinOp, parse_list_filters, text_column,
};
use lariv_core::template::RenderAppPane;
use lariv_core::web::{Htmx, QueryPage, QueryPageSize, html_built_page_with_slots};

use crate::templates::CustomersListPage;

use super::util::path_and_query;

#[derive(Debug, Deserialize, Default)]
pub struct ListQuery {
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    page: QueryPage,
    #[serde(default)]
    page_size: QueryPageSize,
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
        href: row.href,
    }
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
            tracing::error!(error = %err, "finance_accounts: customers list");
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
    if htmx.targets::<CustomerTableKey>() {
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
