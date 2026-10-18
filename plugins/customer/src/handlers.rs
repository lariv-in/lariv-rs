//! HTTP handlers for the customers dashboard.

use axum::extract::Query;
use axum::http::Uri;
use axum::response::{IntoResponse, Response};
use lariv_plugin_contacts::entities::{
    company::Entity as CompanyEntity, contact::Entity as ContactEntity,
};
use lariv_plugin_crm::logic::customer;
use lariv_plugin_users::middleware::RequireAuth;
use sea_orm::EntityTrait;

use crate::forms::CustomerForm;
use crate::keys::CustomerCreateModalKey;
use crate::logic::{self, ListedCustomer};
use crate::routes::CustomerDefaultRouteTag;
use crate::state::CustomerState;
use crate::templates::{CustomerCreateModalPage, CustomerListPage, CustomerRow};
use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::{HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_core::list_filters::{
    CmpOp, FilterClause, FilterField, JoinOp, parse_list_filters, text_column,
};
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, ModalFormQuery, QueryPage, QueryPageSize, html_built_page_with_slots,
    respond_create_modal_done,
};

#[derive(Debug, serde::Deserialize, Default)]
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

fn customer_fields() -> Vec<FilterField> {
    vec![text_column(
        "name",
        "Name",
        lariv_plugin_crm::entities::customer::Column::Id,
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

/// `GET /dashboard/customers` — companies and contacts billed as customers.
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
            tracing::error!(error = %err, "customer: list");
            (Vec::new(), 0)
        }
    };
    let page = CustomerListPage {
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
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<crate::keys::CustomerTableKey>() {
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

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let page = CustomerCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        company_id: 0,
        company_display: String::new(),
        individual_id: 0,
        individual_display: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<CustomerState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<CustomerForm>,
) -> Response {
    match customer::create_for_party(&state.db, form.company_id, form.individual_id).await {
        Ok(_id) => respond_create_modal_done::<CustomerCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &CustomerDefaultRouteTag.url(),
        ),
        Err(error) => {
            let (company_display, individual_display) =
                party_displays(&state.db, form.company_id, form.individual_id).await;
            let page = CustomerCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                company_id: form.company_id,
                company_display,
                individual_id: form.individual_id,
                individual_display,
                error,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

async fn party_displays(
    db: &sea_orm::DatabaseConnection,
    company_id: i64,
    individual_id: i64,
) -> (String, String) {
    let company = if company_id > 0 {
        CompanyEntity::find_by_id(company_id)
            .one(db)
            .await
            .ok()
            .flatten()
            .map(|row| row.name)
            .unwrap_or_default()
    } else {
        String::new()
    };
    let individual = if individual_id > 0 {
        ContactEntity::find_by_id(individual_id)
            .one(db)
            .await
            .ok()
            .flatten()
            .map(|row| row.name)
            .unwrap_or_default()
    } else {
        String::new()
    };
    (company, individual)
}
