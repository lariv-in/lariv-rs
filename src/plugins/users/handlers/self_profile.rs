use axum::{
    extract::Query,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set};

use crate::{
    html_form::HtmlFormBody,
    components::{SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::users::{
        auth,
        entities::user,
        forms::{PasswordForm, SelfEditForm},
        keys::SelfEditModalKey,
        middleware::RequireAuth,
        routes::{UsersSelfChangePasswordPostRouteTag, UsersSelfRouteTag},
        state::UsersState,
        templates::{ChangePasswordPage, SelfDetailPage, SelfEditModalPage},
    },
    web::{
        Htmx, html_built_page_or_app_layout, html_built_page_with_slots, respond_edit_modal_done,
    },
};

use super::users::ModalNameQuery;

/// HTTP handler: `detail`.
pub async fn detail(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> maud::Markup {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let page = SelfDetailPage {
        name: ctx.user.name.clone(),
        email: ctx.user.email.to_string(),
        phone: ctx.user.phone.to_string(),
        timezone: ctx.user.timezone.to_string(),
        role: ctx.role.clone(),
        is_superuser: ctx.user.is_superuser,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
}

/// HTTP handler: `edit_get`.
pub async fn edit_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let page = SelfEditModalPage {
        form_name: q.form_name(),
        name: ctx.user.name.clone(),
        email: ctx.user.email.to_string(),
        phone: ctx.user.phone.to_string(),
        timezone: ctx.user.timezone.to_string(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

/// HTTP handler: `edit_post`.
pub async fn edit_post(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<SelfEditForm>,
) -> Response {
    let mut am: user::ActiveModel = ctx.user.clone().into();
    am.name = Set(form.name.clone());
    am.email = Set(form.email.clone().into());
    am.phone = Set(form.phone.clone().into());
    am.timezone = Set(form.timezone.clone().into());
    am.updated_at = Set(Some(Utc::now()));
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<SelfEditModalKey>(&htmx, &UsersSelfRouteTag.url()),
        Err(e) => {
            let slot_ctx = SlotCtx::from_auth(&ctx);
            let page = SelfEditModalPage {
                form_name: q.form_name(),
                name: form.name,
                email: form.email,
                phone: form.phone,
                timezone: form.timezone,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &slot_ctx).into_response()
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
        user_id: 0,
        user_name: ctx.user.name.clone(),
        action: UsersSelfChangePasswordPostRouteTag.path(),
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
    HtmlFormBody(form): HtmlFormBody<PasswordForm>,
) -> Response {
    let user_name = ctx.user.name.clone();
    if form.new_password != form.confirm_password {
        let slot_ctx = SlotCtx::from_auth(&ctx);
        let page = ChangePasswordPage {
            user_id: 0,
            user_name,
            action: UsersSelfChangePasswordPostRouteTag.path(),
            error: "Passwords do not match".into(),
            is_self: true,
        };
        return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response();
    }
    let am: user::ActiveModel = ctx.user.clone().into();
    match auth::set_password(&state.db, am, &form.new_password).await {
        Ok(_) => htmx.redirect("/users/self/"),
        Err(e) => {
            let slot_ctx = SlotCtx::from_auth(&ctx);
            let page = ChangePasswordPage {
                user_id: 0,
                user_name,
                action: UsersSelfChangePasswordPostRouteTag.path(),
                error: e.to_string(),
                is_self: true,
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
        }
    }
}
