//! Admin CRUD for `db_routes`.

use std::sync::Arc;

use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use serde::Deserialize;

use crate::{
    components::{ManyToManyItem, DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx},
    grapesjs::GrapesJsCapability,
    html_form::HtmlFormBody,
    http::Cap,
    plugins::{
        filesystem::node,
        users::middleware::RequireAuth,
        website::{
            entities::{
                db_route::{self, ActiveModel, Entity as DbRouteEntity},
                route_reference::{self, Entity as RouteRefEntity},
            },
            forms::{RouteCreateBody, RouteEditBody},
            html_edit::{BLANK_PAGE_STARTER_HTML, is_editable_html_name},
            keys::{RouteCreateModalKey, RouteEditModalKey},
            routes::WebsiteRoutesDetailRouteTag,
            state::WebsiteState,
            templates::{
                ConfirmDeletePage, RouteCreateModalPage, RouteDetailPage, RouteEditModalPage,
                RouteListPage, RouteRow,
            },
        },
    },
    web::{
        Htmx, html_built_page_or_app_layout, html_built_page_with_slots, respond_create_modal_done,
        respond_edit_modal_done,
    },
};

use super::ModalNameQuery;


const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, Deserialize, Default)]
pub struct RouteListQuery {
    #[serde(default, rename = "Path", alias = "path")]
    pub path: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
}

fn theme_choices(grapes: &GrapesJsCapability) -> Vec<(String, String)> {
    grapes
        .themes()
        .iter()
        .map(|(id, t)| (id.clone(), t.label.clone()))
        .collect()
}

async fn sync_refs(
    db: &sea_orm::DatabaseConnection,
    route_id: i64,
    ref_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    RouteRefEntity::delete_many()
        .filter(route_reference::Column::DbRouteId.eq(route_id))
        .exec(db)
        .await?;
    for &vid in ref_ids {
        route_reference::ActiveModel {
            db_route_id: Set(route_id),
            v_node_id: Set(vid),
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

async fn load_ref_items(db: &sea_orm::DatabaseConnection, route_id: i64) -> Vec<ManyToManyItem> {
    let links = RouteRefEntity::find().filter(route_reference::Column::DbRouteId.eq(route_id))
        .all(db)
        .await
        .unwrap_or_default();
    let mut out = Vec::new();
    for link in links {
        if let Ok(Some(n)) = node::get_by_id(db, link.v_node_id).await {
            out.push(ManyToManyItem {
                key: n.id.to_string(),
                value: n.name,
            });
        }
    }
    out
}

/// HTTP handler: `list`.
pub async fn list(
    Cap(state): Cap<WebsiteState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<RouteListQuery>,
) -> maud::Markup
{
    let mut query = DbRouteEntity::find();
    let path_f = q.path.clone().unwrap_or_default();
    if !path_f.is_empty() {
        query = query.filter(db_route::Column::Path.contains(&path_f));
    }
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let query = match sort {
        s if s.eq_ignore_ascii_case("Path DESC") => query.order_by_desc(db_route::Column::Path),
        s if s.eq_ignore_ascii_case("Path ASC") || s.eq_ignore_ascii_case("Path") => {
            query.order_by_asc(db_route::Column::Path)
        }
        s if s.eq_ignore_ascii_case("IsActive DESC") => {
            query.order_by_desc(db_route::Column::IsActive)
        }
        s if s.eq_ignore_ascii_case("IsActive ASC") || s.eq_ignore_ascii_case("IsActive") => {
            query.order_by_asc(db_route::Column::IsActive)
        }
        _ => query.order_by_desc(db_route::Column::Id),
    };
    let page = q.page.unwrap_or(1).max(1);
    let paginator = query.paginate(&state.db, PAGE_SIZE as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::new();
    for r in models {
        let page_name = node::get_by_id(&state.db, r.page_id)
            .await
            .ok()
            .flatten()
            .map(|n| n.name)
            .unwrap_or_default();
        rows.push(RouteRow {
            id: r.id,
            path: r.path,
            page_name,
            is_active: r.is_active,
        });
    }
    let list = ObjectList::from_page(rows, page, PAGE_SIZE, total);
    let pq = uri
        .path_and_query()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string());
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let page = RouteListPage {
        routes: list,
        filter_path: path_f,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: pq,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
}

/// HTTP handler: `create_get`.
pub async fn create_get(
    Cap(_state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup
{
    let page = RouteCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        path: String::new(),
        page_id: None,
        page_name: String::new(),
        is_active: true,
        theme: String::new(),
        theme_choices: theme_choices(&grapes),
        references: Vec::new(),
        error_path: None,
        error_page: None,
        error_name: None,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

/// HTTP handler: `create_post`.
pub async fn create_post(
    Cap(state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<RouteCreateBody>,
) -> Response
{
    let path = form.path.trim().to_string();
    let mut error_path = None;
    let mut error_page = None;
    let mut error_name = None;
    if path.is_empty() {
        error_path = Some("path is required".into());
    }

    let create_new = form.kind == "CreateNew";
    let mut page_id = form.page_id.filter(|id| *id > 0);

    if error_path.is_none() {
        if create_new {
            let name = form.new_page_name.clone().unwrap_or_default();
            let name = name.trim().to_string();
            if name.is_empty() {
                error_name = Some("filename is required".into());
            } else if !is_editable_html_name(&name) {
                error_name = Some("filename must end in .html, .htm, or .tmpl".into());
            } else {
                let segments = state.config.new_page_root_segments();
                match node::ensure_directory_path(
                    &state.db,
                    state.store.as_ref(),
                    None,
                    &segments,
                )
                .await
                {
                    Ok(parent_id) => {
                        let parent = match parent_id {
                            Some(id) => node::get_by_id(&state.db, id).await.ok().flatten(),
                            None => None,
                        };
                        match node::create(
                            &state.db,
                            state.store.as_ref(),
                            name.clone(),
                            false,
                            Some(node::NodeFile::Bytes {
                                filename: name.clone(),
                                data: BLANK_PAGE_STARTER_HTML.as_bytes().to_vec(),
                            }),
                            parent.as_ref(),
                        )
                        .await
                        {
                            Ok(n) => page_id = Some(n.id),
                            Err(e) => error_name = Some(e.to_string()),
                        }
                    }
                    Err(e) => error_name = Some(e.to_string()),
                }
            }
        } else if page_id.is_none() {
            error_page = Some("template page is required".into());
        }
    }

    if error_path.is_some() || error_page.is_some() || error_name.is_some() {
        let page = RouteCreateModalPage {
            form_name: q.form_name(),
            refresh_table: q.refresh_table(),
            path,
            page_id,
            page_name: String::new(),
            is_active: form.is_active,
            theme: form.theme.clone().unwrap_or_default(),
            theme_choices: theme_choices(&grapes),
            references: Vec::new(),
            error_path,
            error_page,
            error_name,
        };
        let slot_ctx = SlotCtx::from_auth(&ctx);
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }

    let now = Utc::now();
    let am = ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        path: Set(path),
        page_id: Set(page_id.unwrap_or(0)),
        is_active: Set(form.is_active),
        theme: Set(form.theme.unwrap_or_default()),
        grapes_project: Set(None),
    };
    match am.insert(&state.db).await {
        Ok(route) => {
            let _ = sync_refs(&state.db, route.id, &form.references).await;
            respond_create_modal_done::<RouteCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &WebsiteRoutesDetailRouteTag::new(route.id).url(),
            )
        }
        Err(e) => {
            let page = RouteCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                path: form.path,
                page_id,
                page_name: String::new(),
                is_active: true,
                theme: String::new(),
                theme_choices: theme_choices(&grapes),
                references: Vec::new(),
                error_path: Some(e.to_string()),
                error_page: None,
                error_name: None,
            };
            let slot_ctx = SlotCtx::from_auth(&ctx);
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
        }
    }
}

/// HTTP handler: `detail`.
pub async fn detail(
    Cap(state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
{
    let Some(route) = DbRouteEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Redirect::to("/website").into_response();
    };
    let page = node::get_by_id(&state.db, route.page_id)
        .await
        .ok()
        .flatten();
    let page_name = page.as_ref().map(|p| p.name.clone()).unwrap_or_default();
    let editable = page
        .as_ref()
        .map(|p| is_editable_html_name(&p.name))
        .unwrap_or(false);
    let theme_label = if route.theme.is_empty() {
        "None".into()
    } else {
        grapes
            .theme(&route.theme)
            .map(|t| t.label.clone())
            .unwrap_or_else(|| route.theme.clone())
    };
    let refs = load_ref_items(&state.db, route.id).await;
    let page = RouteDetailPage {
        id: route.id,
        path: route.path,
        page_name,
        page_id: route.page_id,
        is_active: route.is_active,
        theme_label,
        references: refs,
        editable,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

/// HTTP handler: `edit_get`.
pub async fn edit_get(
    Cap(state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response
{
    let Some(route) = DbRouteEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Redirect::to("/website").into_response();
    };
    let page_name = node::get_by_id(&state.db, route.page_id)
        .await
        .ok()
        .flatten()
        .map(|n| n.name)
        .unwrap_or_default();
    let page = RouteEditModalPage {
        id: route.id,
        form_name: q.form_name(),
        path: route.path,
        page_id: Some(route.page_id),
        page_name,
        is_active: route.is_active,
        theme: route.theme,
        theme_choices: theme_choices(&grapes),
        references: load_ref_items(&state.db, route.id).await,
        error_path: None,
        error_page: None,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
}

/// HTTP handler: `edit_post`.
pub async fn edit_post(
    Cap(state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<RouteEditBody>,
) -> Response
{
    let Some(route) = DbRouteEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Redirect::to("/website").into_response();
    };
    let path = form.path.trim().to_string();
    let page_id = form.page_id.filter(|i| *i > 0).unwrap_or(route.page_id);
    if path.is_empty() || page_id == 0 {
        let path_empty = path.is_empty();
        let page = RouteEditModalPage {
            id,
            form_name: q.form_name(),
            path,
            page_id: Some(page_id),
            page_name: String::new(),
            is_active: form.is_active,
            theme: form.theme.unwrap_or_default(),
            theme_choices: theme_choices(&grapes),
            references: load_ref_items(&state.db, id).await,
            error_path: path_empty.then(|| "path is required".into()),
            error_page: (page_id == 0).then(|| "template page is required".into()),
        };
        let slot_ctx = SlotCtx::from_auth(&ctx);
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }
    let theme = form.theme.clone().unwrap_or_default();
    let mut am: ActiveModel = route.into();
    am.path = Set(path.clone());
    am.page_id = Set(page_id);
    am.is_active = Set(form.is_active);
    am.theme = Set(theme.clone());
    am.updated_at = Set(Some(Utc::now()));
    if am.update(&state.db).await.is_err() {
        let page = RouteEditModalPage {
            id,
            form_name: q.form_name(),
            path,
            page_id: Some(page_id),
            page_name: String::new(),
            is_active: form.is_active,
            theme,
            theme_choices: theme_choices(&grapes),
            references: load_ref_items(&state.db, id).await,
            error_path: Some("failed to save route".into()),
            error_page: None,
        };
        let slot_ctx = SlotCtx::from_auth(&ctx);
        return html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response();
    }
    let _ = sync_refs(&state.db, id, &form.references).await;
    respond_edit_modal_done::<RouteEditModalKey>(
        &htmx,
        &WebsiteRoutesDetailRouteTag::new(id).url(),
    )
}

/// HTTP handler: `delete_get`.
pub async fn delete_get(
    Cap(state): Cap<WebsiteState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(_q): Query<ModalNameQuery>,
) -> Response
{
    let Some(route) = DbRouteEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Redirect::to("/website").into_response();
    };
    let page = ConfirmDeletePage {
        id: route.id,
        path: route.path,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
}

/// HTTP handler: `delete_post`.
pub async fn delete_post(
    Cap(state): Cap<WebsiteState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    let _ = DbRouteEntity::delete_by_id(id).exec(&state.db).await;
    Redirect::to("/website").into_response()
}
