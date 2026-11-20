//! HTTP handlers for the customer picker and create modal.

use axum::extract::Query;
use axum::http::Uri;
use axum::response::{IntoResponse, Response};
use lariv_plugin_companies::entities::company::Entity as CompanyEntity;
use lariv_plugin_contacts::entities::contact::Entity as ContactEntity;
use lariv_plugin_users::middleware::RequireAuth;
use sea_orm::EntityTrait;

use crate::forms::CustomerForm;
use crate::keys::{CUSTOMER_CREATE_MODAL, CUSTOMER_SELECT_MODAL, CUSTOMER_SELECT_TABLE};
use crate::logic;
use crate::state::CustomerState;
use crate::templates::{
    CustomerCreateModalPage, CustomerListPage, CustomerRow, CustomerSelectPage,
};
use lariv_core::components::picker::respond_picker_select;
use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::{HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_core::list_filters::{
    CmpOp, FilterClause, FilterField, JoinOp, parse_list_filters, text_column,
};
use lariv_core::web::{
    Htmx, ModalFormQuery, QueryPage, QueryPageSize, html_built_page_with_slots,
    respond_create_modal_done_fk,
};

/// Fallback redirect when create is not an HTMX modal refresh (list lives under Accounting).
pub const CUSTOMERS_LIST_PATH: &str = "/finance/customers";

#[derive(Debug, serde::Deserialize, Default)]
pub struct SelectQuery {
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    page: QueryPage,
    #[serde(default)]
    page_size: QueryPageSize,
    #[serde(default)]
    target_input: Option<String>,
}

pub async fn select(
    Cap(state): Cap<CustomerState>,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<SelectQuery>,
) -> maud::Markup {
    let query = uri.query().unwrap_or("");
    let clauses = clauses_from_query(query);
    let needle = name_needle(&clauses, query);
    let page_size = q.page_size.get();
    let page = q.page.get();
    let offset = (page.saturating_sub(1) as u64) * page_size as u64;
    let sort = q.sort.clone().unwrap_or_else(|| "Name ASC".into());
    let (rows, total) = logic::list_customers(&state.db, &needle, &sort, page_size as u64, offset)
        .await
        .unwrap_or_default();
    let page_model = CustomerSelectPage {
        list: CustomerListPage {
            customers: ObjectList::from_page(
                rows.into_iter()
                    .map(|row| CustomerRow {
                        id: row.id,
                        name: row.name,
                        kind: row.kind,
                        href: String::new(),
                    })
                    .collect(),
                page,
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
        target_input: q
            .target_input
            .clone()
            .unwrap_or_else(|| "CustomerID".into()),
    };
    respond_picker_select(
        &htmx,
        CUSTOMER_SELECT_TABLE,
        CUSTOMER_SELECT_MODAL,
        &page_model,
    )
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let page = CustomerCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
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
    match logic::create_for_party(&state.db, form.company_id, form.individual_id).await {
        Ok(id) => {
            let target = q.target_input();
            let (fk_value, fk_display) = if target.is_empty() {
                (String::new(), String::new())
            } else {
                (id.to_string(), logic::display_name(&state.db, id).await)
            };
            respond_create_modal_done_fk(
                &htmx,
                CUSTOMER_CREATE_MODAL,
                &q.refresh_table(),
                CUSTOMERS_LIST_PATH,
                fk_value,
                &fk_display,
                &target,
            )
        }
        Err(error) => customer_create_error(&state.db, &chrome, &ctx, &q, &form, error).await,
    }
}

async fn customer_create_error(
    db: &sea_orm::DatabaseConnection,
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    q: &ModalFormQuery,
    form: &CustomerForm,
    error: String,
) -> Response {
    let (company_display, individual_display) =
        party_displays(db, form.company_id, form.individual_id).await;
    let page = CustomerCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        company_id: form.company_id,
        company_display,
        individual_id: form.individual_id,
        individual_display,
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
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

fn customer_fields() -> Vec<FilterField> {
    vec![text_column(
        "name",
        "Name",
        crate::entities::customer::Column::Id,
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

fn name_needle(clauses: &[FilterClause], query: &str) -> String {
    if let Some(clause) = clauses.iter().find(|clause| clause.field == "name") {
        return clause
            .values
            .first()
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
    }
    UrlencodedFields::parse(query.as_bytes())
        .ok()
        .and_then(|fields| fields.get_first("Name").map(|name| name.trim().to_string()))
        .unwrap_or_default()
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}
