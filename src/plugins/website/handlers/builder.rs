//! GrapesJS builder UI + project/theme APIs.

use std::sync::Arc;

use axum::{
    extract::Path,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::AsyncReadExt;

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    grapesjs::GrapesJsCapability,
    http::Cap,
    plugins::{
        filesystem::node,
        users::middleware::RequireAuth,
        website::{
            builder::{grapesjs_body_html, grapesjs_head_html},
            builder_refs::{
                builder_footer_fragment, builder_header_fragment, compose_page_template,
                extract_page_content, load_route_ref_parts, merge_content_css, RouteRefParts,
            },
            entities::db_route::{self, Entity as DbRouteEntity},
            publish::fix_navbar_logos,
            render::replace_vnode_content,
            state::WebsiteState,
            templates::RoutesBuilderPage,
        },
    },
    web::html_built_page_with_slots,
};

/// HTTP handler: `builder_page`.
pub async fn builder_page(
    Cap(state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    let Some(route) = crate::web::opt_or_log(
        DbRouteEntity::find_by_id(id).one(&state.db).await,
        "find website route by id",
    ) else {
        return Redirect::to("/website").into_response();
    };
    let page = RoutesBuilderPage {
        head_html: grapesjs_head_html(),
        body_html: grapesjs_body_html(route.id, &route.path, &route.theme, &grapes),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
}

async fn read_page_text(
    state: &WebsiteState,
    page_id: i64,
) -> Result<String, axum::http::StatusCode> {
    let Some(page) =
        crate::web::opt_or_log(node::get_by_id(&state.db, page_id).await, "get node by id")
    else {
        return Err(axum::http::StatusCode::NOT_FOUND);
    };
    let path = page.file_path.as_deref().unwrap_or("");
    let mut download = state
        .store
        .open(path, &page.name)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut buf = String::new();
    download
        .reader
        .read_to_string(&mut buf)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(buf)
}

fn builder_ref_payload(
    ref_parts: &RouteRefParts,
    page_src: &str,
) -> (String, String, String, String) {
    let has_refs = ref_parts.header_src.is_some() || ref_parts.footer_src.is_some();
    if !has_refs {
        return (
            page_src.to_string(),
            String::new(),
            String::new(),
            String::new(),
        );
    }

    let extracted = extract_page_content(page_src);
    let header_frag = ref_parts.header_src.as_deref().map(builder_header_fragment);
    let header_html = header_frag
        .as_ref()
        .map(|h| h.body_html.clone())
        .unwrap_or_default();
    let header_head_html = header_frag.map(|h| h.head_html).unwrap_or_default();
    let footer_html = ref_parts
        .footer_src
        .as_deref()
        .map(builder_footer_fragment)
        .unwrap_or_default();

    (
        extracted.content,
        header_html,
        footer_html,
        header_head_html,
    )
}

/// HTTP handler: `project_load`.
pub async fn project_load(
    Cap(state): Cap<WebsiteState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    let Some(route) = crate::web::opt_or_log(
        DbRouteEntity::find_by_id(id).one(&state.db).await,
        "find website route by id",
    ) else {
        return (axum::http::StatusCode::NOT_FOUND, "route not found").into_response();
    };

    let page_src = match read_page_text(&state, route.page_id).await {
        Ok(src) => src,
        Err(status) => return (status, "page not found").into_response(),
    };

    let ref_parts = match load_route_ref_parts(&state.db, state.store.as_ref(), route.id).await {
        Ok(parts) => parts,
        Err(e) => {
            tracing::error!(error = %e, "builder load: route refs");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load route references",
            )
                .into_response();
        }
    };

    let (content_html, header_html, footer_html, header_head_html) =
        builder_ref_payload(&ref_parts, &page_src);

    let data = route
        .grapes_project
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .and_then(|s| match serde_json::from_str::<Value>(s) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::error!(error = %e, "builder load: invalid grapes project");
                None
            }
        });

    Json(json!({
        "data": data,
        "content_html": content_html,
        "header_html": header_html,
        "footer_html": footer_html,
        "header_head_html": header_head_html,
        "includes": {
            "header": ref_parts.header_path,
            "footer": ref_parts.footer_path,
        },
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
pub struct ProjectStorePayload {
    pub data: Option<Value>,
    #[serde(default)]
    pub html: String,
    #[serde(default)]
    pub css: String,
}

/// HTTP handler: `project_store`.
pub async fn project_store(
    Cap(state): Cap<WebsiteState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
    Json(payload): Json<ProjectStorePayload>,
) -> Response {
    let Some(route) = crate::web::opt_or_log(
        DbRouteEntity::find_by_id(id).one(&state.db).await,
        "find website route by id",
    ) else {
        return (axum::http::StatusCode::NOT_FOUND, "route not found").into_response();
    };
    let Some(data) = payload.data else {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "project data is required",
        )
            .into_response();
    };
    let Ok(project_bytes) = serde_json::to_string(&data) else {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "failed to encode project",
        )
            .into_response();
    };

    let page_src = match read_page_text(&state, route.page_id).await {
        Ok(src) => src,
        Err(status) => return (status, "page not found").into_response(),
    };
    let extracted = extract_page_content(&page_src);

    let ref_parts = match load_route_ref_parts(&state.db, state.store.as_ref(), route.id).await {
        Ok(parts) => parts,
        Err(e) => {
            tracing::error!(error = %e, "builder store: route refs");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load route references",
            )
                .into_response();
        }
    };

    let content = merge_content_css(&fix_navbar_logos(&payload.html), &payload.css);
    let has_refs = ref_parts.header_path.is_some() || ref_parts.footer_path.is_some();
    let template_bytes = if has_refs {
        let header_inc = ref_parts
            .header_path
            .as_deref()
            .or(extracted.leading_include.as_deref());
        let footer_inc = ref_parts
            .footer_path
            .as_deref()
            .or(extracted.trailing_include.as_deref());
        compose_page_template(header_inc, &content, footer_inc)
    } else {
        content
    };

    let Some(page) = crate::web::opt_or_log(
        node::get_by_id(&state.db, route.page_id).await,
        "get node by id",
    ) else {
        return (axum::http::StatusCode::NOT_FOUND, "page not found").into_response();
    };
    if let Err(e) = replace_vnode_content(
        &state.db,
        state.store.as_ref(),
        page,
        template_bytes.as_bytes(),
    )
    .await
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

/// HTTP handler: `theme_store`.
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
    let Some(route) = crate::web::opt_or_log(
        DbRouteEntity::find_by_id(id).one(&state.db).await,
        "find website route by id",
    ) else {
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
