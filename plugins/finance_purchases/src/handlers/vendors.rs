use axum::{
    extract::Query,
    http::Uri,
    response::{IntoResponse, Response},
};
use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::{HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_core::list_filters::{
    CmpOp, FilterClause, FilterField, JoinOp, parse_list_filters, text_column,
};
use lariv_core::picker::respond_picker_select;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_with_slots, respond_create_modal_done_fk,
};
use lariv_plugin_contacts::entities::{
    company::Entity as CompanyEntity, contact::Entity as ContactEntity,
};
use lariv_plugin_users::middleware::RequireAuth;
use sea_orm::EntityTrait;

use crate::forms::VendorForm;
use crate::keys::VendorCreateModalKey;
use crate::logic::vendor::{self, display_name};
use crate::routes::PurchaseDefaultRouteTag;
use crate::state::PurchasesState;
use crate::vendor_ui::{VendorCreateModalPage, VendorRow, VendorSelectPage};

use super::ModalNameQuery;

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
    Cap(state): Cap<PurchasesState>,
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
    let (rows, total) = vendor::search(&state.db, &needle, page_size as u64, offset)
        .await
        .unwrap_or_default();
    let page_model = VendorSelectPage {
        vendors: ObjectList::from_page(
            rows.into_iter()
                .map(|row| VendorRow {
                    id: row.id,
                    name: row.name,
                })
                .collect(),
            page,
            page_size,
            total,
        ),
        fields: vendor_fields(),
        filters: FilterClauseView::from_clauses(&clauses, &[]),
        filter_errors: Vec::new(),
        sort: q.sort.clone().unwrap_or_else(|| "Name ASC".into()),
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.clone().unwrap_or_else(|| "VendorID".into()),
        page_size,
    };
    respond_picker_select::<crate::keys::VendorSelectTableKey, crate::keys::VendorSelectModalKey, _>(
        &htmx,
        &page_model,
    )
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let page = VendorCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        company_id: 0,
        company_display: String::new(),
        individual_id: 0,
        individual_display: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<VendorForm>,
) -> Response {
    match vendor::create_for_party(&state.db, form.company_id, form.individual_id).await {
        Ok(id) => {
            let name = display_name(&state.db, id).await;
            respond_create_modal_done_fk::<VendorCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &PurchaseDefaultRouteTag.url(),
                id,
                &name,
                &q.target_input(),
            )
        }
        Err(error) => {
            let (company_display, individual_display) =
                party_displays(&state.db, form.company_id, form.individual_id).await;
            let page = VendorCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
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

fn vendor_fields() -> Vec<FilterField> {
    vec![text_column(
        "name",
        "Name",
        crate::entities::vendor::Column::Id,
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
