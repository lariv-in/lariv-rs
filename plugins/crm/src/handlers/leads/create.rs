use axum::extract::Query;
use axum::response::{IntoResponse, Response};

use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_with_slots, respond_create_modal_done};
use lariv_plugin_customer::entities::customer::Entity as CustomerEntity;
use lariv_plugin_customer::logic::display_name;
use lariv_plugin_users::lookup::user_display_label;
use lariv_plugin_users::middleware::RequireAuth;
use sea_orm::EntityTrait;

use crate::{
    forms::LeadForm,
    handlers::{ModalNameQuery, lead_tags::tag_items_from_ids},
    keys::LEAD_CREATE_MODAL,
    logic::lead::create_lead,
    routes::LeadDetailRouteTag,
    state::CrmState,
};

use super::{lead_create_modal_page, lead_input_from_form};

async fn customer_exists(db: &sea_orm::DatabaseConnection, id: i64) -> bool {
    lariv_core::web::opt_or_log(CustomerEntity::find_by_id(id).one(db).await, "find by id")
        .is_some()
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = lead_create_modal_page(
        &q,
        0,
        String::new(),
        String::new(),
        String::new(),
        Vec::new(),
        0,
        String::new(),
        String::new(),
        String::new(),
    );
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<LeadForm>,
) -> Response {
    if form.customer_id <= 0 || !customer_exists(&state.db, form.customer_id).await {
        let page = lead_create_modal_page(
            &q,
            form.customer_id,
            display_name(&state.db, form.customer_id).await,
            form.source,
            form.notes,
            tag_items_from_ids(&state.db, &form.tags).await,
            form.assigned_to_id,
            user_display_label(&state.db, form.assigned_to_id).await,
            form.order_expected_date,
            "customer is required".to_string(),
        );
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let input = match lead_input_from_form(&form) {
        Ok(input) => input,
        Err(e) => {
            let page = lead_create_modal_page(
                &q,
                form.customer_id,
                display_name(&state.db, form.customer_id).await,
                form.source,
                form.notes,
                tag_items_from_ids(&state.db, &form.tags).await,
                form.assigned_to_id,
                user_display_label(&state.db, form.assigned_to_id).await,
                form.order_expected_date,
                e,
            );
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    match create_lead(&state.db, input).await {
        Ok(saved) => respond_create_modal_done(
            &htmx,
            LEAD_CREATE_MODAL,
            &q.refresh_table(),
            &LeadDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => {
            let page = lead_create_modal_page(
                &q,
                form.customer_id,
                display_name(&state.db, form.customer_id).await,
                form.source,
                form.notes,
                tag_items_from_ids(&state.db, &form.tags).await,
                form.assigned_to_id,
                user_display_label(&state.db, form.assigned_to_id).await,
                form.order_expected_date,
                e,
            );
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
