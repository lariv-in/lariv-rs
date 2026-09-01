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

use crate::{
    components::{ManyToManyItem, ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    html_form::HtmlFormBody,
    http::Cap,
    picker::respond_picker_select,
    plugins::users::{middleware::RequireAuth, state::AuthContext},
    template::RenderAppPane,
    web::{
        Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done_fk_extra, respond_edit_modal_done,
    },
};

use crate::plugins::crm::{
    entities::{
        lead::Entity as LeadEntity,
        lead_tag::{self, Entity as LeadTagEntity},
    },
    forms::LeadTagForm,
    handlers::{
        ModalNameQuery,
        leads::{HubQuery, query_active_leads, query_converted_leads, query_failed_leads},
    },
    keys::{
        LeadTagCreateModalKey, LeadTagDeleteModalKey, LeadTagEditModalKey, LeadTagLeadsTableKey,
        LeadTagSelectModalKey, LeadTagSelectTableKey, LeadTagTableKey,
    },
    routes::{LeadTagDefaultRouteTag, LeadTagDetailRouteTag},
    scope::{find_lead_tag_scoped, scope_superuser},
    state::CrmState,
    templates::{
        ConfirmDeletePage, LeadTagCreateModalPage, LeadTagDetailPage, LeadTagEditModalPage,
        LeadTagListPage, LeadTagOption, LeadTagRow, LeadTagSelectPage,
    },
};

const DEFAULT_TAG_COLOR: &str = "#6366f1";

const TAG_COLORS: &[&str] = &[
    "#ef4444", "#f97316", "#f59e0b", "#84cc16", "#22c55e", "#14b8a6", "#06b6d4", "#3b82f6",
    "#6366f1", "#8b5cf6", "#a855f7", "#ec4899",
];

pub fn random_tag_color() -> String {
    use rand::seq::SliceRandom;
    TAG_COLORS
        .choose(&mut rand::thread_rng())
        .copied()
        .unwrap_or(DEFAULT_TAG_COLOR)
        .to_string()
}

fn normalize_tag_color(raw: &str) -> String {
    let s = raw.trim();
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        format!("#{}", hex.to_ascii_lowercase())
    } else {
        random_tag_color()
    }
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct LeadTagListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct LeadTagSelectQuery {
    #[serde(flatten)]
    pub filter: LeadTagListQuery,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

async fn query_tags(
    db: &sea_orm::DatabaseConnection,
    q: &LeadTagListQuery,
    auth: &AuthContext,
) -> (Vec<lead_tag::Model>, u32, u64) {
    let mut query = LeadTagEntity::find();
    let name = q.name.clone().unwrap_or_default();
    if !name.is_empty() {
        query = query.filter(lead_tag::Column::Name.contains(&name));
    }
    query = scope_superuser(query, auth);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(lead_tag::Column::Name),
        _ => query.order_by_desc(lead_tag::Column::Id),
    };
    let page = q.page.get();
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

/// Tags currently linked to `lead_id`.
pub async fn load_tags_for_lead(
    db: &sea_orm::DatabaseConnection,
    lead_id: i64,
) -> Vec<lead_tag::Model> {
    let result = LeadEntity::find_by_id(lead_id)
        .find_with_related(LeadTagEntity)
        .all(db)
        .await
        .unwrap_or_default();
    result.into_iter().flat_map(|(_, tags)| tags).collect()
}

/// Tags currently linked to `lead_id`, as [`ManyToManyItem`]s for form pre-fill.
pub async fn load_tag_items_for_lead(
    db: &sea_orm::DatabaseConnection,
    lead_id: i64,
) -> Vec<ManyToManyItem> {
    load_tags_for_lead(db, lead_id)
        .await
        .into_iter()
        .map(|t| ManyToManyItem::new(t.id.to_string(), t.name).with_color(t.color))
        .collect()
}

/// Resolve submitted tag ids to [`ManyToManyItem`]s for re-rendering a form on error.
pub async fn tag_items_from_ids(
    db: &sea_orm::DatabaseConnection,
    ids: &[i64],
) -> Vec<ManyToManyItem> {
    if ids.is_empty() {
        return Vec::new();
    }
    let tags = LeadTagEntity::find()
        .filter(lead_tag::Column::Id.is_in(ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default();
    ids.iter()
        .filter_map(|id| {
            tags.iter().find(|t| t.id == *id).map(|t| {
                ManyToManyItem::new(t.id.to_string(), t.name.clone()).with_color(t.color.clone())
            })
        })
        .collect()
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct LeadTagDetailQuery {
    #[serde(default)]
    pub tab: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: QueryPageSize,
    #[serde(default)]
    pub sort: Option<String>,
}

fn tags_list_url() -> String {
    LeadTagDefaultRouteTag.url()
}

pub async fn list(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<LeadTagListQuery>,
) -> maud::Markup {
    let (models, page, total) = query_tags(&state.db, &q, &ctx).await;
    let rows = models
        .into_iter()
        .map(|t| LeadTagRow {
            id: t.id,
            name: t.name,
            color: t.color,
        })
        .collect();
    let page = LeadTagListPage {
        tags: ObjectList::from_page(rows, page, q.page_size.get(), total),
        filter_name: q.name.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: ctx.user.is_superuser,
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<LeadTagTableKey>() {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn select(
    Cap(state): Cap<CrmState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<LeadTagSelectQuery>,
) -> maud::Markup {
    let (models, page, total) = query_tags(&state.db, &q.filter, &ctx).await;
    let rows = models
        .into_iter()
        .map(|t| LeadTagOption {
            id: t.id,
            name: t.name,
            color: t.color,
        })
        .collect();
    let page = LeadTagSelectPage {
        tags: ObjectList::from_page(rows, page, q.filter.page_size.get(), total),
        filter_name: q.filter.name.clone().unwrap_or_default(),
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.clone().unwrap_or_else(|| "Tags".into()),
        can_edit: ctx.user.is_superuser,
        page_size: q.filter.page_size.get(),
    };
    respond_picker_select::<LeadTagSelectTableKey, LeadTagSelectModalKey, _>(&htmx, &page)
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    if !ctx.user.is_superuser {
        return maud::html! { div class="alert alert-error" { "Forbidden" } };
    }
    let page = LeadTagCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        color: random_tag_color(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<LeadTagForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&tags_list_url()).into_response();
    }
    let name = form.name.trim().to_string();
    if name.is_empty() {
        let page = LeadTagCreateModalPage {
            form_name: q.form_name(),
            refresh_table: q.refresh_table(),
            target_input: q.target_input(),
            name: form.name,
            color: normalize_tag_color(&form.color),
            error: "name is required".to_string(),
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let now = Utc::now();
    let model = lead_tag::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(name),
        color: Set(normalize_tag_color(&form.color)),
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done_fk_extra::<LeadTagCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &LeadTagDetailRouteTag::new(saved.id).url(),
            saved.id,
            &saved.name,
            &q.target_input(),
            &[("color", saved.color.as_str())],
        ),
        Err(e) => {
            let page = LeadTagCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                name: form.name,
                color: normalize_tag_color(&form.color),
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn detail(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<LeadTagDetailQuery>,
) -> Response {
    let Some(tag) = find_lead_tag_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&tags_list_url()).into_response();
    };
    let tab = q.tab.as_deref().unwrap_or("active").to_string();
    let hub = HubQuery {
        tab: Some(tab.clone()),
        page: q.page,
        page_size: q.page_size,
        tags: vec![id],
        sort: q.sort.clone(),
        ..Default::default()
    };
    let (rows, page, total) = match tab.as_str() {
        "converted" => query_converted_leads(&state.db, &hub, hub.page_size.get()).await,
        "failed" => query_failed_leads(&state.db, &hub, hub.page_size.get()).await,
        _ => query_active_leads(&state.db, &hub, hub.page_size.get()).await,
    };
    let page = LeadTagDetailPage {
        id: tag.id,
        name: tag.name,
        color: tag.color,
        can_edit: ctx.user.is_superuser,
        tab,
        leads: ObjectList::from_page(rows, page, q.page_size.get(), total),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    if htmx.targets::<LeadTagLeadsTableKey>() {
        return page.render_leads_table().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_get(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&tags_list_url()).into_response();
    }
    let Some(tag) = find_lead_tag_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&tags_list_url()).into_response();
    };
    let page = LeadTagEditModalPage {
        id: tag.id,
        form_name: q.form_name(),
        name: tag.name,
        color: tag.color,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<LeadTagForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&tags_list_url()).into_response();
    }
    let Some(existing) = find_lead_tag_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&tags_list_url()).into_response();
    };
    let name = form.name.trim().to_string();
    if name.is_empty() {
        let page = LeadTagEditModalPage {
            id,
            form_name: q.form_name(),
            name: form.name,
            color: normalize_tag_color(&form.color),
            error: "name is required".to_string(),
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let now = Utc::now();
    let mut am: lead_tag::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(name);
    am.color = Set(normalize_tag_color(&form.color));
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<LeadTagEditModalKey>(
            &htmx,
            &LeadTagDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = LeadTagEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                color: normalize_tag_color(&form.color),
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
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
        modal_uid: LeadTagDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this tag? It will be removed from all leads."
            .into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_crm.LeadTagDeleteForm".into()),
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
        return Redirect::to(&tags_list_url()).into_response();
    }
    match LeadTagEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&tags_list_url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete lead tag");
            let page = ConfirmDeletePage {
                modal_uid: LeadTagDeleteModalKey::ID.to_string(),
                message:
                    "Are you sure you want to delete this tag? It will be removed from all leads."
                        .into(),
                form_name: "p_crm.LeadTagDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
