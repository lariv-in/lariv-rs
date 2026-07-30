use axum::{
    Form,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use frunk::{Generic, hlist};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde::Deserialize;

use crate::{
    components::{FoldSlots, SlotCapability, SlotCtx},
    http::Cap,
    plugins::users::{
        auth,
        entities::user::{self, Entity as UserEntity},
        middleware::RequireAuth,
        state::UsersState,
        templates::{
            ChangePasswordPage, SelfDetailPage, SelfEditPage, UsersChangePasswordPageTag,
            UsersSelfDetailPageTag, UsersSelfEditPageTag,
        },
    },
    template::{RenderAppPane, TemplateCapability, TemplateOf},
    traits::get::GetByTag,
    web::{Htmx, html_page_or_app_layout},
};

pub async fn detail<Templates, Slots, Idx, P>(
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersSelfDetailPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <SelfDetailPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let slot_ctx = SlotCtx {
        name: Some(ctx.user.name.clone()),
        role: Some(ctx.role.clone()),
        is_superuser: ctx.user.is_superuser,
    };
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![
            ctx.user.name,
            ctx.user.email,
            ctx.user.phone,
            ctx.role,
            ctx.user.is_superuser,
        ],
        &slots,
        &slot_ctx,
    )
}

pub async fn edit_get<Templates, Slots, Idx, P>(
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersSelfEditPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <SelfEditPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let slot_ctx = SlotCtx {
        name: Some(ctx.user.name.clone()),
        role: Some(ctx.role.clone()),
        is_superuser: ctx.user.is_superuser,
    };
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![
            ctx.user.name,
            ctx.user.email,
            ctx.user.phone,
            String::new(),
        ],
        &slots,
        &slot_ctx,
    )
}

#[derive(Deserialize)]
pub struct SelfEditForm {
    #[serde(rename = "Name", alias = "name")]
    pub name: String,
    #[serde(rename = "Email", alias = "email")]
    pub email: String,
    #[serde(rename = "Phone", alias = "phone")]
    pub phone: String,
}

pub async fn edit_post<Templates, Slots, Idx, P>(
    Cap(state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Form(form): Form<SelfEditForm>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersSelfEditPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <SelfEditPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let mut am: user::ActiveModel = ctx.user.clone().into();
    am.name = Set(form.name.clone());
    am.email = Set(form.email.clone());
    am.phone = Set(form.phone.clone());
    am.updated_at = Set(Some(Utc::now()));
    match am.update(&state.db).await {
        Ok(_) => htmx.redirect("/users/self/"),
        Err(e) => {
            let slot_ctx = SlotCtx {
                name: Some(ctx.user.name.clone()),
                role: Some(ctx.role.clone()),
                is_superuser: ctx.user.is_superuser,
            };
            html_page_or_app_layout::<P, Slots>(
                &htmx,
                hlist![form.name, form.email, form.phone, e.to_string()],
                &slots,
                &slot_ctx,
            )
            .into_response()
        }
    }
}

pub async fn change_password_get<Templates, Slots, Idx, P>(
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> maud::Markup
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
    let slot_ctx = SlotCtx {
        name: Some(ctx.user.name.clone()),
        role: Some(ctx.role.clone()),
        is_superuser: ctx.user.is_superuser,
    };
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![
            ctx.user.id,
            ctx.user.name,
            "/users/self/change-password/".into(),
            String::new(),
            true,
        ],
        &slots,
        &slot_ctx,
    )
}

#[derive(Deserialize)]
pub struct PasswordForm {
    pub new_password: String,
    pub confirm_password: String,
}

pub async fn change_password_post<Templates, Slots, Idx, P>(
    Cap(state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
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
    let slot_ctx = SlotCtx {
        name: Some(ctx.user.name.clone()),
        role: Some(ctx.role.clone()),
        is_superuser: ctx.user.is_superuser,
    };
    if form.new_password != form.confirm_password {
        return html_page_or_app_layout::<P, Slots>(
            &htmx,
            hlist![
                ctx.user.id,
                ctx.user.name.clone(),
                "/users/self/change-password/".into(),
                "Passwords do not match".into(),
                true,
            ],
            &slots,
            &slot_ctx,
        )
        .into_response();
    }
    let user_id = ctx.user.id;
    let user_name = ctx.user.name.clone();
    let am: user::ActiveModel = ctx.user.into();
    match auth::set_password(&state.db, am, &form.new_password).await {
        Ok(_) => htmx.redirect("/users/self/"),
        Err(e) => html_page_or_app_layout::<P, Slots>(
            &htmx,
            hlist![
                user_id,
                user_name,
                "/users/self/change-password/".into(),
                e.to_string(),
                true,
            ],
            &slots,
            &slot_ctx,
        )
        .into_response(),
    }
}

#[allow(dead_code)]
async fn _load_user(state: &UsersState, id: i64) -> Option<user::Model> {
    UserEntity::find_by_id(id).one(&state.db).await.ok()?
}
