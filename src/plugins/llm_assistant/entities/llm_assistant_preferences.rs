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
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub type LlmAssistantPreferences = Model;
