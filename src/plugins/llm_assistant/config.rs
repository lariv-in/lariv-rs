//! Assistant plugin configuration (`[llm_assistant]` in TOML).
//!
//! The Gemini API key, selected chat model, and Google CSE credentials are stored in
//! [`crate::plugins::llm_assistant::preferences`] (DB). [`LlmAssistantConfig::chat_model`]
//! is the default when preferences have no model yet.

use serde::Deserialize;

use crate::config::ConfigSection;

/// Config HList tag for [`LlmAssistantConfig`].
pub struct LlmAssistantConfigTag;

impl ConfigSection for LlmAssistantConfigTag {
    const KEY: Option<&'static str> = Some("llm_assistant");
}

pub const DEFAULT_CHAT_MODEL: &str = "gemini-2.5-flash";
/// Default context-window fill that triggers chat compaction.
pub const COMPACTION_THRESHOLD_PERCENT: u32 = 80;

/// Hard-coded app limits.
pub const CHAT_MAX_OUTPUT_TOKENS: i32 = 4096;
/// Max tokens for a compaction summary.
pub const COMPACTION_MAX_OUTPUT_TOKENS: i32 = 8192;
pub const ASSISTANT_TOOL_ROUNDS: i32 = 128;
/// Fallback Gemini input window when `models.get` does not return `inputTokenLimit`.
pub const DEFAULT_INPUT_TOKEN_LIMIT: u32 = 1_048_576;
pub const GOOGLE_SEARCH_RESULT_LIMIT_CAP: i32 = 20;
pub const WEBPAGE_TEXT_CHAR_LIMIT: usize = 50_000;

/// Max attachment parts kept per inbound email.
pub const EMAIL_MAX_ATTACHMENTS: usize = 10;
/// Max bytes per attachment part.
pub const EMAIL_MAX_ATTACHMENT_BYTES: usize = 10 * 1024 * 1024;
/// Max total attachment bytes per inbound email.
pub const EMAIL_MAX_TOTAL_ATTACHMENT_BYTES: usize = 25 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
pub struct LlmAssistantConfig {
    #[serde(default = "default_chat_model", rename = "chatModel")]
    pub chat_model: String,
}

fn default_chat_model() -> String {
    DEFAULT_CHAT_MODEL.into()
}

impl Default for LlmAssistantConfig {
    fn default() -> Self {
        Self {
            chat_model: default_chat_model(),
        }
    }
}
