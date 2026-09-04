//! Shared Axum state for LLM assistant routes (DB, config, Gemini client).
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sea_orm::DatabaseConnection;
use tokio::time::timeout;

use crate::{
    llm_tools::LlmToolsCapability, plugins::filesystem::storage::DynFilestore,
    rune_env::RuneEnvCapability,
};

use super::hitl::HitlCapability;

use super::config::{DEFAULT_INPUT_TOKEN_LIMIT, LlmAssistantConfig};
use super::email_listener::EmailListenerHandle;
use super::genai::GenaiClient;
use super::live_turn::LiveTurns;
use super::preferences::{resolved_api_key, resolved_chat_model};

const MODEL_LIMIT_TIMEOUT: Duration = Duration::from_secs(3);

/// Dependencies for background email-triggered assistant turns.
#[derive(Clone)]
pub struct EmailAutomationDeps {
    pub store: Arc<DynFilestore>,
    pub tools: Arc<LlmToolsCapability>,
    pub rune_env: Arc<RuneEnvCapability>,
    pub hitl: Arc<HitlCapability>,
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
    /// Cached `inputTokenLimit` per Gemini model id.
    model_input_limits: Arc<Mutex<HashMap<String, u32>>>,
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
            model_input_limits: Arc::new(Mutex::new(HashMap::new())),
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

    /// Input token window for the configured chat model (cached; falls back to 1M).
    pub async fn input_token_limit(&self) -> u32 {
        let Ok(genai) = self.genai_with_key().await else {
            return DEFAULT_INPUT_TOKEN_LIMIT;
        };
        let model = genai.model().to_string();
        if model.is_empty() {
            return DEFAULT_INPUT_TOKEN_LIMIT;
        }
        {
            let cache = self
                .model_input_limits
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(&n) = cache.get(&model) {
                return n;
            }
        }
        let fetched = timeout(MODEL_LIMIT_TIMEOUT, genai.model_input_token_limit()).await;
        match fetched {
            Ok(Ok(n)) if n > 0 => {
                self.model_input_limits
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(model, n);
                n
            }
            Ok(Err(e)) => {
                tracing::debug!(error = %e, "llm_assistant: model inputTokenLimit unavailable");
                DEFAULT_INPUT_TOKEN_LIMIT
            }
            Err(_) => {
                tracing::debug!("llm_assistant: model inputTokenLimit timed out");
                DEFAULT_INPUT_TOKEN_LIMIT
            }
            Ok(Ok(_)) => DEFAULT_INPUT_TOKEN_LIMIT,
        }
    }
}
