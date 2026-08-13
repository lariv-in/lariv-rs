use axum::{
    Form,
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

use crate::template::RenderAppPane;
use crate::{
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    http::Cap,
    plugins::users::{
        entities::role::{self, Entity as RoleEntity},
        keys::{
            RoleCreateModalKey, RoleDeleteModalKey, RoleEditModalKey, RoleSelectTableKey,
            RoleTableKey,
        },
        middleware::RequireStaff,
        routes::UsersRolesDetailRouteTag,
        state::UsersState,
        templates::{
            ConfirmDeletePage, RoleCreateModalPage, RoleDetailPage, RoleEditModalPage,
            RoleListPage, RoleOption, RoleSelectPage,
        },
    },
    web::{
        Htmx, html_built_page_or_app_layout, html_built_page_with_slots, respond_create_modal_done,
        respond_edit_modal_done,
    },
};

use super::users::ModalNameQuery;

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

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
    let mut query = RoleEntity::find();
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

/// HTTP handler: `list`.
pub async fn list(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<RoleListQuery>,
) -> maud::Markup {
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
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `select`.
pub async fn select(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<RoleListQuery>,
) -> maud::Markup {
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
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `detail`.
pub async fn detail(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(role) = RoleEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Redirect::to("/users/roles/").into_response();
    };
    let page = RoleDetailPage {
        id: role.id,
        name: role.name,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `create_get`.
pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = RoleCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        name: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

use crate::plugins::users::forms::RoleForm;

/// HTTP handler: `create_post`.
pub async fn create_post(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<RoleForm>,
) -> Response {
    let now = Utc::now();
    let model = role::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(form.name.clone()),
    };
    match model.insert(&state.db).await {
        Ok(role) => respond_create_modal_done::<RoleCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &UsersRolesDetailRouteTag::new(role.id).url(),
        ),
        Err(e) => {
            let page = RoleCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                name: form.name,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

/// HTTP handler: `edit_get`.
pub async fn edit_get(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(role) = RoleEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Redirect::to("/users/roles/").into_response();
    };
    let page = RoleEditModalPage {
        id: role.id,
        form_name: q.form_name(),
        name: role.name,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `edit_post`.
pub async fn edit_post(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<RoleForm>,
) -> Response {
    let Some(role) = RoleEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Redirect::to("/users/roles/").into_response();
    };
    let mut am: role::ActiveModel = role.into();
    am.name = Set(form.name.clone());
    am.updated_at = Set(Some(Utc::now()));
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<RoleEditModalKey>(
            &htmx,
            &UsersRolesDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = RoleEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

/// HTTP handler: `delete_get`.
pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: RoleDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this role?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_users.RoleDeleteForm".into()),
        id,
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `delete_post`.
pub async fn delete_post(
    Cap(state): Cap<UsersState>,
    RequireStaff(_ctx): RequireStaff,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let _ = RoleEntity::delete_by_id(id).exec(&state.db).await;
    htmx.redirect("/users/roles/")
}
