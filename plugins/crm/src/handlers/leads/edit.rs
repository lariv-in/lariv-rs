use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Redirect, Response};

use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::datetime::format_optional_date;
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::web::{
    Htmx, html_built_page_with_slots, respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_contacts::scope::{contact_display_label, find_contact_scoped};
use lariv_plugin_users::lookup::user_display_label;
use lariv_plugin_users::middleware::RequireAuth;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::{
    entities::failed_lead::{self, Entity as FailedLeadEntity},
    forms::{ConvertLeadBody, FailLeadForm, LeadEditBody},
    handlers::{
        ModalNameQuery,
        lead_tags::{load_tag_items_for_lead, tag_items_from_ids},
    },
    keys::{LEAD_CONVERT_MODAL, LEAD_DELETE_MODAL, LEAD_EDIT_MODAL, LEAD_FAIL_MODAL},
    logic::{
        lead::{delete_lead, update_lead},
        lead_conversion::{convert_lead, unconvert_lead},
        lead_fail::{fail_lead, reactivate_lead, update_failed_reason},
    },
    routes::{
        ConvertedLeadDetailRouteTag, FailedLeadDetailRouteTag, LeadDefaultRouteTag,
        LeadDetailRouteTag,
    },
    scope::{find_active_lead, find_lead_scoped},
    state::CrmState,
    templates::{ConfirmDeletePage, ConvertLeadModalPage, FailLeadModalPage, LeadEditModalPage},
};

use super::{
    convert_modal_page_from_body, lead_input_from_form, lead_return_url, opt_string, source_value,
};

pub async fn edit_get(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(lead) = find_lead_scoped(&state.db, id).await else {
        return Redirect::to(&LeadDefaultRouteTag.url()).into_response();
    };
    let failed = lariv_core::web::opt_or_log(
        FailedLeadEntity::find()
            .filter(failed_lead::Column::LeadId.eq(id))
            .one(&state.db)
            .await,
        "db find one",
    );
    let page = LeadEditModalPage {
        id: lead.id,
        form_name: q.form_name(),
        contact_id: lead.contact_id,
        contact_display: contact_display_label(&state.db, lead.contact_id).await,
        source: source_value(lead.source),
        notes: lead.notes.unwrap_or_default(),
        tags: load_tag_items_for_lead(&state.db, lead.id).await,
        assigned_to_id: lead.assigned_to_id.unwrap_or(0),
        assigned_to_display: user_display_label(&state.db, lead.assigned_to_id.unwrap_or(0)).await,
        order_expected_date: format_optional_date(lead.order_expected_date),
        reason: failed
            .as_ref()
            .and_then(|f| f.reason.clone())
            .unwrap_or_default(),
        show_reason: failed.is_some(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn lead_edit_modal_error(
    db: &sea_orm::DatabaseConnection,
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    id: i64,
    q: &ModalNameQuery,
    form: &LeadEditBody,
    error: &str,
) -> Response {
    let show_reason = lariv_core::web::opt_or_log(
        FailedLeadEntity::find()
            .filter(failed_lead::Column::LeadId.eq(id))
            .one(db)
            .await,
        "db find one",
    )
    .is_some();
    let page = LeadEditModalPage {
        id,
        form_name: q.form_name(),
        contact_id: form.lead.contact_id,
        contact_display: contact_display_label(db, form.lead.contact_id).await,
        source: form.lead.source.clone(),
        notes: form.lead.notes.clone(),
        tags: tag_items_from_ids(db, &form.lead.tags).await,
        assigned_to_id: form.lead.assigned_to_id,
        assigned_to_display: user_display_label(db, form.lead.assigned_to_id).await,
        order_expected_date: form.lead.order_expected_date.clone(),
        reason: form.reason.clone(),
        show_reason,
        error: error.to_string(),
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<LeadEditBody>,
) -> Response {
    if find_lead_scoped(&state.db, id).await.is_none() {
        return Redirect::to(&LeadDefaultRouteTag.url()).into_response();
    }
    if form.lead.contact_id <= 0
        || find_contact_scoped(&state.db, form.lead.contact_id)
            .await
            .is_none()
    {
        return lead_edit_modal_error(
            &state.db,
            &chrome,
            &ctx,
            id,
            &q,
            &form,
            "contact is required",
        )
        .await;
    }
    let input = match lead_input_from_form(&form.lead) {
        Ok(input) => input,
        Err(e) => {
            return lead_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, &e).await;
        }
    };
    if let Err(e) = update_lead(&state.db, id, input).await {
        return lead_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, &e).await;
    }
    if lariv_core::web::opt_or_log(
        FailedLeadEntity::find()
            .filter(failed_lead::Column::LeadId.eq(id))
            .one(&state.db)
            .await,
        "db find one",
    )
    .is_some()
    {
        if let Err(e) =
            update_failed_reason(&state.db, id, &ctx, opt_string(form.reason.clone())).await
        {
            return lead_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, &e).await;
        }
    }
    respond_edit_modal_done(
        &htmx,
        LEAD_EDIT_MODAL,
        &lead_return_url(&state.db, id).await,
    )
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: LEAD_DELETE_MODAL.id().to_string(),
        message: "Are you sure you want to delete this lead?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_crm.LeadDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match delete_lead(&state.db, id).await {
        Ok(()) => htmx.redirect(&LeadDefaultRouteTag.url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete lead");
            let page = ConfirmDeletePage {
                modal_uid: LEAD_DELETE_MODAL.id().to_string(),
                message: "Are you sure you want to delete this lead?".into(),
                form_name: "p_crm.LeadDeleteForm".into(),
                id,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn convert_get(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if find_active_lead(&state.db, id).await.is_none() {
        return Redirect::to(&LeadDefaultRouteTag.url()).into_response();
    }
    let page = ConvertLeadModalPage {
        lead_id: id,
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn convert_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(_form): HtmlFormBody<ConvertLeadBody>,
) -> Response {
    match convert_lead(&state.db, id, &ctx).await {
        Ok(result) => respond_create_modal_done(
            &htmx,
            LEAD_CONVERT_MODAL,
            &q.refresh_table(),
            &ConvertedLeadDetailRouteTag::new(result.converted_id).url(),
        ),
        Err(e) => {
            let page = convert_modal_page_from_body(id, &q, e);
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn fail_get(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(lead) = find_lead_scoped(&state.db, id).await else {
        return Redirect::to(&LeadDefaultRouteTag.url()).into_response();
    };
    let already_failed = lariv_core::web::opt_or_log(
        FailedLeadEntity::find()
            .filter(failed_lead::Column::LeadId.eq(lead.id))
            .one(&state.db)
            .await,
        "db find one",
    )
    .is_some();
    if already_failed {
        return Redirect::to(&LeadDefaultRouteTag.url()).into_response();
    }
    let page = FailLeadModalPage {
        lead_id: id,
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        reason: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn fail_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<FailLeadForm>,
) -> Response {
    match fail_lead(&state.db, id, &ctx, opt_string(form.reason.clone())).await {
        Ok(failed_id) => respond_create_modal_done(
            &htmx,
            LEAD_FAIL_MODAL,
            &q.refresh_table(),
            &FailedLeadDetailRouteTag::new(failed_id).url(),
        ),
        Err(e) => {
            let page = FailLeadModalPage {
                lead_id: id,
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                reason: form.reason,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn reactivate_post(
    Cap(state): Cap<CrmState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    match reactivate_lead(&state.db, id, &ctx).await {
        Ok(lead_id) => Redirect::to(&LeadDetailRouteTag::new(lead_id).url()).into_response(),
        Err(_) => Redirect::to(&LeadDefaultRouteTag.url()).into_response(),
    }
}

pub async fn converted_reactivate_post(
    Cap(state): Cap<CrmState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    match unconvert_lead(&state.db, id, &ctx).await {
        Ok(lead_id) => Redirect::to(&LeadDetailRouteTag::new(lead_id).url()).into_response(),
        Err(_) => Redirect::to(&LeadDefaultRouteTag.url()).into_response(),
    }
}
