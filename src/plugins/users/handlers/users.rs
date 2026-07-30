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
        auth,
        entities::{
            role::Entity as RoleEntity,
            user::{self, Entity as UserEntity},
        },
        keys::{UserDeleteModalKey, UserSelectTableKey, UserTableKey},
        middleware::RequireSuperuser,
        state::UsersState,
        templates::{
            ChangePasswordPage, ConfirmDeletePage, UserDetailPage, UserFormPage, UserListPage,
            UserRow, UserSelectPage, UsersChangePasswordPageTag, UsersConfirmDeletePageTag,
            UsersUserDetailPageTag, UsersUserFormPageTag, UsersUserListPageTag,
            UsersUserSelectPageTag,
        },
    },
    template::{RenderAppPane, TemplateCapability, TemplateOf},
    traits::get::GetByTag,
    web::{Htmx, html_page_or_app_layout, html_page_with_slots},
};

use super::self_profile::PasswordForm;

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

pub async fn list<Templates, Slots, Idx, P>(
    Cap(state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireSuperuser(ctx): RequireSuperuser,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<UserListQuery>,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersUserListPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <UserListPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
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
    html_page_with_slots::<P, Slots>(
        hlist![
            page.users,
            page.filter_name,
            page.filter_email,
            page.filter_phone,
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
    Query(q): Query<UserListQuery>,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersUserSelectPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <UserSelectPage as Generic>::Repr> + crate::template::RenderTemplate,
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
    html_page_with_slots::<P, Slots>(
        hlist![
            page.users,
            page.filter_name,
            page.filter_email,
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
    Templates: GetByTag<UsersUserDetailPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <UserDetailPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let Some(user) = UserEntity::find_by_id(id).one(&state.db).await.ok().flatten() else {
        return Redirect::to("/users/").into_response();
    };
    let role = auth::role_name_for_user(&state.db, &user)
        .await
        .unwrap_or_default();
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![
            user.id,
            user.name,
            user.email,
            user.phone,
            role,
            user.is_superuser,
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
    Cap(_state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireSuperuser(ctx): RequireSuperuser,
    htmx: Htmx,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersUserFormPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <UserFormPage as Generic>::Repr>
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
            0_i64,
            String::new(),
            String::new(),
        ],
        &slots,
        &SlotCtx {
            name: Some(ctx.user.name.clone()),
            role: Some(ctx.role.clone()),
            is_superuser: ctx.user.is_superuser,
        },
    )
}

#[derive(Deserialize)]
pub struct UserForm {
    #[serde(rename = "Name", alias = "name")]
    pub name: String,
    #[serde(rename = "Email", alias = "email")]
    pub email: String,
    #[serde(rename = "Phone", alias = "phone")]
    pub phone: String,
    #[serde(rename = "RoleID", alias = "role_id", alias = "RoleId")]
    pub role_id: i64,
}

pub async fn create_post<Templates, Slots, Idx, P>(
    Cap(state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireSuperuser(ctx): RequireSuperuser,
    htmx: Htmx,
    Form(form): Form<UserForm>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersUserFormPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <UserFormPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
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
        Ok(user) => htmx.redirect(&format!("/users/u/{}/", user.id)),
        Err(e) => html_page_or_app_layout::<P, Slots>(
            &htmx,
            hlist![
                0_i64,
                form.name,
                form.email,
                form.phone,
                form.role_id,
                role_display,
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
    Templates: GetByTag<UsersUserFormPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <UserFormPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let Some(user) = UserEntity::find_by_id(id).one(&state.db).await.ok().flatten() else {
        return Redirect::to("/users/").into_response();
    };
    let role_display = role_display(&state.db, user.role_id).await;
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![
            user.id,
            user.name,
            user.email,
            user.phone,
            user.role_id,
            role_display,
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
    Cap(state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireSuperuser(ctx): RequireSuperuser,
    htmx: Htmx,
    Path(id): Path<i64>,
    Form(form): Form<UserForm>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersUserFormPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <UserFormPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
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
        Ok(_) => htmx.redirect( &format!("/users/u/{id}")),
        Err(e) => {
            let role_display = role_display(&state.db, form.role_id).await;
            html_page_or_app_layout::<P, Slots>(
                &htmx,
                hlist![
                    id,
                    form.name,
                    form.email,
                    form.phone,
                    form.role_id,
                    role_display,
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
            UserDeleteModalKey::ID.to_string(),
            "Are you sure you want to delete this user?".into(),
            q.name
                .clone()
                .unwrap_or_else(|| "p_users.UserDeleteForm".into()),
            format!("/users/u/{id}/delete/"),
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
    if let Some(user) = UserEntity::find_by_id(id).one(&state.db).await.ok().flatten() {
        let mut am: user::ActiveModel = user.into();
        am.deleted_at = Set(Some(Utc::now()));
        let _ = am.update(&state.db).await;
    }
    htmx.redirect( "/users/")
}

pub async fn change_password_get<Templates, Slots, Idx, P>(
    Cap(state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireSuperuser(ctx): RequireSuperuser,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersChangePasswordPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <ChangePasswordPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let Some(user) = UserEntity::find_by_id(id).one(&state.db).await.ok().flatten() else {
        return Redirect::to("/users/").into_response();
    };
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![
            user.id,
            user.name,
            format!("/users/u/{id}/change-password"),
            String::new(),
            false,
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

pub async fn change_password_post<Templates, Slots, Idx, P>(
    Cap(state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireSuperuser(ctx): RequireSuperuser,
    htmx: Htmx,
    Path(id): Path<i64>,
    Form(form): Form<PasswordForm>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersChangePasswordPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <ChangePasswordPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let Some(user) = UserEntity::find_by_id(id).one(&state.db).await.ok().flatten() else {
        return Redirect::to("/users/").into_response();
    };
    let user_name = user.name.clone();
    if form.new_password != form.confirm_password {
        return html_page_or_app_layout::<P, Slots>(
            &htmx,
            hlist![
                id,
                user_name,
                format!("/users/u/{id}/change-password"),
                "Passwords do not match".into(),
                false,
            ],
            &slots,
            &SlotCtx {
                name: Some(ctx.user.name.clone()),
                role: Some(ctx.role.clone()),
                is_superuser: ctx.user.is_superuser,
            },
        )
        .into_response();
    }
    let am: user::ActiveModel = user.into();
    match auth::set_password(&state.db, am, &form.new_password).await {
        Ok(_) => htmx.redirect( &format!("/users/u/{id}")),
        Err(e) => html_page_or_app_layout::<P, Slots>(
            &htmx,
            hlist![
                id,
                user_name,
                format!("/users/u/{id}/change-password"),
                e.to_string(),
                false,
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
