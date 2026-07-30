use axum::{
    Form,
    response::{IntoResponse, Response},
};
use frunk::{Generic, hlist};
use serde::Deserialize;

use crate::{
    components::{FoldSlots, SlotCapability, SlotCtx},
    http::Cap,
    plugins::{
        otp::{
            entities::OtpPreferences,
            preferences::{load_preferences, save_preferences},
            state::OtpState,
            templates::{OtpPreferencesPage, OtpPreferencesPageTag},
        },
        users::middleware::RequireSuperuser,
    },
    template::{RenderAppPane, TemplateCapability, TemplateOf},
    traits::get::GetByTag,
    web::{Htmx, html_page_or_app_layout},
};

#[derive(Deserialize, Default)]
pub struct PreferencesForm {
    #[serde(rename = "Msg91AuthKey", default)]
    pub msg91_auth_key: String,
    #[serde(rename = "SmsOtpTemplateId", default)]
    pub sms_otp_template_id: String,
    #[serde(rename = "OtpTemplateId", default)]
    pub otp_template_id: String,
    #[serde(rename = "SmsOtpFieldName", default)]
    pub sms_otp_field_name: String,
    #[serde(rename = "SmsOtpExtraFields", default)]
    pub sms_otp_extra_fields: String,
    #[serde(rename = "EmailOtpTemplateString", default)]
    pub email_otp_template_string: String,
    #[serde(rename = "SmtpHost", default)]
    pub smtp_host: String,
    #[serde(rename = "SmtpPort", default)]
    pub smtp_port: String,
    #[serde(rename = "SmtpUsername", default)]
    pub smtp_username: String,
    #[serde(rename = "SmtpPassword", default)]
    pub smtp_password: String,
    #[serde(rename = "SmtpFrom", default)]
    pub smtp_from: String,
}

fn prefs_hlist(
    prefs: OtpPreferences,
    error: String,
) -> frunk::HList![
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String
] {
    hlist![
        prefs.msg91_auth_key,
        prefs.sms_otp_template_id,
        prefs.otp_template_id,
        prefs.sms_otp_field_name,
        prefs.sms_otp_extra_fields,
        prefs.email_otp_template_string,
        prefs.smtp_host,
        prefs.smtp_port,
        prefs.smtp_username,
        prefs.smtp_password,
        prefs.smtp_from,
        error,
    ]
}

pub async fn get<Templates, Slots, Idx, P>(
    Cap(state): Cap<OtpState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireSuperuser(ctx): RequireSuperuser,
    htmx: Htmx,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<OtpPreferencesPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <OtpPreferencesPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let slot_ctx = SlotCtx {
        name: Some(ctx.user.name.clone()),
        role: Some(ctx.role.clone()),
        is_superuser: ctx.user.is_superuser,
    };
    let prefs = match load_preferences(&state.db).await {
        Ok(p) => p,
        Err(e) => {
            return html_page_or_app_layout::<P, Slots>(
                &htmx,
                prefs_hlist(
                    OtpPreferences {
                        id: 1,
                        created_at: None,
                        updated_at: None,
                        deleted_at: None,
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
                ),
                &slots,
                &slot_ctx,
            )
            .into_response();
        }
    };
    html_page_or_app_layout::<P, Slots>(
        &htmx,
        prefs_hlist(prefs, String::new()),
        &slots,
        &slot_ctx,
    )
    .into_response()
}

pub async fn post<Templates, Slots, Idx, P>(
    Cap(state): Cap<OtpState>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireSuperuser(ctx): RequireSuperuser,
    htmx: Htmx,
    Form(form): Form<PreferencesForm>,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<OtpPreferencesPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <OtpPreferencesPage as Generic>::Repr>
        + crate::template::RenderTemplate
        + RenderAppPane,
{
    let slot_ctx = SlotCtx {
        name: Some(ctx.user.name.clone()),
        role: Some(ctx.role.clone()),
        is_superuser: ctx.user.is_superuser,
    };

    let prefs = OtpPreferences {
        id: 1,
        created_at: None,
        updated_at: None,
        deleted_at: None,
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
        Err(e) => html_page_or_app_layout::<P, Slots>(
            &htmx,
            prefs_hlist(prefs, e.to_string()),
            &slots,
            &slot_ctx,
        )
        .into_response(),
    }
}
