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
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    http::{Cap},
    plugins::users::{
        auth,
        entities::{
            role::Entity as RoleEntity,
            user::{self, Entity as UserEntity},
        },
        keys::{
            UserCreateModalKey, UserDeleteModalKey, UserSelectModalKey, UserSelectTableKey,
            UserTableKey,
        },
        middleware::{RequireStaff, can_change_user_password},
        routes::{UsersChangePasswordPostRouteTag, UsersDetailRouteTag},
        state::UsersState,
        templates::{
            ChangePasswordPage, ConfirmDeletePage, UserCreateModalPage, UserDetailPage,
            UserFormPage, UserListPage, UserRow, UserSelectPage,
        },
    },
    web::{
        Htmx, QueryPage, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done,
    },
};
use crate::picker::respond_picker_select;
use crate::template::RenderAppPane;

use crate::plugins::users::forms::{PasswordForm, UserForm};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

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
    pub page: QueryPage,
    #[serde(default)]
    pub target_input: Option<String>,
}

/// Modal opener query (`?name=…&refresh=table-id`). Case-sensitive vs filter `Name`.
pub use crate::web::ModalFormQuery as ModalNameQuery;

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
    let mut query = UserEntity::find();
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

    let page = q.page.get();
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

/// HTTP handler: `list`.
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
    RequireStaff(_ctx): RequireStaff,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<UserListQuery>,
) -> maud::Markup {
    let users = load_users_page(&state.db, &q).await;
    let page = UserSelectPage {
        users,
        filter_name: filter_name(&q),
        filter_email: filter_email(&q),
        target_input: q.target_input.clone().unwrap_or_else(|| "UserID".into()),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    respond_picker_select::<UserSelectTableKey, UserSelectModalKey, _>(&htmx, &page)
}

/// HTTP handler: `detail`.
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
        timezone: user.timezone,
        role,
        user_is_superuser: user.is_superuser,
        show_change_password,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `create_get`.
pub async fn create_get(
    Cap(_state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = UserCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        name: String::new(),
        email: String::new(),
        phone: String::new(),
        timezone: crate::datetime::DEFAULT_TIMEZONE.to_string(),
        role_id: 0,
        role_display: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `create_post`.
pub async fn create_post(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<UserForm>,
) -> Response {
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
            timezone: Some(form.timezone.clone()),
        },
    )
    .await
    {
        Ok(user) => respond_create_modal_done::<UserCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &UsersDetailRouteTag::new(user.id).url(),
        ),
        Err(e) => {
            let page = UserCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                name: form.name,
                email: form.email,
                phone: form.phone,
                timezone: form.timezone,
                role_id: form.role_id,
                role_display,
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
        timezone: user.timezone,
        role_id: user.role_id,
        role_display,
        error: String::new(),
        show_change_password,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `edit_post`.
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
    am.timezone = Set(form.timezone.clone());
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
                timezone: form.timezone,
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

/// HTTP handler: `delete_get`.
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

/// HTTP handler: `delete_post`.
pub async fn delete_post(
    Cap(state): Cap<UsersState>,
    RequireStaff(_ctx): RequireStaff,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let _ = UserEntity::delete_by_id(id).exec(&state.db).await;
    htmx.redirect( "/users/")
}

/// HTTP handler: `change_password_get`.
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

/// HTTP handler: `change_password_post`.
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
