use axum::{
    Form,
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::users::{
        auth, seed,
        session::{self, clear_auth_cookie, is_secure_request, set_auth_cookie},
        state::UsersState,
        templates::{LoginPage, SignupPage, UnauthenticatedPage},
    },
    web::{Htmx, html_built_page_or_app_layout},
};

use crate::plugins::users::forms::{LoginForm, SignupForm};

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
    Form(form): Form<LoginForm>,
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
            html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default()).into_response()
        }
    }
}

/// HTTP handler: `signup_get`.
pub async fn signup_get(Cap(chrome): Cap<SharedChromeFolder>, htmx: Htmx) -> maud::Markup {
    let page = SignupPage {
        error: String::new(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default())
}

/// HTTP handler: `signup_post`.
pub async fn signup_post(
    Cap(state): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    htmx: Htmx,
    headers: HeaderMap,
    Form(form): Form<SignupForm>,
) -> Response {
    if form.terms_accepted.is_none() {
        let page = SignupPage {
            error: "You must accept the terms".into(),
        };
        return html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default())
            .into_response();
    }
    if form.password1 != form.password2 {
        let page = SignupPage {
            error: "Passwords do not match".into(),
        };
        return html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default())
            .into_response();
    }
    let role = match seed::ensure_unassigned_role(&state.db).await {
        Ok(r) => r,
        Err(e) => {
            let page = SignupPage {
                error: e.to_string(),
            };
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default())
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
            timezone: Some(form.timezone),
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
            Err(_) => {
                let page = SignupPage {
                    error: "Account created but session failed".into(),
                };
                html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default())
                    .into_response()
            }
        },
        Err(e) => {
            let page = SignupPage {
                error: e.to_string(),
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default()).into_response()
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
