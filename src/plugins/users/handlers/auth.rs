use axum::{
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};

use crate::{
    html_form::HtmlFormBody,
    components::{SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::users::{
        auth,
        session::{self, clear_auth_cookie, is_secure_request, set_auth_cookie},
        state::UsersState,
        templates::{LoginPage, UnauthenticatedPage},
    },
    web::{Htmx, html_built_page_or_app_layout},
};

use crate::plugins::users::forms::LoginForm;

/// HTTP handler: `login_get`.
pub async fn login_get(Cap(chrome): Cap<SharedChromeFolder>, htmx: Htmx) -> maud::Markup {
    let page = LoginPage {
        error: String::new(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default())
}

/// HTTP handler: `login_post`.
pub async fn login_post(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    htmx: Htmx,
    headers: HeaderMap,
    HtmlFormBody(form): HtmlFormBody<LoginForm>,
) -> Response {
    match auth::authenticate(&state.db, &form.email, &form.password).await {
        Ok(user) => match auth::login_token(&user, &state.signing_key, &state.jwt_issuer) {
            Ok(token) => {
                let mut response = htmx.redirect("/users/success");
                set_auth_cookie(response.headers_mut(), &token, is_secure_request(&headers));
                response
            }
            Err(_) => {
                let page = LoginPage {
                    error: "Could not create session".into(),
                };
                html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default())
                    .into_response()
            }
        },
        Err(_) => {
            let page = LoginPage {
                error: "Invalid email or password".into(),
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default())
                .into_response()
        }
    }
}

/// HTTP handler: `logout`.
pub async fn logout(htmx: Htmx, headers: HeaderMap) -> Response {
    let mut response = htmx.redirect("/users/login");
    clear_auth_cookie(response.headers_mut(), is_secure_request(&headers));
    let _ = session::AUTH_COOKIE;
    response
}

/// HTTP handler: `unauthenticated`.
pub async fn unauthenticated(Cap(chrome): Cap<SharedChromeFolder>, htmx: Htmx) -> maud::Markup {
    let page = UnauthenticatedPage {};
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default())
}

/// Post-login landing → dashboard.
pub async fn login_success() -> Redirect {
    Redirect::to("/dashboard/")
}
