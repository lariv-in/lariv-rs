//! HTTP handlers for customer create modal.

use axum::extract::Query;
use axum::response::{IntoResponse, Response};
use lariv_plugin_contacts::entities::{
    company::Entity as CompanyEntity, contact::Entity as ContactEntity,
};
use lariv_plugin_users::middleware::RequireAuth;
use sea_orm::EntityTrait;

use crate::forms::CustomerForm;
use crate::keys::CUSTOMER_CREATE_MODAL;
use crate::logic;
use crate::state::CustomerState;
use crate::templates::CustomerCreateModalPage;
use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::web::{
    Htmx, ModalFormQuery, html_built_page_with_slots, respond_create_modal_done,
};

/// Fallback redirect when create is not an HTMX modal refresh (list lives under Accounting).
pub const CUSTOMERS_LIST_PATH: &str = "/finance/customers";

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
    match logic::create_for_party(&state.db, form.company_id, form.individual_id).await {
        Ok(_id) => respond_create_modal_done(
            &htmx,
            CUSTOMER_CREATE_MODAL,
            &q.refresh_table(),
            CUSTOMERS_LIST_PATH,
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
