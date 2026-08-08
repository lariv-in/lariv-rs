//! Shared Axum state for LLM assistant routes (DB, config, Gemini client).
use sea_orm::DatabaseConnection;

use super::config::LlmAssistantConfig;
use super::genai::GenaiClient;
use super::preferences::resolved_api_key;

/// Shared Axum state for the LLM assistant plugin routes.
#[derive(Clone)]
pub struct LlmAssistantState {
    pub db: DatabaseConnection,
    pub config: LlmAssistantConfig,
    /// Gemini client with model from config; API key is applied per request from preferences.
    pub genai: GenaiClient,
}

impl LlmAssistantState {
    pub fn new(db: DatabaseConnection, config: LlmAssistantConfig) -> Self {
        // Key is loaded from DB preferences (or env) when making Gemini calls.
        let genai = GenaiClient::new(String::new(), config.chat_model.clone());
        Self { db, config, genai }
    }

    /// Clone of [`Self::genai`] with the currently configured Gemini API key.
    pub async fn genai_with_key(&self) -> Result<GenaiClient, sea_orm::DbErr> {
        let key = resolved_api_key(&self.db).await?;
        Ok(self.genai.with_api_key(key))
    }
}
