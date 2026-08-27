use axum::{
    response::{IntoResponse, Response},
};

use crate::{
    html_form::HtmlFormBody,
    components::{SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::{
        otp::{
            entities::OtpPreferences,
            preferences::{load_preferences, save_preferences},
            state::OtpState,
            templates::OtpPreferencesPage,
        },
        users::middleware::RequireStaff,
    },
    web::{Htmx, html_built_page_or_app_layout},
};

use crate::plugins::otp::forms::PreferencesForm;

fn prefs_page(prefs: OtpPreferences, error: String) -> OtpPreferencesPage {
    OtpPreferencesPage {
        msg91_auth_key: prefs.msg91_auth_key,
        sms_otp_template_id: prefs.sms_otp_template_id,
        otp_template_id: prefs.otp_template_id,
        sms_otp_field_name: prefs.sms_otp_field_name,
        sms_otp_extra_fields: prefs.sms_otp_extra_fields,
        email_otp_template_string: prefs.email_otp_template_string,
        smtp_host: prefs.smtp_host,
        smtp_port: prefs.smtp_port,
        smtp_username: prefs.smtp_username,
        smtp_password: prefs.smtp_password,
        smtp_from: prefs.smtp_from,
        error,
    }
}

/// HTTP handler: `get`.
pub async fn get(
    Cap(state): Cap<OtpState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
) -> Response {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let prefs = match load_preferences(&state.db).await {
        Ok(p) => p,
        Err(e) => {
            let page = prefs_page(
                OtpPreferences {
                    id: 1,
                    created_at: None,
                    updated_at: None,
                    sms_otp_template_id: String::new(),
                    otp_template_id: String::new(),
                    msg91_auth_key: String::new(),
                    sms_otp_field_name: String::new(),
                    sms_otp_extra_fields: String::new(),
                    email_otp_template_string: String::new(),
                    smtp_host: String::new(),
                    smtp_port: String::new(),
                    smtp_username: String::new(),
                    smtp_password: String::new(),
                    smtp_from: String::new(),
                },
                e.to_string(),
            );
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response();
        }
    };
    let page = prefs_page(prefs, String::new());
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

/// HTTP handler: `post`.
pub async fn post(
    Cap(state): Cap<OtpState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<PreferencesForm>,
) -> Response {
    let slot_ctx = SlotCtx::from_auth(&ctx);

    let prefs = OtpPreferences {
        id: 1,
        created_at: None,
        updated_at: None,
        sms_otp_template_id: form.sms_otp_template_id,
        otp_template_id: form.otp_template_id,
        msg91_auth_key: form.msg91_auth_key,
        sms_otp_field_name: form.sms_otp_field_name,
        sms_otp_extra_fields: form.sms_otp_extra_fields,
        email_otp_template_string: form.email_otp_template_string,
        smtp_host: form.smtp_host,
        smtp_port: form.smtp_port,
        smtp_username: form.smtp_username,
        smtp_password: form.smtp_password,
        smtp_from: form.smtp_from,
    };

    match save_preferences(&state.db, prefs.clone()).await {
        Ok(_) => htmx.redirect("/otp/preferences"),
        Err(e) => {
            let page = prefs_page(prefs, e.to_string());
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
        }
    }
}
