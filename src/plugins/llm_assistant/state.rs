//! Shared Axum state for LLM assistant routes (DB, config, Gemini client).
use sea_orm::DatabaseConnection;

use super::config::LlmAssistantConfig;
use super::genai::GenaiClient;
use super::live_turn::LiveTurns;
use super::preferences::{resolved_api_key, resolved_chat_model};

/// Shared Axum state for the LLM assistant plugin routes.
#[derive(Clone)]
pub struct LlmAssistantState {
    pub db: DatabaseConnection,
    pub config: LlmAssistantConfig,
    /// Gemini client; API key and model are applied per request from preferences.
    pub genai: GenaiClient,
    /// In-flight turns keyed by session id (survives WebSocket reconnect).
    pub live_turns: LiveTurns,
}

impl LlmAssistantState {
    pub fn new(db: DatabaseConnection, config: LlmAssistantConfig) -> Self {
        // Key is loaded from DB preferences (or env) when making Gemini calls.
        let genai = GenaiClient::new(String::new(), config.chat_model.clone());
        Self {
            db,
            config,
            genai,
            live_turns: LiveTurns::new(),
        }
    }

    /// Clone of [`Self::genai`] with the current Gemini API key and chat model.
    pub async fn genai_with_key(&self) -> Result<GenaiClient, sea_orm::DbErr> {
        let key = resolved_api_key(&self.db).await?;
        let model = resolved_chat_model(&self.db, &self.config.chat_model).await?;
        Ok(self.genai.with_api_key(key).with_model(model))
    }
}
