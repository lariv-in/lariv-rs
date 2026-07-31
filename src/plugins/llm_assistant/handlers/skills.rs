use axum::{
    Form,
    body::Body,
    extract::{Multipart, Path, Query},
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use frunk::{Generic, hlist};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder,
};
use serde::Deserialize;

use crate::{
    components::{FoldSlots, ManyToManyItem, ObjectList, SlotCapability, SlotCtx, SwapKey},
    html_form::multipart::collect_multipart,
    http::Cap,
    plugins::{
        filesystem::{
            entities::filesystem_node::{
                Column as VNodeColumn, Entity as VNodeEntity,
            },
            state::FilesystemState,
        },
        llm_assistant::{
            entities::{
                skill::{self, Entity as SkillEntity},
                skill_file_link,
            },
            forms::SkillForm,
            keys::{SkillDeleteModalKey, SkillsTableKey},
            skill_zip::{export_skill, import_skill},
            state::LlmAssistantState,
            templates::{
                SkillConfirmDeletePageTag, SkillDetailPage, SkillDetailPageTag, SkillFormPage,
                SkillFormPageTag, SkillImportPageTag, SkillListPage, SkillListPageTag, SkillRow,
                ConfirmDeletePage, SkillImportPage,
            },
        },
        users::middleware::RequireAuth,
    },
    template::{RenderAppPane, TemplateCapability, TemplateOf},
    traits::get::GetByTag,
    web::{Htmx, html_page_or_app_layout, html_page_with_slots},
};

use super::ModalNameQuery;

const PAGE_SIZE: u32 = 12;

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

fn format_updated_at(dt: Option<chrono::DateTime<Utc>>) -> String {
    dt.map(|d| d.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

async fn query_skills(
    db: &sea_orm::DatabaseConnection,
    q: &SkillListQuery,
) -> (Vec<skill::Model>, u32, u64) {
    let mut query = SkillEntity::find().filter(skill::Column::DeletedAt.is_null());
    let name = q.name.clone().unwrap_or_default();
    if !name.is_empty() {
        query = query.filter(skill::Column::Name.contains(&name));
    }
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(skill::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(skill::Column::Name)
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
) -> ObjectList<SkillRow> {
    let (models, page, total) = query_skills(db, q).await;
    let rows = models
        .into_iter()
        .map(|s| SkillRow {
            id: s.id,
            name: s.name,
            description: s.description,
            updated_at: format_updated_at(s.updated_at),
        })
        .collect();
    ObjectList::from_page(rows, page, PAGE_SIZE, total)
}

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
        .filter(|n| n.deleted_at.is_none())
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
        .map(|(id, name)| ManyToManyItem {
            key: id.to_string(),
            value: name,
        })
        .collect()
}

async fn file_items_from_ids(
    db: &sea_orm::DatabaseConnection,
    ids: &[i64],
) -> Vec<ManyToManyItem> {
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
            nodes.iter().find(|n| n.id == *id).map(|n| ManyToManyItem {
                key: n.id.to_string(),
                value: n.name.clone(),
            })
        })
        .collect()
}

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

pub async fn list<Templates, Slots, Idx, P>(
    Cap(state): Cap<LlmAssistantState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<SkillListQuery>,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<SkillListPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <SkillListPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let skills = load_skills_page(&state.db, &q).await;
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
        return page.render_main();
    }
    if htmx.wants_app_layout() {
        return page.render_pane();
    }
    html_page_with_slots::<P, Slots>(
        hlist![
            page.skills,
            page.filter_name,
            page.sort,
            page.path_and_query,
        ],
        &slots,
        &SlotCtx {
            name: Some(ctx.user.name.clone()),
            role: Some(ctx.role.clone()),
            is_superuser: ctx.user.is_superuser,
        },
    )
}

pub async fn detail<Templates, Slots, Idx, P>(
    Cap(state): Cap<LlmAssistantState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<SkillDetailPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <SkillDetailPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let Some(skill) = SkillEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .filter(|s| s.deleted_at.is_none())
    else {
        return Redirect::to("/llm-assistant/skills/").into_response();
    };
    let files = load_files_for_skill(&state.db, id).await;
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![
            skill.id,
            skill.name,
            skill.description,
            skill.content,
            files,
        ],
        &slots,
        &SlotCtx {
            name: Some(ctx.user.name.clone()),
            role: Some(ctx.role.clone()),
            is_superuser: ctx.user.is_superuser,
        },
    )
    .into_response()
}

pub async fn create_get<Templates, Slots, Idx, P>(
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<SkillFormPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <SkillFormPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![
            0_i64,
            String::new(),
            String::new(),
            String::new(),
            Vec::<ManyToManyItem>::new(),
            String::new(),
        ],
        &slots,
        &SlotCtx {
            name: Some(ctx.user.name.clone()),
            role: Some(ctx.role.clone()),
            is_superuser: ctx.user.is_superuser,
        },
    )
    .into_response()
}

pub async fn create_post<Templates, Slots, Idx, P>(
    Cap(state): Cap<LlmAssistantState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Form(form): Form<SkillForm>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<SkillFormPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <SkillFormPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let now = Utc::now();
    let model = skill::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        deleted_at: Set(None),
        name: Set(form.name.clone()),
        description: Set(form.description.clone()),
        content: Set(form.content.clone()),
    };
    match model.insert(&state.db).await {
        Ok(saved) => {
            let _ = sync_skill_files(&state.db, saved.id, &form.files).await;
            htmx.redirect(&format!("/llm-assistant/skills/{}/", saved.id))
        }
        Err(e) => {
            let file_items = file_items_from_ids(&state.db, &form.files).await;
            html_page_or_app_layout::<P, Slots>(
                &htmx,
                hlist![
                    0_i64,
                    form.name,
                    form.description,
                    form.content,
                    file_items,
                    e.to_string(),
                ],
                &slots,
                &SlotCtx {
                    name: Some(ctx.user.name.clone()),
                    role: Some(ctx.role.clone()),
                    is_superuser: ctx.user.is_superuser,
                },
            )
            .into_response()
        }
    }
}

pub async fn edit_get<Templates, Slots, Idx, P>(
    Cap(state): Cap<LlmAssistantState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<SkillFormPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <SkillFormPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let Some(skill) = SkillEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .filter(|s| s.deleted_at.is_none())
    else {
        return Redirect::to("/llm-assistant/skills/").into_response();
    };
    let files = load_file_items_for_skill(&state.db, id).await;
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![
            skill.id,
            skill.name,
            skill.description,
            skill.content,
            files,
            String::new(),
        ],
        &slots,
        &SlotCtx {
            name: Some(ctx.user.name.clone()),
            role: Some(ctx.role.clone()),
            is_superuser: ctx.user.is_superuser,
        },
    )
    .into_response()
}

pub async fn edit_post<Templates, Slots, Idx, P>(
    Cap(state): Cap<LlmAssistantState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Form(form): Form<SkillForm>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<SkillFormPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <SkillFormPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let Some(skill) = SkillEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .filter(|s| s.deleted_at.is_none())
    else {
        return Redirect::to("/llm-assistant/skills/").into_response();
    };
    let mut am: skill::ActiveModel = skill.into();
    am.name = Set(form.name.clone());
    am.description = Set(form.description.clone());
    am.content = Set(form.content.clone());
    am.updated_at = Set(Some(Utc::now()));
    match am.update(&state.db).await {
        Ok(_) => {
            let _ = sync_skill_files(&state.db, id, &form.files).await;
            htmx.redirect(&format!("/llm-assistant/skills/{id}/"))
        }
        Err(e) => {
            let file_items = file_items_from_ids(&state.db, &form.files).await;
            html_page_or_app_layout::<P, Slots>(
                &htmx,
                hlist![
                    id,
                    form.name,
                    form.description,
                    form.content,
                    file_items,
                    e.to_string(),
                ],
                &slots,
                &SlotCtx {
                    name: Some(ctx.user.name.clone()),
                    role: Some(ctx.role.clone()),
                    is_superuser: ctx.user.is_superuser,
                },
            )
            .into_response()
        }
    }
}

pub async fn delete_get<Templates, Slots, Idx, P>(
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<SkillConfirmDeletePageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <ConfirmDeletePage as Generic>::Repr> + crate::template::RenderTemplate,
{
    html_page_with_slots::<P, Slots>(
        hlist![
            SkillDeleteModalKey::ID.to_string(),
            "Are you sure you want to delete this skill?".into(),
            q.name
                .clone()
                .unwrap_or_else(|| "p_llm_assistant.SkillDeleteForm".into()),
            format!("/llm-assistant/skills/{id}/delete/"),
        ],
        &slots,
        &SlotCtx {
            name: Some(ctx.user.name.clone()),
            role: Some(ctx.role.clone()),
            is_superuser: ctx.user.is_superuser,
        },
    )
}

pub async fn delete_post(
    Cap(state): Cap<LlmAssistantState>,
    RequireAuth(_ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if let Some(skill) = SkillEntity::find_by_id(id).one(&state.db).await.ok().flatten() {
        let mut am: skill::ActiveModel = skill.into();
        am.deleted_at = Set(Some(Utc::now()));
        let _ = am.update(&state.db).await;
    }
    htmx.redirect("/llm-assistant/skills/")
}

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

pub async fn import_get<Templates, Slots, Idx, P>(
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<SkillImportPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <SkillImportPage as Generic>::Repr> + crate::template::RenderTemplate,
{
    html_page_with_slots::<P, Slots>(
        hlist![],
        &slots,
        &SlotCtx {
            name: Some(ctx.user.name.clone()),
            role: Some(ctx.role.clone()),
            is_superuser: ctx.user.is_superuser,
        },
    )
}

pub async fn import_post(
    Cap(state): Cap<LlmAssistantState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(_ctx): RequireAuth,
    htmx: Htmx,
    multipart: Multipart,
) -> Response {
    let parts = match collect_multipart(multipart, &[], &["File"]).await {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    let file = parts
        .files
        .get("File")
        .or_else(|| parts.file_lists.get("File").and_then(|v| v.first()));
    let Some(file) = file else {
        return (StatusCode::BAD_REQUEST, "zip file is required").into_response();
    };
    let bytes = match tokio::fs::read(file.path()).await {
        Ok(b) => b,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    if bytes.len() > 10 * 1024 * 1024 {
        return (StatusCode::BAD_REQUEST, "zip file too large").into_response();
    }
    match import_skill(&state.db, fs.store.as_ref(), &bytes).await {
        Ok(skill) => htmx.redirect(&format!("/llm-assistant/skills/{}/", skill.id)),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}
