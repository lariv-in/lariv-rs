use sea_orm::DatabaseConnection;

use crate::plugins::otp::{
    adapters::smtp::send_otp_email,
    preferences::load_preferences,
};

pub async fn send_portal_credentials_email(
    db: &DatabaseConnection,
    to_email: &str,
    name: &str,
    password: &str,
) -> Result<(), String> {
    let prefs = load_preferences(db).await.map_err(|e| e.to_string())?;
    if prefs.smtp_host.is_empty() || prefs.smtp_from.is_empty() {
        return Err("SMTP is not configured".to_string());
    }

    let body = format!(
        "Hello {name},\n\n\
Your portal account has been created.\n\n\
Email: {email}\n\
Password: {password}\n\n\
You can sign in at the login page using these credentials.\n",
        name = name.trim(),
        email = to_email.trim(),
        password = password,
    );

    send_otp_email(&prefs, to_email, &body)
        .await
        .map_err(|e| e.to_string())
}
