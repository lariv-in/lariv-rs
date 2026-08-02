use axum::{
    Form,
    extract::{Path, Query},
    http::{StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use serde::Deserialize;

use crate::{
    components::{ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    http::{Cap},
    plugins::users::{
        auth,
        entities::{
            role::Entity as RoleEntity,
            user::{self, Entity as UserEntity},
        },
        keys::{UserDeleteModalKey, UserSelectTableKey, UserTableKey},
        middleware::{RequireStaff, can_change_user_password},
        routes::{UsersChangePasswordPostRouteTag, UsersDetailRouteTag},
        state::UsersState,
        templates::{
            ChangePasswordPage, ConfirmDeletePage, UserDetailPage, UserFormPage, UserListPage,
            UserRow, UserSelectPage,
        },
    },
    web::{Htmx, html_built_page_or_app_layout, html_built_page_with_slots},
};
use crate::template::RenderAppPane;

use crate::plugins::users::forms::{PasswordForm, UserForm};

const PAGE_SIZE: u32 = 12;

#[derive(Debug, Deserialize, Default)]
pub struct UserListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default, rename = "Email", alias = "email")]
    pub email: Option<String>,
    #[serde(default, rename = "Phone", alias = "phone")]
    pub phone: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub target_input: Option<String>,
}

// Modal opener query (`?name=p_users.UserCreateForm`). Case-sensitive vs filter `Name`.
#[derive(Debug, Deserialize, Default)]
pub struct ModalNameQuery {
    #[serde(default)]
    pub name: Option<String>,
}

fn filter_name(q: &UserListQuery) -> String {
    q.name.clone().unwrap_or_default()
}

fn filter_email(q: &UserListQuery) -> String {
    q.email.clone().unwrap_or_default()
}

fn filter_phone(q: &UserListQuery) -> String {
    q.phone.clone().unwrap_or_default()
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

async fn load_users_page(
    db: &sea_orm::DatabaseConnection,
    q: &UserListQuery,
) -> ObjectList<UserRow> {
    let mut query = UserEntity::find().filter(user::Column::DeletedAt.is_null());
    let name = filter_name(q);
    let email = filter_email(q);
    let phone = filter_phone(q);
    if !name.is_empty() {
        query = query.filter(user::Column::Name.contains(&name));
    }
    if !email.is_empty() {
        query = query.filter(user::Column::Email.contains(&email));
    }
    if !phone.is_empty() {
        query = query.filter(user::Column::Phone.contains(&phone));
    }
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(user::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(user::Column::Name)
        }
        s if s.eq_ignore_ascii_case("Email DESC") => query.order_by_desc(user::Column::Email),
        s if s.eq_ignore_ascii_case("Email ASC") || s.eq_ignore_ascii_case("Email") => {
            query.order_by_asc(user::Column::Email)
        }
        s if s.eq_ignore_ascii_case("Phone DESC") => query.order_by_desc(user::Column::Phone),
        s if s.eq_ignore_ascii_case("Phone ASC") || s.eq_ignore_ascii_case("Phone") => {
            query.order_by_asc(user::Column::Phone)
        }
        _ => query.order_by_asc(user::Column::Id),
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
        .map(|u| UserRow {
            id: u.id,
            name: u.name,
            email: u.email,
            phone: u.phone,
        })
        .collect();
    ObjectList::from_page(rows, page, PAGE_SIZE, total)
}

async fn role_display(db: &sea_orm::DatabaseConnection, role_id: i64) -> String {
    RoleEntity::find_by_id(role_id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|r| r.name)
        .unwrap_or_default()
}

pub async fn list(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<UserListQuery>,
) -> maud::Markup
{
    let users = load_users_page(&state.db, &q).await;
    let page = UserListPage {
        users,
        filter_name: filter_name(&q),
        filter_email: filter_email(&q),
        filter_phone: filter_phone(&q),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    if htmx.targets::<UserTableKey>() {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main();
    }
    if htmx.wants_app_layout() {
        return page.render_pane();
    }
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn select(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<UserListQuery>,
) -> maud::Markup
{
    let users = load_users_page(&state.db, &q).await;
    let page = UserSelectPage {
        users,
        filter_name: filter_name(&q),
        filter_email: filter_email(&q),
        target_input: q.target_input.clone().unwrap_or_else(|| "UserID".into()),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    if htmx.targets::<UserSelectTableKey>() {
        return page.render_table();
    }
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn detail(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
{
    let Some(user) = UserEntity::find_by_id(id).one(&state.db).await.ok().flatten() else {
        return Redirect::to("/users/").into_response();
    };
    let role = auth::role_name_for_user(&state.db, &user)
        .await
        .unwrap_or_default();
    let show_change_password = can_change_user_password(&ctx, id);
    let page = UserDetailPage {
        id: user.id,
        name: user.name,
        email: user.email,
        phone: user.phone,
        role,
        user_is_superuser: user.is_superuser,
        show_change_password,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(_state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
) -> maud::Markup {
    let page = UserFormPage {
        id: 0,
        name: String::new(),
        email: String::new(),
        phone: String::new(),
        role_id: 0,
        role_display: String::new(),
        error: String::new(),
        show_change_password: false,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    Form(form): Form<UserForm>,
) -> Response
{
    let role_display = role_display(&state.db, form.role_id).await;
    match auth::create_user(
        &state.db,
        auth::CreateUser {
            name: form.name.clone(),
            email: form.email.clone(),
            phone: form.phone.clone(),
            plain_password: String::new(),
            role_id: form.role_id,
            is_superuser: false,
            timezone: None,
        },
    )
    .await
    {
        Ok(user) => htmx.redirect(&UsersDetailRouteTag::new(user.id).url()),
        Err(e) => {
            let page = UserFormPage {
                id: 0,
                name: form.name,
                email: form.email,
                phone: form.phone,
                role_id: form.role_id,
                role_display,
                error: e.to_string(),
                show_change_password: false,
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response()
        }
    }
}

pub async fn edit_get(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
{
    let Some(user) = UserEntity::find_by_id(id).one(&state.db).await.ok().flatten() else {
        return Redirect::to("/users/").into_response();
    };
    let role_display = role_display(&state.db, user.role_id).await;
    let show_change_password = can_change_user_password(&ctx, id);
    let page = UserFormPage {
        id: user.id,
        name: user.name,
        email: user.email,
        phone: user.phone,
        role_id: user.role_id,
        role_display,
        error: String::new(),
        show_change_password,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    Path(id): Path<i64>,
    Form(form): Form<UserForm>,
) -> Response
{
    let Some(user) = UserEntity::find_by_id(id).one(&state.db).await.ok().flatten() else {
        return Redirect::to("/users/").into_response();
    };
    let mut am: user::ActiveModel = user.into();
    am.name = Set(form.name.clone());
    am.email = Set(form.email.clone());
    am.phone = Set(form.phone.clone());
    am.role_id = Set(form.role_id);
    am.updated_at = Set(Some(Utc::now()));
    match am.update(&state.db).await {
        Ok(_) => htmx.redirect(&UsersDetailRouteTag::new(id).url()),
        Err(e) => {
            let role_display = role_display(&state.db, form.role_id).await;
            let show_change_password = can_change_user_password(&ctx, id);
            let page = UserFormPage {
                id,
                name: form.name,
                email: form.email,
                phone: form.phone,
                role_id: form.role_id,
                role_display,
                error: e.to_string(),
                show_change_password,
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response()
        }
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: UserDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this user?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_users.UserDeleteForm".into()),
        id,
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<UsersState>,
    RequireStaff(_ctx): RequireStaff,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if let Some(user) = UserEntity::find_by_id(id).one(&state.db).await.ok().flatten() {
        let mut am: user::ActiveModel = user.into();
        am.deleted_at = Set(Some(Utc::now()));
        let _ = am.update(&state.db).await;
    }
    htmx.redirect( "/users/")
}

pub async fn change_password_get(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
{
    let Some(user) = UserEntity::find_by_id(id).one(&state.db).await.ok().flatten() else {
        return Redirect::to("/users/").into_response();
    };
    if !can_change_user_password(&ctx, id) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let page = ChangePasswordPage {
        user_id: user.id,
        user_name: user.name,
        action: UsersChangePasswordPostRouteTag::new(id).path(),
        error: String::new(),
        is_self: false,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn change_password_post(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    Path(id): Path<i64>,
    Form(form): Form<PasswordForm>,
) -> Response
{
    let Some(user) = UserEntity::find_by_id(id).one(&state.db).await.ok().flatten() else {
        return Redirect::to("/users/").into_response();
    };
    if !can_change_user_password(&ctx, id) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let user_name = user.name.clone();
    if form.new_password != form.confirm_password {
        let page = ChangePasswordPage {
            user_id: id,
            user_name: user_name.clone(),
            action: UsersChangePasswordPostRouteTag::new(id).path(),
            error: "Passwords do not match".into(),
            is_self: false,
        };
        return html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let am: user::ActiveModel = user.into();
    match auth::set_password(&state.db, am, &form.new_password).await {
        Ok(_) => htmx.redirect(&UsersDetailRouteTag::new(id).url()),
        Err(e) => {
            let page = ChangePasswordPage {
                user_id: id,
                user_name,
                action: UsersChangePasswordPostRouteTag::new(id).path(),
                error: e.to_string(),
                is_self: false,
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response()
        }
    }
}
