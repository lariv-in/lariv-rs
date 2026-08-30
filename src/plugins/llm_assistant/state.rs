//! Shared Axum state for LLM assistant routes (DB, config, Gemini client).
use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::{
    llm_tools::LlmToolsCapability, plugins::filesystem::storage::DynFilestore,
    rune_env::RuneEnvCapability,
};

use super::config::LlmAssistantConfig;
use super::email_listener::EmailListenerHandle;
use super::genai::GenaiClient;
use super::live_turn::LiveTurns;
use super::preferences::{resolved_api_key, resolved_chat_model};

/// Dependencies for background email-triggered assistant turns.
#[derive(Clone)]
pub struct EmailAutomationDeps {
    pub store: Arc<DynFilestore>,
    pub tools: Arc<LlmToolsCapability>,
    pub rune_env: Arc<RuneEnvCapability>,
}

/// Shared Axum state for the LLM assistant plugin routes.
#[derive(Clone)]
pub struct LlmAssistantState {
    pub db: DatabaseConnection,
    pub config: LlmAssistantConfig,
    /// Gemini client; API key and model are applied per request from preferences.
    pub genai: GenaiClient,
    /// In-flight turns keyed by session id (survives WebSocket reconnect).
    pub live_turns: LiveTurns,
    /// Filestore, tools, and Rune env for email-triggered turns.
    pub email_automation: EmailAutomationDeps,
    /// Handle for restarting the background IMAP IDLE listener.
    pub email_listener: EmailListenerHandle,
}

impl LlmAssistantState {
    pub fn new(
        db: DatabaseConnection,
        config: LlmAssistantConfig,
        email_automation: EmailAutomationDeps,
    ) -> Self {
        let chat_model = config.chat_model.clone();
        Self {
            db,
            config,
            genai: GenaiClient::new(String::new(), chat_model),
            live_turns: LiveTurns::new(),
            email_automation,
            email_listener: super::email_listener::new_handle(),
        }
    }

    /// Bind shared state into the email listener handle (does not start IMAP).
    pub fn bind_email_listener(self) -> Self {
        let state = Arc::new(self);
        state.email_listener.bind(Arc::clone(&state));
        Arc::try_unwrap(state).unwrap_or_else(|arc| (*arc).clone())
    }

    /// Clone of [`Self::genai`] with the current Gemini API key and chat model.
    pub async fn genai_with_key(&self) -> Result<GenaiClient, sea_orm::DbErr> {
        let key = resolved_api_key(&self.db).await?;
        let model = resolved_chat_model(&self.db, &self.config.chat_model).await?;
        Ok(self.genai.with_api_key(key).with_model(model))
    }
}
