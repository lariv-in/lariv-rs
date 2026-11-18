//! LLM Assistant preferences for the signed-in user.
//!
//! Holds Gemini API key / chat model, Google Custom Search (`cse_api_key`, `cse_cx`),
//! and email (IMAP/SMTP) credentials.
//!
//! Deployments may register [`SharedLlmCredentials`] so Gemini and CSE keys live
//! outside this table (for example a site-wide preferences app).

use std::sync::{Arc, Mutex};

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    DatabaseConnection, DbErr, EntityTrait, QuerySelect,
};

use crate::genai::GenaiClient;

use super::config::{
    COMPACTION_THRESHOLD_PERCENT, DEFAULT_CHAT_MAX_OUTPUT_TOKENS, DEFAULT_CHAT_MODEL,
};
use super::entities::{
    LlmAssistantPreferences,
    llm_assistant_preferences::{self, Entity as PrefsEntity},
};

/// Preferences plus the user who owns the row, for background workers.
#[async_trait::async_trait]
pub trait BackgroundPreferences: Send + Sync {
    /// Rows a worker may use. Empty when nobody has configured the assistant.
    async fn owned_preferences(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<(i64, LlmAssistantPreferences)>, DbErr>;
}

static BACKGROUND: Mutex<Option<Arc<dyn BackgroundPreferences>>> = Mutex::new(None);

/// Install the loader used by the IMAP listener when row visibility is per user.
pub fn set_background_preferences(source: Arc<dyn BackgroundPreferences>) {
    *BACKGROUND
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(source);
}

pub(crate) fn background_preferences() -> Option<Arc<dyn BackgroundPreferences>> {
    BACKGROUND
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// Site-wide Gemini and Google CSE credentials, when a deployment owns them.
#[async_trait::async_trait]
pub trait SharedLlmCredentials: Send + Sync {
    async fn api_key(&self, db: &DatabaseConnection) -> Result<String, DbErr>;
    async fn cse_api_key(&self, db: &DatabaseConnection) -> Result<String, DbErr>;
    async fn cse_cx(&self, db: &DatabaseConnection) -> Result<String, DbErr>;
}

static SHARED: Mutex<Option<Arc<dyn SharedLlmCredentials>>> = Mutex::new(None);

/// Install the shared Gemini / CSE credential source for this process.
pub fn set_shared_llm_credentials(source: Arc<dyn SharedLlmCredentials>) {
    *SHARED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(source);
}

pub fn shared_llm_credentials() -> Option<Arc<dyn SharedLlmCredentials>> {
    SHARED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// True when a deployment registered [`SharedLlmCredentials`].
pub fn shared_llm_credentials_registered() -> bool {
    shared_llm_credentials().is_some()
}

/// Load the visible preferences row, creating it if missing.
///
/// A background task with no session user does not insert. That insert is rejected
/// when row-level security is on, and it would create an unowned row.
pub async fn load_preferences(db: &DatabaseConnection) -> Result<LlmAssistantPreferences, DbErr> {
    if let Some(prefs) = PrefsEntity::find().limit(1).one(db).await? {
        return Ok(prefs);
    }
    if lariv_core::db::db_actor().is_none()
        && lariv_plugin_users::role_authorization::current_auth().is_none()
    {
        return Ok(empty_preferences());
    }

    let now = Utc::now();
    let model = llm_assistant_preferences::ActiveModel {
        id: NotSet,
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
        chat_attachments_parent_id: Set(None),
        compactor_model: Set(DEFAULT_CHAT_MODEL.to_string()),
        compaction_threshold_percent: Set(COMPACTION_THRESHOLD_PERCENT as i32),
        max_output_tokens: Set(DEFAULT_CHAT_MAX_OUTPUT_TOKENS),
    };
    model.insert(db).await
}

fn empty_preferences() -> LlmAssistantPreferences {
    LlmAssistantPreferences {
        id: 0,
        created_at: None,
        updated_at: None,
        api_key: String::new(),
        chat_model: DEFAULT_CHAT_MODEL.to_string(),
        cse_api_key: String::new(),
        cse_cx: String::new(),
        imap_server: String::new(),
        imap_port: String::new(),
        smtp_server: String::new(),
        smtp_port: String::new(),
        email: String::new(),
        password: String::new(),
        mail_encryption: DEFAULT_MAIL_ENCRYPTION.to_string(),
        email_filter: String::new(),
        email_owner_user_id: None,
        email_attachments_parent_id: None,
        chat_attachments_parent_id: None,
        compactor_model: DEFAULT_CHAT_MODEL.to_string(),
        compaction_threshold_percent: COMPACTION_THRESHOLD_PERCENT as i32,
        max_output_tokens: DEFAULT_CHAT_MAX_OUTPUT_TOKENS,
    }
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
    am.email_attachments_parent_id = Set(prefs.email_attachments_parent_id.filter(|id| *id > 0));
    am.chat_attachments_parent_id = Set(prefs.chat_attachments_parent_id.filter(|id| *id > 0));
    am.compactor_model = Set(chat_model_or_default(
        &prefs.compactor_model,
        DEFAULT_CHAT_MODEL,
    ));
    am.compaction_threshold_percent =
        Set(compaction_threshold_or_default(prefs.compaction_threshold_percent) as i32);
    am.max_output_tokens = Set(max_output_tokens_or_default(prefs.max_output_tokens));
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await
}

/// Gemini API key: shared credentials, then preferences row, then env.
pub async fn resolved_api_key(db: &DatabaseConnection) -> Result<String, DbErr> {
    if let Some(shared) = shared_llm_credentials() {
        let key = shared.api_key(db).await?;
        let resolved = api_key_from_prefs_or_env(&key);
        if !resolved.is_empty() {
            return Ok(resolved);
        }
    }
    let prefs = load_preferences(db).await?;
    Ok(api_key_from_prefs_or_env(&prefs.api_key))
}

/// Google CSE API key: shared credentials, then preferences row.
pub async fn resolved_cse_api_key(db: &DatabaseConnection) -> Result<String, DbErr> {
    if let Some(shared) = shared_llm_credentials() {
        let key = shared.cse_api_key(db).await?;
        if !key.trim().is_empty() {
            return Ok(key);
        }
    }
    let prefs = load_preferences(db).await?;
    Ok(prefs.cse_api_key)
}

/// Google CSE CX: shared credentials, then preferences row.
pub async fn resolved_cse_cx(db: &DatabaseConnection) -> Result<String, DbErr> {
    if let Some(shared) = shared_llm_credentials() {
        let cx = shared.cse_cx(db).await?;
        if !cx.trim().is_empty() {
            return Ok(cx);
        }
    }
    let prefs = load_preferences(db).await?;
    Ok(prefs.cse_cx)
}

/// Chat model: preferences row if set, otherwise `fallback` (usually config `chatModel`).
pub async fn resolved_chat_model(db: &DatabaseConnection, fallback: &str) -> Result<String, DbErr> {
    let prefs = load_preferences(db).await?;
    Ok(chat_model_or_default(&prefs.chat_model, fallback))
}

/// Compactor model: preferences row if set, otherwise [`DEFAULT_CHAT_MODEL`].
pub async fn resolved_compactor_model(db: &DatabaseConnection) -> Result<String, DbErr> {
    let prefs = load_preferences(db).await?;
    Ok(chat_model_or_default(
        &prefs.compactor_model,
        DEFAULT_CHAT_MODEL,
    ))
}

/// Compaction threshold (1–100); invalid values fall back to [`COMPACTION_THRESHOLD_PERCENT`].
pub async fn resolved_compaction_threshold_percent(db: &DatabaseConnection) -> Result<u32, DbErr> {
    let prefs = load_preferences(db).await?;
    Ok(compaction_threshold_or_default(
        prefs.compaction_threshold_percent,
    ))
}

/// Chat `maxOutputTokens`; invalid values fall back to [`DEFAULT_CHAT_MAX_OUTPUT_TOKENS`].
pub async fn resolved_max_output_tokens(db: &DatabaseConnection) -> Result<i32, DbErr> {
    let prefs = load_preferences(db).await?;
    Ok(max_output_tokens_or_default(prefs.max_output_tokens))
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

pub fn compaction_threshold_or_default(raw: i32) -> u32 {
    if (1..=100).contains(&raw) {
        raw as u32
    } else {
        COMPACTION_THRESHOLD_PERCENT
    }
}

pub fn max_output_tokens_or_default(raw: i32) -> i32 {
    if raw > 0 {
        raw
    } else {
        DEFAULT_CHAT_MAX_OUTPUT_TOKENS
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

    #[test]
    fn threshold_clamps_and_falls_back() {
        assert_eq!(compaction_threshold_or_default(80), 80);
        assert_eq!(compaction_threshold_or_default(1), 1);
        assert_eq!(compaction_threshold_or_default(100), 100);
        assert_eq!(
            compaction_threshold_or_default(0),
            COMPACTION_THRESHOLD_PERCENT
        );
        assert_eq!(
            compaction_threshold_or_default(-1),
            COMPACTION_THRESHOLD_PERCENT
        );
        assert_eq!(
            compaction_threshold_or_default(101),
            COMPACTION_THRESHOLD_PERCENT
        );
    }

    #[test]
    fn max_output_tokens_falls_back_when_invalid() {
        assert_eq!(max_output_tokens_or_default(65_536), 65_536);
        assert_eq!(max_output_tokens_or_default(8192), 8192);
        assert_eq!(
            max_output_tokens_or_default(0),
            DEFAULT_CHAT_MAX_OUTPUT_TOKENS
        );
        assert_eq!(
            max_output_tokens_or_default(-1),
            DEFAULT_CHAT_MAX_OUTPUT_TOKENS
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
