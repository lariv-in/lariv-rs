//! Singleton LLM Assistant preferences (`id = 1`).
//!
//! Holds Gemini API key / chat model, Google Custom Search (`cse_api_key`, `cse_cx`),
//! and email (IMAP/SMTP) credentials.

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, DbErr, EntityTrait};

use crate::genai::GenaiClient;

use super::config::DEFAULT_CHAT_MODEL;
use super::entities::{
    LlmAssistantPreferences,
    llm_assistant_preferences::{self, Entity as PrefsEntity},
};

/// Load singleton preferences row (`id = 1`), creating it if missing.
pub async fn load_preferences(db: &DatabaseConnection) -> Result<LlmAssistantPreferences, DbErr> {
    if let Some(prefs) = PrefsEntity::find_by_id(1).one(db).await? {
        return Ok(prefs);
    }

    let now = Utc::now();
    let model = llm_assistant_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        api_key: Set(String::new()),
        chat_model: Set(DEFAULT_CHAT_MODEL.to_string()),
        cse_api_key: Set(String::new()),
        cse_cx: Set(String::new()),
        imap_server: Set(String::new()),
        imap_port: Set(String::new()),
        smtp_server: Set(String::new()),
        smtp_port: Set(String::new()),
        email: Set(String::new()),
        password: Set(String::new()),
        mail_encryption: Set(DEFAULT_MAIL_ENCRYPTION.to_string()),
        email_filter: Set(String::new()),
        email_owner_user_id: Set(None),
        email_attachments_parent_id: Set(None),
    };
    model.insert(db).await
}

/// Persist preferences fields onto the singleton row.
pub async fn save_preferences(
    db: &DatabaseConnection,
    prefs: LlmAssistantPreferences,
) -> Result<LlmAssistantPreferences, DbErr> {
    let mut am: llm_assistant_preferences::ActiveModel = load_preferences(db).await?.into();
    am.api_key = Set(prefs.api_key);
    am.chat_model = Set(chat_model_or_default(&prefs.chat_model, DEFAULT_CHAT_MODEL));
    am.cse_api_key = Set(prefs.cse_api_key);
    am.cse_cx = Set(prefs.cse_cx);
    am.imap_server = Set(prefs.imap_server);
    am.imap_port = Set(prefs.imap_port);
    am.smtp_server = Set(prefs.smtp_server);
    am.smtp_port = Set(prefs.smtp_port);
    am.email = Set(prefs.email);
    am.password = Set(prefs.password);
    am.mail_encryption = Set(mail_encryption_or_default(&prefs.mail_encryption));
    am.email_filter = Set(prefs.email_filter);
    am.email_owner_user_id = Set(prefs.email_owner_user_id.filter(|id| *id > 0));
    am.email_attachments_parent_id =
        Set(prefs.email_attachments_parent_id.filter(|id| *id > 0));
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await
}

/// Gemini API key: preferences row first, then `GOOGLE_API_KEY` / `GEMINI_API_KEY`.
pub async fn resolved_api_key(db: &DatabaseConnection) -> Result<String, DbErr> {
    let prefs = load_preferences(db).await?;
    Ok(api_key_from_prefs_or_env(&prefs.api_key))
}

/// Chat model: preferences row if set, otherwise `fallback` (usually config `chatModel`).
pub async fn resolved_chat_model(db: &DatabaseConnection, fallback: &str) -> Result<String, DbErr> {
    let prefs = load_preferences(db).await?;
    Ok(chat_model_or_default(&prefs.chat_model, fallback))
}

pub fn api_key_from_prefs_or_env(prefs_key: &str) -> String {
    let key = prefs_key.trim();
    if !key.is_empty() {
        return key.to_string();
    }
    std::env::var("GOOGLE_API_KEY")
        .or_else(|_| std::env::var("GEMINI_API_KEY"))
        .unwrap_or_default()
}

pub const DEFAULT_MAIL_ENCRYPTION: &str = "tls";

pub fn mail_encryption_or_default(raw: &str) -> String {
    let enc = raw.trim();
    if enc == "ssl" || enc == "tls" {
        enc.to_string()
    } else {
        DEFAULT_MAIL_ENCRYPTION.to_string()
    }
}

pub fn mail_encryption_choices() -> Vec<(String, String)> {
    vec![
        ("ssl".to_string(), "SSL (port 993)".to_string()),
        ("tls".to_string(), "STARTTLS (port 143)".to_string()),
    ]
}

pub fn chat_model_or_default(raw: &str, fallback: &str) -> String {
    let model = raw.trim();
    if model.is_empty() {
        let fallback = fallback.trim();
        if fallback.is_empty() {
            DEFAULT_CHAT_MODEL.to_string()
        } else {
            fallback.to_string()
        }
    } else {
        model.to_string()
    }
}

/// `(id, display_name)` pairs for the Gemini model select, plus an optional list error.
pub async fn gemini_model_choices(
    api_key: &str,
    current: &str,
) -> (Vec<(String, String)>, Option<String>) {
    let key = api_key_from_prefs_or_env(api_key);
    let (mut choices, list_error) = if key.is_empty() {
        (
            Vec::new(),
            Some("Save a Gemini API key to load the model list.".to_string()),
        )
    } else {
        let client = GenaiClient::new(key, String::new());
        match client.list_generate_content_models().await {
            Ok(models) => (models, None),
            Err(e) => (
                Vec::new(),
                Some(format!("Could not list Gemini models: {e}")),
            ),
        }
    };
    if !current.is_empty() && !choices.iter().any(|(id, _)| id == current) {
        choices.insert(0, (current.to_string(), current.to_string()));
    }
    (choices, list_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_model_uses_fallback_then_default() {
        assert_eq!(
            chat_model_or_default("", "gemini-2.0-flash"),
            "gemini-2.0-flash"
        );
        assert_eq!(chat_model_or_default("  ", ""), DEFAULT_CHAT_MODEL);
        assert_eq!(
            chat_model_or_default(" gemini-2.5-pro ", "ignored"),
            "gemini-2.5-pro"
        );
    }

    #[tokio::test]
    async fn empty_api_key_does_not_list_models() {
        let (choices, err) = gemini_model_choices("", "gemini-2.5-flash").await;
        assert_eq!(
            choices,
            vec![(
                "gemini-2.5-flash".to_string(),
                "gemini-2.5-flash".to_string()
            )]
        );
        assert!(
            err.as_deref()
                .is_some_and(|e| e.contains("Save a Gemini API key")),
            "{err:?}"
        );
    }
}
