use axum::{
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};

use crate::{
    components::{SharedChromeFolder, SlotCtx, SwapKey},
    html_form::HtmlFormBody,
    http::Cap,
    plugins::users::{middleware::RequireAuth, state::AuthContext},
    web::{
        Htmx, html_built_page_or_app_layout, html_built_page_with_slots, respond_edit_modal_done,
    },
};

use crate::plugins::crm::{
    entities::lead_update::{self, Entity as LeadUpdateEntity},
    forms::{LeadUpdateForm, LeadUpdateQuickForm},
    handlers::ModalNameQuery,
    keys::{LEAD_UPDATE_SAVED_EVENT, LeadUpdateDeleteModalKey, LeadUpdateEditModalKey},
    routes::LeadDetailRouteTag,
    scope::{
        find_lead_scoped, find_lead_update_scoped, lead_display_name, scope_superuser,
        user_display_label, user_exists,
    },
    state::CrmState,
    templates::{
        ConfirmDeletePage, LeadUpdateDetailPage, LeadUpdateEditModalPage, LeadUpdateItem,
        LeadUpdatesPanel,
    },
};

use axum::extract::Query;

fn lead_url(lead_id: i64) -> String {
    LeadDetailRouteTag::new(lead_id).url()
}

pub(crate) async fn load_updates_panel(
    db: &sea_orm::DatabaseConnection,
    auth: &AuthContext,
    lead_id: i64,
    can_edit: bool,
) -> LeadUpdatesPanel {
    let mut query = LeadUpdateEntity::find().filter(lead_update::Column::LeadId.eq(lead_id));
    query = scope_superuser(query, auth);
    let models = query
        .order_by_desc(lead_update::Column::Datetime)
        .order_by_desc(lead_update::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();
    let items = models
        .into_iter()
        .map(|u| LeadUpdateItem {
            id: u.id,
            datetime: auth.format_datetime(u.datetime).into_string(),
            description: u.description,
        })
        .collect();
    LeadUpdatesPanel {
        lead_id,
        items,
        can_edit,
        default_datetime: auth.datetime_local_input(Utc::now()).into_string(),
    }
}

pub async fn detail(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(update) = find_lead_update_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    let Some(lead) = find_lead_scoped(&state.db, update.lead_id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    let page = LeadUpdateDetailPage {
        id: update.id,
        lead_id: update.lead_id,
        display_name: lead_display_name(&state.db, &lead).await,
        created_by: user_display_label(&state.db, update.created_by_id).await,
        datetime: ctx.format_datetime(update.datetime).into_string(),
        description: update.description,
        can_edit: ctx.user.is_superuser,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

fn created_by_id(form: &LeadUpdateForm, fallback: i64) -> i64 {
    if form.created_by_id <= 0 {
        fallback
    } else {
        form.created_by_id
    }
}

pub async fn add_post(
    Cap(state): Cap<CrmState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(lead_id): Path<i64>,
    HtmlFormBody(form): HtmlFormBody<LeadUpdateQuickForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&lead_url(lead_id)).into_response();
    }
    if find_lead_scoped(&state.db, lead_id, &ctx).await.is_none() {
        return Redirect::to("/crm/leads").into_response();
    }
    let description = form.description.trim();
    if description.is_empty() {
        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
    }
    let Some(datetime) = ctx.parse_datetime_local_input(&form.datetime) else {
        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
    };
    let now = Utc::now();
    let model = lead_update::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        lead_id: Set(lead_id),
        created_by_id: Set(ctx.user.id),
        datetime: Set(datetime),
        description: Set(description.to_string()),
    };
    if model.insert(&state.db).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    if !htmx.request {
        return Redirect::to(&lead_url(lead_id)).into_response();
    }
    let panel = load_updates_panel(&state.db, &ctx, lead_id, true).await;
    let body = panel.render_list().into_string();
    let trigger = format!(r#"{{"{LEAD_UPDATE_SAVED_EVENT}":true}}"#);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header("HX-Trigger", trigger)
        .body(body.into())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub async fn edit_get(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    let Some(update) = find_lead_update_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    let page = LeadUpdateEditModalPage {
        id: update.id,
        form_name: q.form_name(),
        created_by_id: update.created_by_id,
        created_by_display: user_display_label(&state.db, update.created_by_id).await,
        datetime: ctx.datetime_local_input(update.datetime).into_string(),
        description: update.description,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn edit_modal_error(
    db: &sea_orm::DatabaseConnection,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    id: i64,
    q: &ModalNameQuery,
    form: &LeadUpdateForm,
    created_by_id: i64,
    error: &str,
) -> Response {
    let page = LeadUpdateEditModalPage {
        id,
        form_name: q.form_name(),
        created_by_id,
        created_by_display: user_display_label(db, created_by_id).await,
        datetime: form.datetime.clone(),
        description: form.description.clone(),
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
    HtmlFormBody(form): HtmlFormBody<LeadUpdateForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    let Some(existing) = find_lead_update_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    let created_by_id = created_by_id(&form, existing.created_by_id);
    if !user_exists(&state.db, created_by_id).await {
        return edit_modal_error(
            &state.db,
            &chrome,
            &ctx,
            id,
            &q,
            &form,
            created_by_id,
            "created by is required",
        )
        .await;
    }
    if form.description.trim().is_empty() {
        return edit_modal_error(
            &state.db,
            &chrome,
            &ctx,
            id,
            &q,
            &form,
            created_by_id,
            "description is required",
        )
        .await;
    }
    let Some(datetime) = ctx.parse_datetime_local_input(&form.datetime) else {
        return edit_modal_error(
            &state.db,
            &chrome,
            &ctx,
            id,
            &q,
            &form,
            created_by_id,
            "invalid date & time",
        )
        .await;
    };
    let lead_id = existing.lead_id;
    let now = Utc::now();
    let mut am: lead_update::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.created_by_id = Set(created_by_id);
    am.datetime = Set(datetime);
    am.description = Set(form.description.trim().to_string());
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<LeadUpdateEditModalKey>(&htmx, &lead_url(lead_id)),
        Err(e) => {
            edit_modal_error(
                &state.db,
                &chrome,
                &ctx,
                id,
                &q,
                &form,
                created_by_id,
                &e.to_string(),
            )
            .await
        }
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: LeadUpdateDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this update?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_crm.LeadUpdateDeleteForm".into()),
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
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    let Some(update) = find_lead_update_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    let lead_id = update.lead_id;
    match LeadUpdateEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&lead_url(lead_id)),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete lead update");
            let page = ConfirmDeletePage {
                modal_uid: LeadUpdateDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this update?".into(),
                form_name: "p_crm.LeadUpdateDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
