use axum::{
    Form,
    extract::Query,
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use frunk::{Generic, hlist};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde::Deserialize;

use crate::{
    components::{FoldSlots, SlotCapability, SlotCtx},
    http::Cap,
    plugins::{
        otp::{
            otp::{self as otp_logic},
            state::OtpState,
            templates::{
                EmailOtpRequestPage, ForgotPasswordPage, OtpEmailRequestPageTag,
                OtpForgotPasswordPageTag, OtpPhoneRequestPageTag, OtpVerifyPage,
                OtpVerifyPageTag, PhoneOtpRequestPage,
            },
        },
        users::{
            auth,
            entities::user::{self, Entity as UserEntity},
            middleware::OptionalAuth,
            session::{is_secure_request, set_auth_cookie},
            state::UsersState,
        },
    },
    template::{RenderAppPane, TemplateCapability, TemplateOf},
    traits::get::GetByTag,
    web::{Htmx, html_page_or_app_layout},
};

#[derive(Deserialize)]
pub struct IdentifierForm {
    #[serde(rename = "Identifier", alias = "identifier")]
    pub identifier: String,
}

#[derive(Deserialize)]
pub struct VerifyForm {
    #[serde(rename = "Otp", alias = "otp")]
    pub otp: String,
    #[serde(rename = "NewPassword", alias = "new_password")]
    pub new_password: String,
    #[serde(rename = "NewPassword2", alias = "new_password2")]
    pub new_password2: String,
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

pub async fn forgot_get<Templates, Slots, Idx, P>(
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    htmx: Htmx,
) -> maud::Markup
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<OtpForgotPasswordPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <ForgotPasswordPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    html_page_or_app_layout::<P, Slots>(&htmx, hlist![], &slots, &SlotCtx::default())
}

pub async fn phone_get<Templates, Slots, Idx, P>(
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    OptionalAuth(auth): OptionalAuth,
    htmx: Htmx,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<OtpPhoneRequestPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <PhoneOtpRequestPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    if auth.is_some() {
        return htmx.redirect("/users/");
    }
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![String::new(), String::new()],
        &slots,
        &SlotCtx::default(),
    )
    .into_response()
}

pub async fn phone_post<Templates, Slots, Idx, P>(
    Cap(state): Cap<OtpState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    OptionalAuth(auth): OptionalAuth,
    htmx: Htmx,
    Form(form): Form<IdentifierForm>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<OtpPhoneRequestPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <PhoneOtpRequestPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    if auth.is_some() {
        return htmx.redirect("/users/");
    }

    let identifier = form.identifier.trim().to_string();
    let err_page = |msg: String| {
        html_page_or_app_layout::<P, Slots>(
            &htmx,
            hlist![identifier.clone(), msg],
            &slots,
            &SlotCtx::default(),
        )
        .into_response()
    };

    if identifier.is_empty() {
        return err_page("phone number is required".into());
    }

    let count = match UserEntity::find()
        .filter(user::Column::Phone.eq(&identifier))
        .filter(user::Column::DeletedAt.is_null())
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
            let url = format!(
                "/otp/verify?identifier={}",
                query_escape(&identifier)
            );
            htmx.redirect(&url)
        }
        Err(_) => err_page("failed to send OTP. please check configuration".into()),
    }
}

pub async fn email_get<Templates, Slots, Idx, P>(
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    OptionalAuth(auth): OptionalAuth,
    htmx: Htmx,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<OtpEmailRequestPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <EmailOtpRequestPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    if auth.is_some() {
        return htmx.redirect("/users/");
    }
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![String::new(), String::new()],
        &slots,
        &SlotCtx::default(),
    )
    .into_response()
}

pub async fn email_post<Templates, Slots, Idx, P>(
    Cap(state): Cap<OtpState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    OptionalAuth(auth): OptionalAuth,
    htmx: Htmx,
    Form(form): Form<IdentifierForm>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<OtpEmailRequestPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <EmailOtpRequestPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    if auth.is_some() {
        return htmx.redirect("/users/");
    }

    let identifier = form.identifier.trim().to_string();
    let err_page = |msg: String| {
        html_page_or_app_layout::<P, Slots>(
            &htmx,
            hlist![identifier.clone(), msg],
            &slots,
            &SlotCtx::default(),
        )
        .into_response()
    };

    if identifier.is_empty() {
        return err_page("email address is required".into());
    }

    let count = match UserEntity::find()
        .filter(user::Column::Email.eq(&identifier))
        .filter(user::Column::DeletedAt.is_null())
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
            let url = format!(
                "/otp/verify?identifier={}",
                query_escape(&identifier)
            );
            htmx.redirect(&url)
        }
        Err(_) => err_page("failed to send OTP. please check configuration".into()),
    }
}

pub async fn verify_get<Templates, Slots, Idx, P>(
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    Query(q): Query<IdentifierQuery>,
    htmx: Htmx,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<OtpVerifyPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <OtpVerifyPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let Some(identifier) = q.identifier.filter(|s| !s.is_empty()) else {
        return htmx.redirect("/users/login");
    };
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![
            identifier,
            String::new(),
            String::new(),
            String::new(),
            String::new()
        ],
        &slots,
        &SlotCtx::default(),
    )
    .into_response()
}

pub async fn verify_post<Templates, Slots, Idx, P>(
    Cap(state): Cap<OtpState>,
    Cap(users): Cap<UsersState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    Query(q): Query<IdentifierQuery>,
    htmx: Htmx,
    headers: HeaderMap,
    Form(form): Form<VerifyForm>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<OtpVerifyPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <OtpVerifyPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let Some(identifier) = q.identifier.filter(|s| !s.is_empty()) else {
        return htmx.redirect("/users/login");
    };

    let otp = form.otp.trim().to_string();
    let new_password = form.new_password.trim().to_string();
    let new_password2 = form.new_password2.trim().to_string();

    let mut otp_err = String::new();
    let mut pw_err = String::new();
    let mut pw2_err = String::new();

    if new_password.is_empty() {
        pw_err = "new password is required".into();
    }
    if new_password2.is_empty() {
        pw2_err = "please confirm your new password".into();
    } else if !new_password.is_empty() && new_password != new_password2 {
        pw2_err = "passwords do not match".into();
    }
    if otp.is_empty() {
        otp_err = "OTP is required".into();
    } else if otp.len() != 6 {
        otp_err = "OTP must be 6 digits".into();
    }

    let render_err = |otp_e: String, pw_e: String, pw2_e: String| {
        html_page_or_app_layout::<P, Slots>(
            &htmx,
            hlist![
                identifier.clone(),
                otp.clone(),
                otp_e,
                pw_e,
                pw2_e
            ],
            &slots,
            &SlotCtx::default(),
        )
        .into_response()
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
        .filter(user::Column::DeletedAt.is_null())
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

    let user = match auth::set_password(&state.db, user.into(), &new_password).await {
        Ok(u) => u,
        Err(_) => {
            return render_err(
                String::new(),
                "could not update password. please try again".into(),
                String::new(),
            );
        }
    };

    match auth::login_token(&user, &users.signing_key, &users.jwt_issuer) {
        Ok(token) => {
            let mut response = htmx.redirect("/users/login");
            set_auth_cookie(response.headers_mut(), &token, is_secure_request(&headers));
            response
        }
        Err(_) => htmx.redirect("/users/login"),
    }
}
