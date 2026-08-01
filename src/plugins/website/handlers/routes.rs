//! Admin CRUD for `db_routes`.

use std::sync::Arc;

use axum::{
    Form,
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use frunk::{Generic, hlist, into_generic};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use serde::Deserialize;

use crate::{
    components::{FoldSlots, ManyToManyItem, ObjectList, SlotCapability, SlotCtx},
    grapesjs::GrapesJsCapability,
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
            state::WebsiteState,
            templates::{
                ConfirmDeletePage, RouteDetailPage, RouteDetailPageTag, RouteFormPage,
                RouteFormPageTag, RouteListPage, RouteListPageTag, RouteRow, WebsiteConfirmDeletePageTag,
            },
        },
    },
    template::{RenderAppPane, TemplateCapability, TemplateOf},
    traits::get::GetByTag,
    web::{Htmx, html_page_or_app_layout, html_page_with_slots},
};


const PAGE_SIZE: u32 = 20;

#[derive(Debug, Deserialize, Default)]
pub struct RouteListQuery {
    #[serde(default, rename = "Path", alias = "path")]
    pub path: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
}

fn checkbox_on(v: &Option<String>) -> bool {
    matches!(v.as_deref(), Some("on") | Some("true") | Some("1") | Some("yes"))
}

fn parse_ref_ids(raw: &Option<String>) -> Vec<i64> {
    raw.as_deref()
        .unwrap_or("")
        .split([',', ' '])
        .filter_map(|s| s.trim().parse().ok())
        .collect()
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
    let links = RouteRefEntity::find()
        .filter(route_reference::Column::DbRouteId.eq(route_id))
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

pub async fn list<Templates, Slots, Idx, P>(
    Cap(state): Cap<WebsiteState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<RouteListQuery>,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<RouteListPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <RouteListPage as Generic>::Repr> + RenderAppPane + crate::template::RenderTemplate,
{
    let mut query = DbRouteEntity::find().filter(db_route::Column::DeletedAt.is_null());
    let path_f = q.path.clone().unwrap_or_default();
    if !path_f.is_empty() {
        query = query.filter(db_route::Column::Path.contains(&path_f));
    }
    let sort = q.sort.as_deref().unwrap_or("");
    let query = match sort {
        s if s.eq_ignore_ascii_case("Path DESC") => query.order_by_desc(db_route::Column::Path),
        s if s.eq_ignore_ascii_case("Path ASC") || s.eq_ignore_ascii_case("Path") => {
            query.order_by_asc(db_route::Column::Path)
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
    html_page_or_app_layout::<P, _>(
        &htmx,
        hlist![list, path_f, pq],
        &slots,
        &slot_ctx,
    )
}

pub async fn create_get<Templates, Slots, Idx, P>(
    Cap(_state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> maud::Markup
where
    Templates: GetByTag<RouteFormPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <RouteFormPage as Generic>::Repr> + RenderAppPane + crate::template::RenderTemplate,
    Slots: FoldSlots + Clone + Send + Sync + 'static,
{
    let page = RouteFormPage {
        id: None,
        path: String::new(),
        page_id: None,
        page_name: String::new(),
        is_active: true,
        theme: String::new(),
        theme_choices: theme_choices(&grapes),
        references: Vec::new(),
        allow_create_new: true,
        error_path: None,
        error_page: None,
        error_name: None,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    html_page_or_app_layout::<P, _>(&htmx, into_generic(page), &slots, &slot_ctx)
}

pub async fn create_post<Templates, Slots, Idx, P>(
    Cap(state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Form(form): Form<RouteCreateBody>,
) -> Response
where
    Templates: GetByTag<RouteFormPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <RouteFormPage as Generic>::Repr> + RenderAppPane + crate::template::RenderTemplate,
    Slots: FoldSlots + Clone + Send + Sync + 'static,
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
        let page = RouteFormPage {
            id: None,
            path,
            page_id,
            page_name: String::new(),
            is_active: checkbox_on(&form.is_active),
            theme: form.theme.clone().unwrap_or_default(),
            theme_choices: theme_choices(&grapes),
            references: Vec::new(),
            allow_create_new: true,
            error_path,
            error_page,
            error_name,
        };
        let slot_ctx = SlotCtx::from_auth(&ctx);
        return html_page_or_app_layout::<P, _>(&htmx, into_generic(page), &slots, &slot_ctx)
            .into_response();
    }

    let now = Utc::now();
    let am = ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        deleted_at: Set(None),
        path: Set(path),
        page_id: Set(page_id.unwrap_or(0)),
        is_active: Set(checkbox_on(&form.is_active) || form.is_active.is_none()),
        theme: Set(form.theme.unwrap_or_default()),
        grapes_project: Set(None),
    };
    match am.insert(&state.db).await {
        Ok(route) => {
            let _ = sync_refs(&state.db, route.id, &parse_ref_ids(&form.references)).await;
            Redirect::to("/website").into_response()
        }
        Err(e) => {
            let page = RouteFormPage {
                id: None,
                path: form.path,
                page_id,
                page_name: String::new(),
                is_active: true,
                theme: String::new(),
                theme_choices: theme_choices(&grapes),
                references: Vec::new(),
                allow_create_new: true,
                error_path: Some(e.to_string()),
                error_page: None,
                error_name: None,
            };
            let slot_ctx = SlotCtx::from_auth(&ctx);
            html_page_or_app_layout::<P, _>(&htmx, into_generic(page), &slots, &slot_ctx).into_response()
        }
    }
}

pub async fn detail<Templates, Slots, Idx, P>(
    Cap(state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
where
    Templates: GetByTag<RouteDetailPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <RouteDetailPage as Generic>::Repr> + RenderAppPane + crate::template::RenderTemplate,
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
    html_page_or_app_layout::<P, _>(&htmx, into_generic(page), &slots, &slot_ctx).into_response()
}

pub async fn edit_get<Templates, Slots, Idx, P>(
    Cap(state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
where
    Templates: GetByTag<RouteFormPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <RouteFormPage as Generic>::Repr> + RenderAppPane + crate::template::RenderTemplate,
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
    let page_name = node::get_by_id(&state.db, route.page_id)
        .await
        .ok()
        .flatten()
        .map(|n| n.name)
        .unwrap_or_default();
    let page = RouteFormPage {
        id: Some(route.id),
        path: route.path,
        page_id: Some(route.page_id),
        page_name,
        is_active: route.is_active,
        theme: route.theme,
        theme_choices: theme_choices(&grapes),
        references: load_ref_items(&state.db, route.id).await,
        allow_create_new: false,
        error_path: None,
        error_page: None,
        error_name: None,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    html_page_or_app_layout::<P, _>(&htmx, into_generic(page), &slots, &slot_ctx).into_response()
}

pub async fn edit_post<Templates, Slots, Idx, P>(
    Cap(state): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Form(form): Form<RouteEditBody>,
) -> Response
where
    Templates: GetByTag<RouteFormPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <RouteFormPage as Generic>::Repr> + RenderAppPane + crate::template::RenderTemplate,
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
    let path = form.path.trim().to_string();
    let page_id = form.page_id.filter(|i| *i > 0).unwrap_or(route.page_id);
    if path.is_empty() || page_id == 0 {
        let path_empty = path.is_empty();
        let page = RouteFormPage {
            id: Some(id),
            path,
            page_id: Some(page_id),
            page_name: String::new(),
            is_active: checkbox_on(&form.is_active),
            theme: form.theme.unwrap_or_default(),
            theme_choices: theme_choices(&grapes),
            references: load_ref_items(&state.db, id).await,
            allow_create_new: false,
            error_path: path_empty.then(|| "path is required".into()),
            error_page: (page_id == 0).then(|| "template page is required".into()),
            error_name: None,
        };
        let slot_ctx = SlotCtx::from_auth(&ctx);
        return html_page_or_app_layout::<P, _>(&htmx, into_generic(page), &slots, &slot_ctx)
            .into_response();
    }
    let mut am: ActiveModel = route.into();
    am.path = Set(path);
    am.page_id = Set(page_id);
    am.is_active = Set(checkbox_on(&form.is_active));
    am.theme = Set(form.theme.unwrap_or_default());
    am.updated_at = Set(Some(Utc::now()));
    if am.update(&state.db).await.is_err() {
        return Redirect::to(&crate::plugins::website::routes::WebsiteRoutesEditGetRouteTag::new(id).url()).into_response();
    }
    let _ = sync_refs(&state.db, id, &parse_ref_ids(&form.references)).await;
    Redirect::to(&crate::plugins::website::routes::WebsiteRoutesDetailRouteTag::new(id).url()).into_response()
}

pub async fn delete_get<Templates, Slots, Idx, P>(
    Cap(state): Cap<WebsiteState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(_q): Query<crate::plugins::website::handlers::ModalNameQuery>,
) -> Response
where
    Templates: GetByTag<WebsiteConfirmDeletePageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <ConfirmDeletePage as Generic>::Repr> + crate::template::RenderTemplate,
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
    let page = ConfirmDeletePage {
        id: route.id,
        path: route.path,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    html_page_with_slots::<P, _>(into_generic(page), &slots, &slot_ctx).into_response()
}

pub async fn delete_post(
    Cap(state): Cap<WebsiteState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if let Ok(Some(route)) = DbRouteEntity::find_by_id(id)
        .filter(db_route::Column::DeletedAt.is_null())
        .one(&state.db)
        .await
    {
        let mut am: ActiveModel = route.into();
        am.deleted_at = Set(Some(Utc::now()));
        let _ = am.update(&state.db).await;
    }
    Redirect::to("/website").into_response()
}
