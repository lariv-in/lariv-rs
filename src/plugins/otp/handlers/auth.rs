use axum::{
    extract::Query,
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde::Deserialize;

use crate::{
    html_form::HtmlFormBody,
    components::{SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::{
        otp::{
            otp::{self as otp_logic},
            state::OtpState,
            templates::{
                EmailOtpRequestPage, ForgotPasswordPage, LoginPageWithForgot, OtpVerifyPage,
                PhoneOtpRequestPage,
            },
        },
        users::{
            auth,
            entities::user::{self, Entity as UserEntity},
            forms::LoginForm,
            middleware::OptionalAuth,
            session::{is_secure_request, set_auth_cookie},
            state::UsersState,
        },
    },
    web::{Htmx, html_built_page_or_app_layout},
};

use crate::plugins::otp::forms::{IdentifierForm, VerifyForm};

/// HTTP handler: `login_get` (login page with forgot-password CTA).
pub async fn login_get(Cap(chrome): Cap<SharedChromeFolder>, htmx: Htmx) -> maud::Markup {
    let page = LoginPageWithForgot {
        error: String::new(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default())
}

/// HTTP handler: `login_post` (re-renders forgot-password-aware login on error).
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
                let page = LoginPageWithForgot {
                    error: "Could not create session".into(),
                };
                html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default())
                    .into_response()
            }
        },
        Err(_) => {
            let page = LoginPageWithForgot {
                error: "Invalid email or password".into(),
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default())
                .into_response()
        }
    }
}

#[derive(Deserialize, Default)]
pub struct IdentifierQuery {
    #[serde(default)]
    pub identifier: Option<String>,
}

fn query_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(*b));
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// HTTP handler: `forgot_get`.
pub async fn forgot_get(Cap(chrome): Cap<SharedChromeFolder>, htmx: Htmx) -> maud::Markup {
    let page = ForgotPasswordPage {};
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default())
}

/// HTTP handler: `phone_get`.
pub async fn phone_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    OptionalAuth(auth): OptionalAuth,
    htmx: Htmx,
) -> Response {
    if auth.is_some() {
        return htmx.redirect("/users/");
    }
    let page = PhoneOtpRequestPage {
        identifier: String::new(),
        error: String::new(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default()).into_response()
}

/// HTTP handler: `phone_post`.
pub async fn phone_post(
    Cap(state): Cap<OtpState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    OptionalAuth(auth): OptionalAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<IdentifierForm>,
) -> Response {
    if auth.is_some() {
        return htmx.redirect("/users/");
    }

    let identifier = form.identifier.trim().to_string();
    let err_page = |msg: String| {
        let page = PhoneOtpRequestPage {
            identifier: identifier.clone(),
            error: msg,
        };
        html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default()).into_response()
    };

    if identifier.is_empty() {
        return err_page("phone number is required".into());
    }

    let count = match UserEntity::find()
        .filter(user::Column::Phone.eq(&identifier))
        .count(&state.db)
        .await
    {
        Ok(c) => c,
        Err(_) => return err_page("internal error. please try again later".into()),
    };
    if count == 0 {
        return err_page("no user found with this phone number".into());
    }

    match otp_logic::send_sms_otp(&state.db, &state.cache, &identifier).await {
        Ok(()) => {
            let url = format!("/otp/verify?identifier={}", query_escape(&identifier));
            htmx.redirect(&url)
        }
        Err(_) => err_page("failed to send OTP. please check configuration".into()),
    }
}

/// HTTP handler: `email_get`.
pub async fn email_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    OptionalAuth(auth): OptionalAuth,
    htmx: Htmx,
) -> Response {
    if auth.is_some() {
        return htmx.redirect("/users/");
    }
    let page = EmailOtpRequestPage {
        identifier: String::new(),
        error: String::new(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default()).into_response()
}

/// HTTP handler: `email_post`.
pub async fn email_post(
    Cap(state): Cap<OtpState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    OptionalAuth(auth): OptionalAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<IdentifierForm>,
) -> Response {
    if auth.is_some() {
        return htmx.redirect("/users/");
    }

    let identifier = form.identifier.trim().to_string();
    let err_page = |msg: String| {
        let page = EmailOtpRequestPage {
            identifier: identifier.clone(),
            error: msg,
        };
        html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default()).into_response()
    };

    if identifier.is_empty() {
        return err_page("email address is required".into());
    }

    let count = match UserEntity::find()
        .filter(user::Column::Email.eq(&identifier))
        .count(&state.db)
        .await
    {
        Ok(c) => c,
        Err(_) => return err_page("internal error. please try again later".into()),
    };
    if count == 0 {
        return err_page("no user found with this email".into());
    }

    match otp_logic::send_email_otp(&state.db, &state.cache, &identifier).await {
        Ok(()) => {
            let url = format!("/otp/verify?identifier={}", query_escape(&identifier));
            htmx.redirect(&url)
        }
        Err(_) => err_page("failed to send OTP. please check configuration".into()),
    }
}

/// HTTP handler: `verify_get`.
pub async fn verify_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    Query(q): Query<IdentifierQuery>,
    htmx: Htmx,
) -> Response {
    let Some(identifier) = q.identifier.filter(|s| !s.is_empty()) else {
        return htmx.redirect("/users/login");
    };
    let page = OtpVerifyPage {
        identifier,
        otp: String::new(),
        otp_error: String::new(),
        password_error: String::new(),
        password2_error: String::new(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default()).into_response()
}

/// HTTP handler: `verify_post`.
pub async fn verify_post(
    Cap(state): Cap<OtpState>,
    Cap(users): Cap<UsersState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    Query(q): Query<IdentifierQuery>,
    htmx: Htmx,
    headers: HeaderMap,
    HtmlFormBody(form): HtmlFormBody<VerifyForm>,
) -> Response {
    let Some(identifier) = q.identifier.filter(|s| !s.is_empty()) else {
        return htmx.redirect("/users/login");
    };

    let otp = form.otp.trim().to_string();
    let new_password = form.new_password.trim().to_string();
    let new_password2 = form.new_password2.trim().to_string();

    let mut otp_err = String::new();
    let mut pw_err = String::new();
    let mut pw2_err = String::new();

    if !new_password.is_empty() || !new_password2.is_empty() {
        if new_password.is_empty() {
            pw_err = "new password is required".into();
        }
        if new_password2.is_empty() {
            pw2_err = "please confirm your new password".into();
        } else if !new_password.is_empty() && new_password != new_password2 {
            pw2_err = "passwords do not match".into();
        }
    }
    if otp.is_empty() {
        otp_err = "OTP is required".into();
    } else if otp.len() != 6 {
        otp_err = "OTP must be 6 digits".into();
    }

    let render_err = |otp_e: String, pw_e: String, pw2_e: String| {
        let page = OtpVerifyPage {
            identifier: identifier.clone(),
            otp: otp.clone(),
            otp_error: otp_e,
            password_error: pw_e,
            password2_error: pw2_e,
        };
        html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default()).into_response()
    };

    if !otp_err.is_empty() || !pw_err.is_empty() || !pw2_err.is_empty() {
        return render_err(otp_err, pw_err, pw2_err);
    }

    if !otp_logic::verify_otp(&state.cache, &identifier, &otp) {
        return render_err("invalid OTP".into(), String::new(), String::new());
    }

    let user = match UserEntity::find()
        .filter(
            user::Column::Phone
                .eq(&identifier)
                .or(user::Column::Email.eq(&identifier)),
        )
        .one(&state.db)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return render_err("user not found".into(), String::new(), String::new());
        }
        Err(_) => {
            return render_err(
                "internal error. please try again later".into(),
                String::new(),
                String::new(),
            );
        }
    };

    let user = if new_password.is_empty() {
        user
    } else {
        match auth::set_password(&state.db, user.into(), &new_password).await {
            Ok(u) => u,
            Err(_) => {
                return render_err(
                    String::new(),
                    "could not update password. please try again".into(),
                    String::new(),
                );
            }
        }
    };

    match auth::login_token(&user, &users.signing_key, &users.jwt_issuer) {
        Ok(token) => {
            let mut response = htmx.redirect("/users/success");
            set_auth_cookie(response.headers_mut(), &token, is_secure_request(&headers));
            response
        }
        Err(_) => {
            let page = LoginPageWithForgot {
                error: "Could not create session".into(),
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default())
                .into_response()
        }
    }
}
