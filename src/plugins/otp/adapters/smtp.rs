use lettre::{
    Message, SmtpTransport, Transport,
    message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use thiserror::Error;

use crate::plugins::otp::entities::OtpPreferences;

#[derive(Debug, Error)]
pub enum SmtpError {
    #[error("invalid from address: {0}")]
    From(String),
    #[error("invalid to address: {0}")]
    To(String),
    #[error("failed to build message: {0}")]
    Build(String),
    #[error("SMTP send failed: {0}")]
    Send(String),
}

/// Send a plain-text OTP email.
pub async fn send_otp_email(
    prefs: &OtpPreferences,
    to_email: &str,
    body: &str,
) -> Result<(), SmtpError> {
    let from: Mailbox = prefs
        .smtp_from
        .parse()
        .map_err(|e| SmtpError::From(format!("{e}")))?;
    let to: Mailbox = to_email
        .parse()
        .map_err(|e| SmtpError::To(format!("{e}")))?;

    let email = Message::builder()
        .from(from)
        .to(to)
        .subject("Your OTP Code")
        .body(body.to_string())
        .map_err(|e| SmtpError::Build(e.to_string()))?;

    let port: u16 = prefs.smtp_port.parse().unwrap_or(25);
    let prefs_host = prefs.smtp_host.clone();
    let prefs_user = prefs.smtp_username.clone();
    let prefs_pass = prefs.smtp_password.clone();

    tokio::task::spawn_blocking(move || {
        let mut builder = SmtpTransport::builder_dangerous(&prefs_host).port(port);
        if !prefs_user.is_empty() {
            builder = builder.credentials(Credentials::new(prefs_user, prefs_pass));
        }
        let mailer = builder.build();
        mailer
            .send(&email)
            .map(|_| ())
            .map_err(|e| SmtpError::Send(e.to_string()))
    })
    .await
    .map_err(|e| SmtpError::Send(e.to_string()))?
}
