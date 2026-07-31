use sea_orm::DatabaseConnection;

use super::config::LlmAssistantConfig;
use super::genai::GenaiClient;

/// Shared Axum state for the LLM assistant plugin routes.
#[derive(Clone)]
pub struct LlmAssistantState {
    pub db: DatabaseConnection,
    pub config: LlmAssistantConfig,
    pub genai: GenaiClient,
}

impl LlmAssistantState {
    pub fn new(db: DatabaseConnection, config: LlmAssistantConfig) -> Self {
        let genai = GenaiClient::new(config.resolved_api_key(), config.chat_model.clone());
        Self { db, config, genai }
    }
}
