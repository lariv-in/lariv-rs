use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_assistant_preferences")]
/// Singleton preferences row (`id = 1`).
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    /// Gemini API key (stored in DB; empty → env fallback at resolve time).
    pub api_key: String,
    /// Gemini model id (empty → [`crate::plugins::llm_assistant::config::DEFAULT_CHAT_MODEL`]).
    pub chat_model: String,
    /// Google Custom Search JSON API key.
    pub cse_api_key: String,
    /// Google Programmable Search engine id (`cx`).
    pub cse_cx: String,
    /// IMAP server hostname.
    pub imap_server: String,
    /// IMAP server port.
    pub imap_port: String,
    /// SMTP server hostname.
    pub smtp_server: String,
    /// SMTP server port.
    pub smtp_port: String,
    /// Email account address.
    pub email: String,
    /// Email account password.
    pub password: String,
    /// Mail encryption: `ssl` or `tls`.
    pub mail_encryption: String,
    /// Natural-language criteria for inbound email triage.
    pub email_filter: String,
    /// Staff user who owns auto-created email sessions.
    pub email_owner_user_id: Option<i64>,
    /// Parent directory for saving inbound email attachments (filesystem VNode id).
    pub email_attachments_parent_id: Option<i64>,
    /// Parent directory for chat conversation attachment folders (filesystem VNode id).
    pub chat_attachments_parent_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub type LlmAssistantPreferences = Model;
