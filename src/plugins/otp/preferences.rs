use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait,
};
use serde_json::Map;
use tracing::warn;

use super::entities::{
    otp_preferences::{self, Entity as OtpPreferencesEntity},
    OtpPreferences,
};
use super::error::OtpError;

/// Load singleton preferences row (`id = 1`), creating it if missing.
pub async fn load_preferences(db: &DatabaseConnection) -> Result<OtpPreferences, OtpError> {
    if let Some(prefs) = OtpPreferencesEntity::find_by_id(1).one(db).await? {
        return Ok(prefs);
    }

    let now = Utc::now();
    let model = otp_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        sms_otp_template_id: Set(String::new()),
        otp_template_id: Set(String::new()),
        msg91_auth_key: Set(String::new()),
        sms_otp_field_name: Set(String::new()),
        sms_otp_extra_fields: Set(String::new()),
        email_otp_template_string: Set(String::new()),
        smtp_host: Set(String::new()),
        smtp_port: Set(String::new()),
        smtp_username: Set(String::new()),
        smtp_password: Set(String::new()),
        smtp_from: Set(String::new()),
    };
    Ok(model.insert(db).await?)
}

/// Parse `sms_otp_extra_fields` JSON into a map (empty on error).
pub fn extra_fields(prefs: &OtpPreferences) -> Map<String, serde_json::Value> {
    if prefs.sms_otp_extra_fields.is_empty() {
        return Map::new();
    }
    match serde_json::from_str::<Map<String, serde_json::Value>>(&prefs.sms_otp_extra_fields) {
        Ok(m) => m,
        Err(err) => {
            warn!(
                error = %err,
                value = %prefs.sms_otp_extra_fields,
                "failed to unmarshal SmsOtpExtraFields JSON"
            );
            Map::new()
        }
    }
}

/// Persist preferences fields onto the singleton row.
pub async fn save_preferences(
    db: &DatabaseConnection,
    prefs: OtpPreferences,
) -> Result<OtpPreferences, OtpError> {
    let mut am: otp_preferences::ActiveModel = load_preferences(db).await?.into();
    am.sms_otp_template_id = Set(prefs.sms_otp_template_id);
    am.otp_template_id = Set(prefs.otp_template_id);
    am.msg91_auth_key = Set(prefs.msg91_auth_key);
    am.sms_otp_field_name = Set(prefs.sms_otp_field_name);
    am.sms_otp_extra_fields = Set(prefs.sms_otp_extra_fields);
    am.email_otp_template_string = Set(prefs.email_otp_template_string);
    am.smtp_host = Set(prefs.smtp_host);
    am.smtp_port = Set(prefs.smtp_port);
    am.smtp_username = Set(prefs.smtp_username);
    am.smtp_password = Set(prefs.smtp_password);
    am.smtp_from = Set(prefs.smtp_from);
    am.updated_at = Set(Some(Utc::now()));
    Ok(am.update(db).await?)
}
