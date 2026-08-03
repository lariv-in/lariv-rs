use axum::{
    Form,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::users::{
        auth,
        entities::user::{self, Entity as UserEntity},
        forms::{PasswordForm, SelfEditForm},
        middleware::RequireAuth,
        state::UsersState,
        templates::{ChangePasswordPage, SelfDetailPage, SelfEditPage},
    },
    web::{Htmx, html_built_page_or_app_layout},
};

/// HTTP handler: `detail`.
pub async fn detail(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> maud::Markup {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let page = SelfDetailPage {
        name: ctx.user.name.clone(),
        email: ctx.user.email.clone(),
        phone: ctx.user.phone.clone(),
        role: ctx.role.clone(),
        is_superuser: ctx.user.is_superuser,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
}

/// HTTP handler: `edit_get`.
pub async fn edit_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> maud::Markup {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let page = SelfEditPage {
        name: ctx.user.name.clone(),
        email: ctx.user.email.clone(),
        phone: ctx.user.phone.clone(),
        error: String::new(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
}

/// HTTP handler: `edit_post`.
pub async fn edit_post(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Form(form): Form<SelfEditForm>,
) -> Response {
    let mut am: user::ActiveModel = ctx.user.clone().into();
    am.name = Set(form.name.clone());
    am.email = Set(form.email.clone());
    am.phone = Set(form.phone.clone());
    am.updated_at = Set(Some(Utc::now()));
    match am.update(&state.db).await {
        Ok(_) => htmx.redirect("/users/self/"),
        Err(e) => {
            let slot_ctx = SlotCtx::from_auth(&ctx);
            let page = SelfEditPage {
                name: form.name,
                email: form.email,
                phone: form.phone,
                error: e.to_string(),
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
        }
    }
}

/// HTTP handler: `change_password_get`.
pub async fn change_password_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> maud::Markup {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let page = ChangePasswordPage {
        user_id: ctx.user.id,
        user_name: ctx.user.name.clone(),
        action: "/users/self/change-password/".into(),
        error: String::new(),
        is_self: true,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
}

/// HTTP handler: `change_password_post`.
pub async fn change_password_post(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Form(form): Form<PasswordForm>,
) -> Response {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if form.new_password != form.confirm_password {
        let page = ChangePasswordPage {
            user_id: ctx.user.id,
            user_name: ctx.user.name.clone(),
            action: "/users/self/change-password/".into(),
            error: "Passwords do not match".into(),
            is_self: true,
        };
        return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response();
    }
    let user_id = ctx.user.id;
    let user_name = ctx.user.name.clone();
    let am: user::ActiveModel = ctx.user.into();
    match auth::set_password(&state.db, am, &form.new_password).await {
        Ok(_) => htmx.redirect("/users/self/"),
        Err(e) => {
            let page = ChangePasswordPage {
                user_id,
                user_name,
                action: "/users/self/change-password/".into(),
                error: e.to_string(),
                is_self: true,
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
        }
    }
}

#[allow(dead_code)]
async fn _load_user(state: &UsersState, id: i64) -> Option<user::Model> {
    UserEntity::find_by_id(id).one(&state.db).await.ok()?
}
