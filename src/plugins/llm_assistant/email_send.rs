//! Outbound SMTP replies for email-triggered assistant sessions.

use lettre::{
    Message, SmtpTransport, Transport,
    message::{Mailbox, MultiPart, SinglePart},
    message::header::{InReplyTo, MessageId, References},
    transport::smtp::authentication::Credentials,
    transport::smtp::client::{Tls, TlsParameters},
};
use thiserror::Error;

use crate::components::markdown::render_markdown_email;

use super::{
    entities::LlmAssistantPreferences,
    preferences::mail_encryption_or_default,
};

const LOG_TARGET: &str = "llm_assistant::imap";

#[derive(Debug, Clone, Default)]
pub struct EmailThreading {
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
}

#[derive(Debug, Error)]
pub enum EmailSendError {
    #[error("invalid from address: {0}")]
    From(String),
    #[error("invalid to address: {0}")]
    To(String),
    #[error("failed to build message: {0}")]
    Build(String),
    #[error("SMTP send failed: {0}")]
    Send(String),
}

/// Send a reply using SMTP preferences.
///
/// `body` is treated as markdown: clients receive a `multipart/alternative`
/// with the original text plus an HTML part rendered from it.
///
/// Returns `Ok(())` without sending when the SMTP host is empty (not configured).
pub async fn send_reply_email(
    prefs: &LlmAssistantPreferences,
    to: &str,
    subject: &str,
    body: &str,
    threading: EmailThreading,
) -> Result<(), EmailSendError> {
    let host = prefs.smtp_server.trim();
    if host.is_empty() {
        tracing::warn!(target: LOG_TARGET, "SMTP not configured; skipping email reply");
        return Ok(());
    }

    let from_addr = prefs.email.trim();
    if from_addr.is_empty() {
        tracing::warn!(target: LOG_TARGET, "SMTP from email not configured; skipping reply");
        return Ok(());
    }

    let from: Mailbox = from_addr
        .parse()
        .map_err(|e| EmailSendError::From(format!("{e}")))?;
    let to_mailbox: Mailbox = to
        .parse()
        .map_err(|e| EmailSendError::To(format!("{e}")))?;

    let outbound_id = format!(
        "<{}.{}@{}>",
        uuid::Uuid::new_v4(),
        chrono::Utc::now().timestamp(),
        from_domain(from_addr)
    );

    let mut builder = Message::builder()
        .from(from)
        .to(to_mailbox)
        .subject(subject)
        .header(MessageId::from(outbound_id));

    if let Some(in_reply_to) = threading
        .in_reply_to
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        builder = builder.header(InReplyTo::from(in_reply_to.to_string()));
    }

    let references = build_references(&threading);
    if !references.is_empty() {
        builder = builder.header(References::from(references));
    }

    let email = builder
        .multipart(reply_body_multipart(body))
        .map_err(|e| EmailSendError::Build(e.to_string()))?;

    let encryption = mail_encryption_or_default(&prefs.mail_encryption);
    let default_port = if encryption == "ssl" { 465 } else { 587 };
    let port: u16 = prefs.smtp_port.trim().parse().unwrap_or(default_port);
    let username = prefs.email.trim().to_string();
    let password = prefs.password.clone();
    let host = host.to_string();

    tokio::task::spawn_blocking(move || {
        let tls = TlsParameters::builder(host.clone())
            .build()
            .map_err(|e| EmailSendError::Send(e.to_string()))?;
        let tls_mode = if encryption == "ssl" || port == 465 {
            Tls::Wrapper(tls)
        } else {
            Tls::Required(tls)
        };

        let mut builder = SmtpTransport::relay(&host)
            .map_err(|e| EmailSendError::Send(e.to_string()))?
            .port(port)
            .tls(tls_mode);
        if !username.is_empty() {
            builder = builder.credentials(Credentials::new(username, password));
        }
        let mailer = builder.build();
        mailer
            .send(&email)
            .map(|_| ())
            .map_err(|e| EmailSendError::Send(e.to_string()))
    })
    .await
    .map_err(|e| EmailSendError::Send(e.to_string()))?
}

/// Build `multipart/alternative` with plain markdown and rendered HTML.
fn reply_body_multipart(markdown: &str) -> MultiPart {
    MultiPart::alternative()
        .singlepart(SinglePart::plain(markdown.to_string()))
        .singlepart(SinglePart::html(wrap_email_html(&render_markdown_email(
            markdown,
        ))))
}

fn wrap_email_html(body: &str) -> String {
    format!(
        "<!DOCTYPE html>\
<html>\
<head><meta charset=\"utf-8\"></head>\
<body style=\"font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif;\
font-size:14px;line-height:1.5;color:#111;\">{body}</body>\
</html>"
    )
}

fn build_references(threading: &EmailThreading) -> String {
    let refs = threading.references.as_deref().unwrap_or("").trim();
    let in_reply = threading.in_reply_to.as_deref().unwrap_or("").trim();
    match (refs.is_empty(), in_reply.is_empty()) {
        (true, true) => String::new(),
        (false, true) => refs.to_string(),
        (true, false) => in_reply.to_string(),
        (false, false) => format!("{refs} {in_reply}"),
    }
}

fn from_domain(addr: &str) -> String {
    addr.split('@')
        .nth(1)
        .unwrap_or("localhost")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn skips_when_smtp_host_empty() {
        let prefs = LlmAssistantPreferences {
            id: 1,
            created_at: None,
            updated_at: None,
            api_key: String::new(),
            chat_model: String::new(),
            cse_api_key: String::new(),
            cse_cx: String::new(),
            imap_server: String::new(),
            imap_port: String::new(),
            smtp_server: String::new(),
            smtp_port: String::new(),
            email: "bot@example.com".to_string(),
            password: String::new(),
            mail_encryption: String::new(),
            email_filter: String::new(),
            email_owner_user_id: None,
            email_attachments_parent_id: None,
        };
        send_reply_email(
            &prefs,
            "user@example.com",
            "Re: hi",
            "hello",
            EmailThreading::default(),
        )
        .await
        .expect("skip send");
    }

    #[test]
    fn references_chain_includes_in_reply_to() {
        let threading = EmailThreading {
            in_reply_to: Some("<b@test>".into()),
            references: Some("<a@test>".into()),
        };
        assert_eq!(build_references(&threading), "<a@test> <b@test>");
    }

    #[test]
    fn reply_body_includes_html_and_plain() {
        let formatted = String::from_utf8(reply_body_multipart("**hi**").formatted())
            .expect("utf8 multipart");
        assert!(formatted.contains("text/plain"), "{formatted}");
        assert!(formatted.contains("text/html"), "{formatted}");
        assert!(formatted.contains("**hi**"), "{formatted}");
        assert!(formatted.contains("<strong>hi</strong>"), "{formatted}");
        assert!(formatted.contains("<!DOCTYPE html>"), "{formatted}");
    }
}
