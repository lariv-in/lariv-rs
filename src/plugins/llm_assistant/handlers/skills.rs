use axum::{
    body::Body,
    extract::{Multipart, Path, Query},
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder,
};
use serde::Deserialize;

use crate::template::RenderAppPane;
use crate::{
    components::{
        ManyToManyItem, ObjectList, SharedChromeFolder, SlotCtx, SwapKey, DEFAULT_PAGE_SIZE,
    },
    html_form::{HtmlForm, HtmlFormBody},
    http::Cap,
    plugins::{
        filesystem::{
            entities::filesystem_node::{Column as VNodeColumn, Entity as VNodeEntity},
            state::FilesystemState,
        },
        llm_assistant::{
            entities::{
                skill::{self, Entity as SkillEntity},
                skill_file_link,
            },
            forms::{SkillForm, SkillImportForm},
            keys::{SkillCreateModalKey, SkillDeleteModalKey, SkillEditModalKey, SkillsTableKey},
            routes::SkillsDetailRouteTag,
            skill_zip::{export_skill, import_skill},
            state::LlmAssistantState,
            templates::{
                ConfirmDeletePage, SkillCreateModalPage, SkillDetailPage, SkillEditModalPage,
                SkillImportPage, SkillListPage, SkillRow,
            },
        },
        users::middleware::RequireAuth,
    },
    web::{
        html_built_page_or_app_layout, html_built_page_with_slots, respond_create_modal_done,
        respond_edit_modal_done, Htmx,
    },
};

use super::ModalNameQuery;

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, Deserialize, Default)]
pub struct SkillListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn format_updated_at(dt: Option<chrono::DateTime<Utc>>, tz: &str) -> String {
    crate::datetime::DatetimeLabel::short_optional(dt, tz).into_string()
}

async fn query_skills(
    db: &sea_orm::DatabaseConnection,
    q: &SkillListQuery,
) -> (Vec<skill::Model>, u32, u64) {
    let mut query = SkillEntity::find();
    let name = q.name.clone().unwrap_or_default();
    if !name.is_empty() {
        query = crate::db::trigram::apply_text_search(
            query,
            db.get_database_backend(),
            &[skill::Column::Name, skill::Column::Description],
            &name,
        );
    }
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(skill::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(skill::Column::Name)
        }
        s if s.eq_ignore_ascii_case("Description DESC") => {
            query.order_by_desc(skill::Column::Description)
        }
        s if s.eq_ignore_ascii_case("Description ASC") || s.eq_ignore_ascii_case("Description") => {
            query.order_by_asc(skill::Column::Description)
        }
        _ => query.order_by_desc(skill::Column::Id),
    };

    let page = q.page.unwrap_or(1).max(1);
    let paginator = query.paginate(db, PAGE_SIZE as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

async fn load_skills_page(
    db: &sea_orm::DatabaseConnection,
    q: &SkillListQuery,
    tz: &str,
) -> ObjectList<SkillRow> {
    let (models, page, total) = query_skills(db, q).await;
    let rows = models
        .into_iter()
        .map(|s| SkillRow {
            id: s.id,
            name: s.name,
            description: s.description,
            updated_at: format_updated_at(s.updated_at, tz),
        })
        .collect();
    ObjectList::from_page(rows, page, PAGE_SIZE, total)
}

/// HTTP handler: `load_files_for_skill`.
pub async fn load_files_for_skill(
    db: &sea_orm::DatabaseConnection,
    skill_id: i64,
) -> Vec<(i64, String)> {
    let result = SkillEntity::find_by_id(skill_id)
        .find_with_related(VNodeEntity)
        .all(db)
        .await
        .unwrap_or_default();
    result
        .into_iter()
        .flat_map(|(_, nodes)| nodes)
        .map(|n| (n.id, n.name))
        .collect()
}

async fn load_file_items_for_skill(
    db: &sea_orm::DatabaseConnection,
    skill_id: i64,
) -> Vec<ManyToManyItem> {
    load_files_for_skill(db, skill_id)
        .await
        .into_iter()
        .map(|(id, name)| ManyToManyItem::new(id.to_string(), name))
        .collect()
}

async fn file_items_from_ids(db: &sea_orm::DatabaseConnection, ids: &[i64]) -> Vec<ManyToManyItem> {
    if ids.is_empty() {
        return Vec::new();
    }
    let nodes = VNodeEntity::find()
        .filter(VNodeColumn::Id.is_in(ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default();
    ids.iter()
        .filter_map(|id| {
            nodes
                .iter()
                .find(|n| n.id == *id)
                .map(|n| ManyToManyItem::new(n.id.to_string(), n.name.clone()))
        })
        .collect()
}

/// HTTP handler: `sync_skill_files`.
pub async fn sync_skill_files(
    db: &sea_orm::DatabaseConnection,
    skill_id: i64,
    file_ids: &[i64],
) -> Result<(), DbErr> {
    skill_file_link::Entity::delete_many()
        .filter(skill_file_link::Column::SkillId.eq(skill_id))
        .exec(db)
        .await?;
    for &vnode_id in file_ids {
        skill_file_link::ActiveModel {
            skill_id: Set(skill_id),
            v_node_id: Set(vnode_id),
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

/// HTTP handler: `list`.
pub async fn list(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<SkillListQuery>,
) -> maud::Markup {
    let skills = load_skills_page(&state.db, &q, &ctx.timezone).await;
    let page = SkillListPage {
        skills,
        filter_name: q.name.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    if htmx.targets::<SkillsTableKey>() {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `detail`.
pub async fn detail(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(skill) = crate::web::opt_or_log(
        SkillEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    ) else {
        return Redirect::to("/llm-assistant/skills/").into_response();
    };
    let files = load_files_for_skill(&state.db, id).await;
    let page = SkillDetailPage {
        id: skill.id,
        name: skill.name,
        description: skill.description,
        content: skill.content,
        files,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `create_get`.
pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = SkillCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        name: String::new(),
        description: String::new(),
        content: String::new(),
        files: Vec::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `create_post`.
pub async fn create_post(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<SkillForm>,
) -> Response {
    let now = Utc::now();
    let model = skill::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(form.name.clone()),
        description: Set(form.description.clone()),
        content: Set(form.content.clone()),
    };
    match model.insert(&state.db).await {
        Ok(saved) => {
            if let Err(e) = sync_skill_files(&state.db, saved.id, &form.files).await {
                let file_items = file_items_from_ids(&state.db, &form.files).await;
                let page = SkillCreateModalPage {
                    form_name: q.form_name(),
                    refresh_table: q.refresh_table(),
                    name: form.name,
                    description: form.description,
                    content: form.content,
                    files: file_items,
                    error: e.to_string(),
                };
                return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                    .into_response();
            }
            respond_create_modal_done::<SkillCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &SkillsDetailRouteTag::new(saved.id).url(),
            )
        }
        Err(e) => {
            let file_items = file_items_from_ids(&state.db, &form.files).await;
            let page = SkillCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                name: form.name,
                description: form.description,
                content: form.content,
                files: file_items,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

/// HTTP handler: `edit_get`.
pub async fn edit_get(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(skill) = crate::web::opt_or_log(
        SkillEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    ) else {
        return Redirect::to("/llm-assistant/skills/").into_response();
    };
    let files = load_file_items_for_skill(&state.db, id).await;
    let page = SkillEditModalPage {
        id: skill.id,
        form_name: q.form_name(),
        name: skill.name,
        description: skill.description,
        content: skill.content,
        files,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `edit_post`.
pub async fn edit_post(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<SkillForm>,
) -> Response {
    let Some(skill) = crate::web::opt_or_log(
        SkillEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    ) else {
        return Redirect::to("/llm-assistant/skills/").into_response();
    };
    let mut am: skill::ActiveModel = skill.into();
    am.name = Set(form.name.clone());
    am.description = Set(form.description.clone());
    am.content = Set(form.content.clone());
    am.updated_at = Set(Some(Utc::now()));
    match am.update(&state.db).await {
        Ok(_) => {
            if let Err(e) = sync_skill_files(&state.db, id, &form.files).await {
                let file_items = file_items_from_ids(&state.db, &form.files).await;
                let page = SkillEditModalPage {
                    id,
                    form_name: q.form_name(),
                    name: form.name,
                    description: form.description,
                    content: form.content,
                    files: file_items,
                    error: e.to_string(),
                };
                return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                    .into_response();
            }
            respond_edit_modal_done::<SkillEditModalKey>(
                &htmx,
                &SkillsDetailRouteTag::new(id).url(),
            )
        }
        Err(e) => {
            let file_items = file_items_from_ids(&state.db, &form.files).await;
            let page = SkillEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                description: form.description,
                content: form.content,
                files: file_items,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

/// HTTP handler: `delete_get`.
pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: SkillDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this skill?".into(),
        name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_llm_assistant.SkillDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `delete_post`.
pub async fn delete_post(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match SkillEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect("/llm-assistant/skills/"),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete skill");
            let page = ConfirmDeletePage {
                modal_uid: SkillDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this skill?".into(),
                name: "p_llm_assistant.SkillDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

/// HTTP handler: `export_skill_handler`.
pub async fn export_skill_handler(
    Cap(state): Cap<LlmAssistantState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    match export_skill(&state.db, fs.store.as_ref(), id).await {
        Ok((bytes, filename)) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/zip")
            .header(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}.zip\""),
            )
            .body(Body::from(bytes))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        Err(e) => (StatusCode::NOT_FOUND, e).into_response(),
    }
}

/// HTTP handler: `import_get`.
pub async fn import_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
) -> maud::Markup {
    let page = SkillImportPage;
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `import_post`.
pub async fn import_post(
    Cap(state): Cap<LlmAssistantState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(_ctx): RequireAuth,
    htmx: Htmx,
    multipart: Multipart,
) -> Response {
    let parsed = match SkillImportForm::from_multipart(multipart).await {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    let bytes = match tokio::fs::read(parsed.file.path()).await {
        Ok(b) => b,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    if bytes.len() > 10 * 1024 * 1024 {
        return (StatusCode::BAD_REQUEST, "zip file too large").into_response();
    }
    match import_skill(&state.db, fs.store.as_ref(), &bytes).await {
        Ok(skill) => htmx.redirect(&SkillsDetailRouteTag::new(skill.id).url()),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}
