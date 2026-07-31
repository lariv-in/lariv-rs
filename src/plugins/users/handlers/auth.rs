use axum::{
    Form,
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use frunk::{Generic, hlist};

use crate::{
    components::{FoldSlots, SlotCapability, SlotCtx},
    http::Cap,
    plugins::users::{
        auth, seed,
        session::{self, clear_auth_cookie, is_secure_request, set_auth_cookie},
        state::UsersState,
        templates::{
            LoginPage, SignupPage, UnauthenticatedPage, UsersLoginPageTag, UsersSignupPageTag,
            UsersUnauthenticatedPageTag,
        },
    },
    template::{RenderAppPane, TemplateCapability, TemplateOf},
    traits::get::GetByTag,
    web::{Htmx, html_page_or_app_layout},
};

use crate::plugins::users::forms::{LoginForm, SignupForm};

pub async fn login_get<Templates, Slots, Idx, P>(
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    htmx: Htmx,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersLoginPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <LoginPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![String::new()],
        &slots,
        &SlotCtx::default(),
    )
}

pub async fn login_post<Templates, Slots, Idx, P>(
    Cap(state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    htmx: Htmx,
    headers: HeaderMap,
    Form(form): Form<LoginForm>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersLoginPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <LoginPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    match auth::authenticate(&state.db, &form.email, &form.password).await {
        Ok(user) => match auth::login_token(&user, &state.signing_key, &state.jwt_issuer) {
            Ok(token) => {
                let mut response = htmx.redirect("/users/success");
                set_auth_cookie(response.headers_mut(), &token, is_secure_request(&headers));
                response
            }
            Err(_) => html_page_or_app_layout::<P, Slots>(
                &htmx,
                hlist!["Could not create session".into()],
                &slots,
                &SlotCtx::default(),
            )
            .into_response(),
        },
        Err(_) => html_page_or_app_layout::<P, Slots>(
            &htmx,
            hlist!["Invalid email or password".into()],
            &slots,
            &SlotCtx::default(),
        )
        .into_response(),
    }
}

pub async fn signup_get<Templates, Slots, Idx, P>(
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    htmx: Htmx,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersSignupPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <SignupPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![String::new()],
        &slots,
        &SlotCtx::default(),
    )
}

pub async fn signup_post<Templates, Slots, Idx, P>(
    Cap(state): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    htmx: Htmx,
    headers: HeaderMap,
    Form(form): Form<SignupForm>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersSignupPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <SignupPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    if form.terms_accepted.is_none() {
        return html_page_or_app_layout::<P, Slots>(
            &htmx,
            hlist!["You must accept the terms".into()],
            &slots,
            &SlotCtx::default(),
        )
        .into_response();
    }
    if form.password1 != form.password2 {
        return html_page_or_app_layout::<P, Slots>(
            &htmx,
            hlist!["Passwords do not match".into()],
            &slots,
            &SlotCtx::default(),
        )
        .into_response();
    }
    let role = match seed::ensure_unassigned_role(&state.db).await {
        Ok(r) => r,
        Err(e) => {
            return html_page_or_app_layout::<P, Slots>(
                &htmx,
                hlist![e.to_string()],
                &slots,
                &SlotCtx::default(),
            )
            .into_response();
        }
    };
    match auth::create_user(
        &state.db,
        auth::CreateUser {
            name: form.name,
            email: form.email,
            phone: form.phone,
            plain_password: form.password1,
            role_id: role.id,
            is_superuser: false,
            timezone: None,
        },
    )
    .await
    {
        Ok(user) => match auth::login_token(&user, &state.signing_key, &state.jwt_issuer) {
            Ok(token) => {
                let mut response = htmx.redirect("/users/success");
                set_auth_cookie(response.headers_mut(), &token, is_secure_request(&headers));
                response
            }
            Err(_) => html_page_or_app_layout::<P, Slots>(
                &htmx,
                hlist!["Account created but session failed".into()],
                &slots,
                &SlotCtx::default(),
            )
            .into_response(),
        },
        Err(e) => html_page_or_app_layout::<P, Slots>(
            &htmx,
            hlist![e.to_string()],
            &slots,
            &SlotCtx::default(),
        )
        .into_response(),
    }
}

pub async fn logout(htmx: Htmx, headers: HeaderMap) -> Response {
    let mut response = htmx.redirect("/users/login");
    clear_auth_cookie(response.headers_mut(), is_secure_request(&headers));
    let _ = session::AUTH_COOKIE;
    response
}

pub async fn unauthenticated<Templates, Slots, Idx, P>(
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    htmx: Htmx,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<UsersUnauthenticatedPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <UnauthenticatedPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    html_page_or_app_layout::<P, Slots>(&htmx, hlist![], &slots, &SlotCtx::default())
}

// Post-login landing (Go `LoginSuccessRoute`) → dashboard.
pub async fn login_success() -> Redirect {
    Redirect::to("/dashboard/")
}
