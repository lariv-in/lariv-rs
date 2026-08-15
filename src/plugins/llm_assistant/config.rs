//! Assistant plugin configuration (`[llm_assistant]` in TOML).
//!
//! CSE credentials remain in TOML. The Gemini API key and selected chat model are
//! stored in [`crate::plugins::llm_assistant::preferences`] (DB). [`LlmAssistantConfig::chat_model`]
//! is the default when preferences have no model yet.

use serde::Deserialize;

use crate::config::ConfigSection;

/// Config HList tag for [`LlmAssistantConfig`].
pub struct LlmAssistantConfigTag;

impl ConfigSection for LlmAssistantConfigTag {
    const KEY: Option<&'static str> = Some("llm_assistant");
}

pub const DEFAULT_CHAT_MODEL: &str = "gemini-2.5-flash";

/// Hard-coded app limits.
pub const CHAT_MAX_OUTPUT_TOKENS: i32 = 4096;
pub const ASSISTANT_TOOL_ROUNDS: i32 = 14;
pub const GOOGLE_SEARCH_RESULT_LIMIT_CAP: i32 = 20;

#[derive(Debug, Clone, Deserialize)]
pub struct LlmAssistantConfig {
    #[serde(default, rename = "cseApiKey")]
    pub cse_api_key: String,
    #[serde(default, rename = "cseCx")]
    pub cse_cx: String,
    #[serde(default = "default_chat_model", rename = "chatModel")]
    pub chat_model: String,
}

fn default_chat_model() -> String {
    DEFAULT_CHAT_MODEL.into()
}

impl Default for LlmAssistantConfig {
    fn default() -> Self {
        Self {
            cse_api_key: String::new(),
            cse_cx: String::new(),
            chat_model: default_chat_model(),
        }
    }
}
