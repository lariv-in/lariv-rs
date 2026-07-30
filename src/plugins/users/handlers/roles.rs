use axum::{
    Form,
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use frunk::{Generic, hlist};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use serde::Deserialize;

use crate::{
    components::{FoldSlots, ObjectList, SlotCapability, SlotCtx, SwapKey},
    http::Cap,
    plugins::users::{
        entities::role::{self, Entity as RoleEntity},
        keys::{RoleDeleteModalKey, RoleSelectTableKey, RoleTableKey},
        middleware::RequireSuperuser,
        state::UsersState,
        templates::{
            ConfirmDeletePage, RoleCreateModalPage, RoleDetailPage, RoleFormPage, RoleListPage,
            RoleOption, RoleSelectPage, UsersConfirmDeletePageTag, UsersRoleCreateModalPageTag,
            UsersRoleDetailPageTag, UsersRoleFormPageTag, UsersRoleListPageTag,
            UsersRoleSelectPageTag,
        },
    },
    template::{RenderAppPane, TemplateCapability, TemplateOf},
    traits::get::GetByTag,
    web::{Htmx, html_page_or_app_layout, html_page_with_slots},
};

use super::users::ModalNameQuery;

const PAGE_SIZE: u32 = 12;

#[derive(Debug, Deserialize, Default)]
pub struct RoleListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

async fn load_roles_page(
    db: &sea_orm::DatabaseConnection,
    q: &RoleListQuery,
) -> ObjectList<RoleOption> {
    let mut query = RoleEntity::find().filter(role::Column::DeletedAt.is_null());
    let name = q.name.clone().unwrap_or_default();
    if !name.is_empty() {
        query = query.filter(role::Column::Name.contains(&name));
    }
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(role::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(role::Column::Name)
        }
        _ => query.order_by_asc(role::Column::Id),
    };

    let page = q.page.unwrap_or(1).max(1);
    let paginator = query.paginate(db, PAGE_SIZE as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|r| RoleOption {
            id: r.id,
            name: r.name,
        })
        .collect();
    ObjectList::from_page(rows, page, PAGE_SIZE, total)
}

pub async fn list<Templates, Slots, Idx, P>(
    Cap(state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireSuperuser(ctx): RequireSuperuser,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<RoleListQuery>,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersRoleListPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <RoleListPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let roles = load_roles_page(&state.db, &q).await;
    let page = RoleListPage {
        roles,
        filter_name: q.name.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    if htmx.targets::<RoleTableKey>() {
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
            page.roles,
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

pub async fn select<Templates, Slots, Idx, P>(
    Cap(state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireSuperuser(ctx): RequireSuperuser,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<RoleListQuery>,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersRoleSelectPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <RoleSelectPage as Generic>::Repr> + crate::template::RenderTemplate,
{
    let roles = load_roles_page(&state.db, &q).await;
    let page = RoleSelectPage {
        roles,
        filter_name: q.name.clone().unwrap_or_default(),
        target_input: q.target_input.clone().unwrap_or_else(|| "RoleID".into()),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    if htmx.targets::<RoleSelectTableKey>() {
        return page.render_table();
    }
    html_page_with_slots::<P, Slots>(
        hlist![
            page.roles,
            page.filter_name,
            page.target_input,
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
    Cap(state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireSuperuser(ctx): RequireSuperuser,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersRoleDetailPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <RoleDetailPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let Some(role) = RoleEntity::find_by_id(id).one(&state.db).await.ok().flatten() else {
        return Redirect::to("/users/roles/").into_response();
    };
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![role.id, role.name],
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
    RequireSuperuser(ctx): RequireSuperuser,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersRoleCreateModalPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <RoleCreateModalPage as Generic>::Repr> + crate::template::RenderTemplate,
{
    html_page_with_slots::<P, Slots>(
        hlist![q.name.clone().unwrap_or_default(), String::new(), String::new()],
        &slots,
        &SlotCtx {
            name: Some(ctx.user.name.clone()),
            role: Some(ctx.role.clone()),
            is_superuser: ctx.user.is_superuser,
        },
    )
}

#[derive(Deserialize)]
pub struct RoleForm {
    #[serde(rename = "Name", alias = "name")]
    pub name: String,
}

pub async fn create_post<Templates, Slots, Idx, P>(
    Cap(state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireSuperuser(ctx): RequireSuperuser,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<RoleForm>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersRoleCreateModalPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <RoleCreateModalPage as Generic>::Repr> + crate::template::RenderTemplate,
{
    let now = Utc::now();
    let model = role::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        deleted_at: Set(None),
        name: Set(form.name.clone()),
    };
    match model.insert(&state.db).await {
        Ok(role) => htmx.redirect(&format!("/users/roles/{}/", role.id)),
        Err(e) => html_page_with_slots::<P, Slots>(
            hlist![
                q.name.clone().unwrap_or_default(),
                form.name,
                e.to_string(),
            ],
            &slots,
            &SlotCtx {
                name: Some(ctx.user.name.clone()),
                role: Some(ctx.role.clone()),
                is_superuser: ctx.user.is_superuser,
            },
        )
        .into_response(),
    }
}

pub async fn edit_get<Templates, Slots, Idx, P>(
    Cap(state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireSuperuser(ctx): RequireSuperuser,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersRoleFormPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <RoleFormPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let Some(role) = RoleEntity::find_by_id(id).one(&state.db).await.ok().flatten() else {
        return Redirect::to("/users/roles/").into_response();
    };
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![role.id, role.name, String::new()],
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
    Cap(state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireSuperuser(ctx): RequireSuperuser,
    htmx: Htmx,
    Path(id): Path<i64>,
    Form(form): Form<RoleForm>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersRoleFormPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <RoleFormPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let Some(role) = RoleEntity::find_by_id(id).one(&state.db).await.ok().flatten() else {
        return Redirect::to("/users/roles/").into_response();
    };
    let mut am: role::ActiveModel = role.into();
    am.name = Set(form.name.clone());
    am.updated_at = Set(Some(Utc::now()));
    match am.update(&state.db).await {
        Ok(_) => htmx.redirect(&format!("/users/roles/{id}")),
        Err(e) => html_page_or_app_layout::<P, Slots>(
            &htmx,
            hlist![id, form.name, e.to_string()],
            &slots,
            &SlotCtx {
                name: Some(ctx.user.name.clone()),
                role: Some(ctx.role.clone()),
                is_superuser: ctx.user.is_superuser,
            },
        )
        .into_response(),
    }
}

pub async fn delete_get<Templates, Slots, Idx, P>(
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireSuperuser(ctx): RequireSuperuser,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersConfirmDeletePageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <ConfirmDeletePage as Generic>::Repr> + crate::template::RenderTemplate,
{
    html_page_with_slots::<P, Slots>(
        hlist![
            RoleDeleteModalKey::ID.to_string(),
            "Are you sure you want to delete this role?".into(),
            q.name
                .clone()
                .unwrap_or_else(|| "p_users.RoleDeleteForm".into()),
            format!("/users/roles/{id}/delete/"),
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
    Cap(state): Cap<UsersState>,
    RequireSuperuser(_ctx): RequireSuperuser,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if let Some(role) = RoleEntity::find_by_id(id).one(&state.db).await.ok().flatten() {
        let mut am: role::ActiveModel = role.into();
        am.deleted_at = Set(Some(Utc::now()));
        let _ = am.update(&state.db).await;
    }
    htmx.redirect("/users/roles/")
}
