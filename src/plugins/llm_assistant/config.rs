//! Assistant plugin configuration (`[llm_assistant]` in TOML).
//!
//! Absorbs (`apiKey`) and `p_llm_assistant` (CSE + chat model).

use serde::Deserialize;

use crate::config::ConfigSection;

/// Config HList tag for [`LlmAssistantConfig`].
pub struct LlmAssistantConfigTag;

impl ConfigSection for LlmAssistantConfigTag {
    const KEY: Option<&'static str> = Some("llm_assistant");
}

const DEFAULT_CHAT_MODEL: &str = "gemini-2.5-flash";

/// Hard-coded app limits.
pub const CHAT_MAX_OUTPUT_TOKENS: i32 = 4096;
pub const ASSISTANT_TOOL_ROUNDS: i32 = 14;
pub const GOOGLE_SEARCH_RESULT_LIMIT_CAP: i32 = 20;

#[derive(Debug, Clone, Deserialize)]
pub struct LlmAssistantConfig {
    /// Gemini API key. Empty → `GOOGLE_API_KEY` / `GEMINI_API_KEY` env fallback.
    #[serde(default, rename = "apiKey")]
    pub api_key: String,
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
            api_key: String::new(),
            cse_api_key: String::new(),
            cse_cx: String::new(),
            chat_model: default_chat_model(),
        }
    }
}

impl LlmAssistantConfig {
    /// Resolved Gemini API key (TOML, then env).
    pub fn resolved_api_key(&self) -> String {
        let key = self.api_key.trim();
        if !key.is_empty() {
            return key.to_string();
        }
        std::env::var("GOOGLE_API_KEY")
            .or_else(|_| std::env::var("GEMINI_API_KEY"))
            .unwrap_or_default()
    }
}
