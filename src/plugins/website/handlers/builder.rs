//! GrapesJS builder UI + project/theme APIs.

use std::sync::Arc;
use axum::{
    Json,
    extract::Path,
    response::{IntoResponse, Redirect, Response},
};
use frunk::into_generic;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::AsyncReadExt;

use crate::{
    components::{FoldSlots, SlotCapability, SlotCtx},
    grapesjs::GrapesJsCapability,
    http::Cap,
    plugins::{
        filesystem::node,
        users::middleware::RequireAuth,
        website::{
            builder::{grapesjs_body_html, grapesjs_head_html},
            entities::db_route::{self, Entity as DbRouteEntity},
            publish::finalize_published_html,
            render::replace_vnode_content,
            state::WebsiteState,
            templates::{RoutesBuilderPage, RoutesBuilderPageTag},
        },
    },
    template::{TemplateCapability, TemplateOf},
    traits::get::GetByTag,
    web::html_page_with_slots,
};

pub async fn builder_page<Templates, Slots, Idx, P>(
    Cap(state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response
where
    Templates: GetByTag<RoutesBuilderPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: frunk::Generic<Repr = <RoutesBuilderPage as frunk::Generic>::Repr>
        + crate::template::RenderTemplate,
    Slots: FoldSlots + Clone + Send + Sync + 'static,
{
    let Some(route) = DbRouteEntity::find_by_id(id)
        .filter(db_route::Column::DeletedAt.is_null())
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Redirect::to("/website").into_response();
    };
    let page = RoutesBuilderPage {
        head_html: grapesjs_head_html(),
        body_html: grapesjs_body_html(route.id, &route.path, &route.theme, &grapes),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    html_page_with_slots::<P, _>(into_generic(page), &slots, &slot_ctx).into_response()
}

pub async fn project_load(
    Cap(state): Cap<WebsiteState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    let Some(route) = DbRouteEntity::find_by_id(id)
        .filter(db_route::Column::DeletedAt.is_null())
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return (axum::http::StatusCode::NOT_FOUND, "route not found").into_response();
    };

    if let Some(project) = route
        .grapes_project
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        match serde_json::from_str::<Value>(project) {
            Ok(data) => return Json(json!({ "data": data })).into_response(),
            Err(e) => {
                tracing::error!(error = %e, "builder load: invalid grapes project");
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "invalid stored project",
                )
                    .into_response();
            }
        }
    }

    let Some(page) = node::get_by_id(&state.db, route.page_id).await.ok().flatten() else {
        return (axum::http::StatusCode::NOT_FOUND, "page not found").into_response();
    };
    let path = page.file_path.as_deref().unwrap_or("");
    match state.store.open(path, &page.name).await {
        Ok(mut download) => {
            let mut buf = String::new();
            if download.reader.read_to_string(&mut buf).await.is_err() {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to read page",
                )
                    .into_response();
            }
            Json(json!({ "data": null, "html": buf })).into_response()
        }
        Err(_) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "failed to read page",
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ProjectStorePayload {
    pub data: Option<Value>,
    #[serde(default)]
    pub html: String,
    #[serde(default)]
    pub css: String,
}

pub async fn project_store(
    Cap(state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
    Json(payload): Json<ProjectStorePayload>,
) -> Response {
    let Some(route) = DbRouteEntity::find_by_id(id)
        .filter(db_route::Column::DeletedAt.is_null())
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return (axum::http::StatusCode::NOT_FOUND, "route not found").into_response();
    };
    let Some(data) = payload.data else {
        return (axum::http::StatusCode::BAD_REQUEST, "project data is required").into_response();
    };
    let Ok(project_bytes) = serde_json::to_string(&data) else {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "failed to encode project",
        )
            .into_response();
    };

    let theme_id = route.theme.clone();
    let theme = grapes.theme(&theme_id);
    let published = finalize_published_html(&payload.html, &payload.css, &theme_id, theme);

    let Some(page) = node::get_by_id(&state.db, route.page_id).await.ok().flatten() else {
        return (axum::http::StatusCode::NOT_FOUND, "page not found").into_response();
    };
    if let Err(e) =
        replace_vnode_content(&state.db, state.store.as_ref(), page, published.as_bytes()).await
    {
        tracing::error!(error = %e, "builder store: replace page");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "failed to save page content",
        )
            .into_response();
    }

    let mut am: db_route::ActiveModel = route.into();
    am.grapes_project = Set(Some(project_bytes));
    if am.update(&state.db).await.is_err() {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "failed to save project",
        )
            .into_response();
    }
    Json(json!({ "ok": true })).into_response()
}

#[derive(Debug, Deserialize)]
pub struct ThemePayload {
    #[serde(default)]
    pub theme: String,
}

pub async fn theme_store(
    Cap(state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
    Json(payload): Json<ThemePayload>,
) -> Response {
    let theme_id = payload.theme.trim().to_string();
    if !theme_id.is_empty() && grapes.theme(&theme_id).is_none() {
        return (axum::http::StatusCode::BAD_REQUEST, "unknown theme").into_response();
    }
    let Some(route) = DbRouteEntity::find_by_id(id)
        .filter(db_route::Column::DeletedAt.is_null())
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return (axum::http::StatusCode::NOT_FOUND, "route not found").into_response();
    };
    let mut am: db_route::ActiveModel = route.into();
    am.theme = Set(theme_id);
    if am.update(&state.db).await.is_err() {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "failed to save theme",
        )
            .into_response();
    }
    Json(json!({ "ok": true })).into_response()
}
