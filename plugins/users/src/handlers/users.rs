use axum::{
    extract::{Path, Query},
    http::{StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use serde::Deserialize;

use lariv_core::picker::respond_picker_select;
use lariv_core::template::RenderAppPane;
use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx, SwapKey};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use crate::{
        auth,
        entities::user::{self, Entity as UserEntity},
        keys::{
            UserCreateModalKey, UserDeleteModalKey, UserEditModalKey, UserSelectModalKey,
            UserSelectTableKey, UserTableKey,
        },
        middleware::{RequireAuth, can_change_user_password, can_set_superuser},
        routes::{UsersChangePasswordPostRouteTag, UsersDetailRouteTag, UsersListRouteTag},
        state::UsersState,
        templates::{
            ChangePasswordPage, ConfirmDeletePage, UserCreateModalPage, UserDetailPage,
            UserEditModalPage, UserListPage, UserRow, UserSelectPage,
        },
    };
use lariv_core::web::{
        Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done_fk, respond_edit_modal_done,
    };

use crate::forms::{PasswordForm, UserForm};

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
    pub page_size: QueryPageSize,
    #[serde(default)]
    pub target_input: Option<String>,
}

/// Modal opener query (`?name=…&refresh=table-id`). Case-sensitive vs filter `Name`.
pub use lariv_core::web::ModalFormQuery as ModalNameQuery;

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
    let mut query = UserEntity::find()
        .select_only()
        .column(user::Column::Id)
        .column(user::Column::Name)
        .column(user::Column::Email)
        .column(user::Column::Phone);
    let name = filter_name(q);
    let email = filter_email(q);
    let phone = filter_phone(q);
    if !name.is_empty() {
        query = query.filter(user::Column::Name.contains(&name));
    }
    if !email.is_empty() {
        query = lariv_core::db::trigram::apply_text_search(
            query,
            db.get_database_backend(),
            &[user::Column::Email],
            &email,
        );
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
        _ => query.order_by_desc(user::Column::Id),
    };

    let page = q.page.get();
    let page_size = q.page_size.get();
    let paginator = query
        .into_tuple::<(i64, String, Option<String>, Option<String>)>()
        .paginate(db, page_size as u64);
    let total = match paginator.num_items().await {
        Ok(n) => n,
        Err(e) => {
            tracing::error!(error = %e, "failed to count users");
            return ObjectList::from_page(Vec::new(), page, page_size, 0);
        }
    };
    let rows = match paginator.fetch_page((page as u64).saturating_sub(1)).await {
        Ok(rows) => rows
            .into_iter()
            .map(|(id, name, email, phone)| UserRow {
                id,
                name,
                email: email.unwrap_or_default(),
                phone: phone.unwrap_or_default(),
            })
            .collect(),
        Err(e) => {
            tracing::error!(error = %e, "failed to load users");
            Vec::new()
        }
    };
    ObjectList::from_page(rows, page, page_size, total)
}

fn role_title(name: &str) -> String {
    crate::role_registry::current_role_registry().title_of(name)
}

/// Non-superusers cannot grant or revoke the Superuser role.
fn assignable_role(
    actor: &crate::state::AuthContext,
    requested: &str,
    current: Option<&str>,
) -> Result<String, String> {
    if can_set_superuser(actor) {
        return Ok(requested.to_string());
    }
    let current_is = current.is_some_and(crate::roles::Superuser::matches);
    let requested_is = crate::roles::Superuser::matches(requested);
    if requested_is && !current_is {
        return Err("Only a superuser can assign the Superuser role.".into());
    }
    if current_is && !requested_is {
        return Err("Only a superuser can change the Superuser role.".into());
    }
    Ok(requested.to_string())
}

/// HTTP handler: `list`.
pub async fn list(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<UserListQuery>,
) -> maud::Markup {
    let users = load_users_page(&state.db, &q).await;
    let page = UserListPage {
        users,
        filter_name: filter_name(&q),
        filter_email: filter_email(&q),
        filter_phone: filter_phone(&q),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
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
    RequireAuth(ctx): RequireAuth,
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
        current_user_id: ctx.user.id,
        current_user_name: ctx.user.name.clone(),
        page_size: q.page_size.get(),
    };
    respond_picker_select::<UserSelectTableKey, UserSelectModalKey, _>(&htmx, &page)
}

/// HTTP handler: `detail`.
pub async fn detail(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let user = match UserEntity::find_by_id(id).one(&state.db).await {
        Ok(Some(user)) => user,
        Ok(None) => return Redirect::to(&UsersListRouteTag.url()).into_response(),
        Err(e) => {
            tracing::error!(error = %e, user_id = id, "failed to load user detail");
            return Redirect::to(&UsersListRouteTag.url()).into_response();
        }
    };
    let role = role_title(&user.role);
    let show_change_password = can_change_user_password(&ctx, id);
    let page = UserDetailPage {
        id: user.id,
        name: user.name,
        email: user.email.to_string(),
        phone: user.phone.to_string(),
        timezone: user.timezone.to_string(),
        role,
        show_change_password,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `create_get`.
pub async fn create_get(
    Cap(_state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = UserCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        email: String::new(),
        phone: String::new(),
        timezone: lariv_core::datetime::DEFAULT_TIMEZONE.to_string(),
        role: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `create_post`.
pub async fn create_post(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<UserForm>,
) -> Response {
    if let Err(err) = auth::require_role(&form.role) {
        let page = UserCreateModalPage {
            form_name: q.form_name(),
            refresh_table: q.refresh_table(),
            target_input: q.target_input(),
            name: form.name,
            email: form.email,
            phone: form.phone,
            timezone: form.timezone,
            role: form.role,
            error: err.to_string(),
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let role = match assignable_role(&ctx, &form.role, None) {
        Ok(role) => role,
        Err(err) => {
            let page = UserCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                name: form.name,
                email: form.email,
                phone: form.phone,
                timezone: form.timezone,
                role: form.role,
                error: err,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    match auth::create_user(
        &state.db,
        auth::CreateUser {
            name: form.name.clone(),
            email: form.email.clone(),
            phone: form.phone.clone(),
            plain_password: String::new(),
            role,
            timezone: Some(form.timezone.clone()),
        },
    )
    .await
    {
        Ok(user) => respond_create_modal_done_fk::<UserCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &UsersDetailRouteTag::new(user.id).url(),
            user.id,
            &user.name,
            &q.target_input(),
        ),
        Err(e) => {
            let page = UserCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                name: form.name,
                email: form.email,
                phone: form.phone,
                timezone: form.timezone,
                role: form.role,
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
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(user) = lariv_core::web::opt_or_log(
        UserEntity::find_by_id(id).one(&state.db).await,
        "find user by id",
    ) else {
        return Redirect::to(&UsersListRouteTag.url()).into_response();
    };
    let page = UserEditModalPage {
        id: user.id,
        form_name: q.form_name(),
        name: user.name,
        email: user.email.to_string(),
        phone: user.phone.to_string(),
        timezone: user.timezone.to_string(),
        role: user.role.clone(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `edit_post`.
pub async fn edit_post(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<UserForm>,
) -> Response {
    let Some(user) = lariv_core::web::opt_or_log(
        UserEntity::find_by_id(id).one(&state.db).await,
        "find user by id",
    ) else {
        return Redirect::to(&UsersListRouteTag.url()).into_response();
    };
    if let Err(err) = auth::require_role(&form.role) {
        let page = UserEditModalPage {
            id,
            form_name: q.form_name(),
            name: form.name,
            email: form.email,
            phone: form.phone,
            timezone: form.timezone,
            role: form.role,
            error: err.to_string(),
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let role = match assignable_role(&ctx, &form.role, Some(&user.role)) {
        Ok(role) => role,
        Err(err) => {
            let page = UserEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                email: form.email,
                phone: form.phone,
                timezone: form.timezone,
                role: form.role,
                error: err,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let mut am: user::ActiveModel = user.into();
    am.name = Set(form.name.clone());
    am.email = Set(form.email.clone().into());
    am.phone = Set(form.phone.clone().into());
    am.role = Set(role);
    am.timezone = Set(form.timezone.clone().into());
    am.updated_at = Set(Some(Utc::now()));
    match am.update(&state.db).await {
        Ok(_) => {
            respond_edit_modal_done::<UserEditModalKey>(&htmx, &UsersDetailRouteTag::new(id).url())
        }
        Err(e) => {
            let page = UserEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                email: form.email,
                phone: form.phone,
                timezone: form.timezone,
                role: form.role,
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
        modal_uid: UserDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this user?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_users.UserDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `delete_post`.
pub async fn delete_post(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match UserEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&UsersListRouteTag.url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete user");
            let page = ConfirmDeletePage {
                modal_uid: UserDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this user?".into(),
                form_name: "p_users.UserDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

/// HTTP handler: `change_password_get`.
pub async fn change_password_get(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(user) = lariv_core::web::opt_or_log(
        UserEntity::find_by_id(id).one(&state.db).await,
        "find user by id",
    ) else {
        return Redirect::to(&UsersListRouteTag.url()).into_response();
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
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    HtmlFormBody(form): HtmlFormBody<PasswordForm>,
) -> Response {
    let Some(user) = lariv_core::web::opt_or_log(
        UserEntity::find_by_id(id).one(&state.db).await,
        "find user by id",
    ) else {
        return Redirect::to(&UsersListRouteTag.url()).into_response();
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
