//! Thin Gemini HTTP client (`generateContent` + `streamGenerateContent`).
//!
//! [`GenaiClient`] wraps reqwest calls to `generativelanguage.googleapis.com`. Use
//! [`GenaiClient::generate_text`] for one-shot prompts (Totschool workers) or
//! [`GenaiClient::generate_content`] / [`GenaiClient::stream_generate_content`] for
//! multi-turn LLM Assistant chat with optional function declarations.
//!
//! # Configuration
//!
//! - API key: constructor argument or `GOOGLE_API_KEY` / `GEMINI_API_KEY` via [`GenaiClient::from_env`]
//! - Model: e.g. `"gemini-2.0-flash"` passed to `new` / `from_env`
//!
//! # Examples
//!
//! ```rust ignore
//! let client = GenaiClient::from_env("gemini-2.0-flash");
//!
//! // Simple text generation:
//! let summary = client.generate_text("You summarize.", user_input).await?;
//!
//! // Streaming with tool declarations:
//! let merged = client
//!     .stream_generate_content(history, 8192, &decls, |chunk| { /* push SSE */ })
//!     .await?;
//! ```

use futures_util::StreamExt;
use tokio::time::{Duration, sleep};

use super::errors::GenaiError;
use super::types::{
    Content, FunctionDeclaration, GenerateContentRequest, GenerateContentResponse,
    GenerationConfig, ROLE_USER, Tool, ToolConfig,
};
use super::util::{content_text, merge_content};

/// Default system prompt for the LLM Assistant plugin (skills, tools, multimodal guidance).
pub const ASSISTANT_SYSTEM_PROMPT: &str = r#"You are LLM Assistant inside the Lariv app. You help operators search the public web via Google Programmable Search.

You are a multimodal assistant. You can see, analyze, and process any files, documents, or images attached by the user.

CRITICAL: You have access to various registered skills that help you handle tasks. You MUST check the list of available skills (by calling the list_skills tool) before generating your response to see if an existing skill is suited to the user's request. Checking for available skills is your absolute highest priority.

To properly use a skill, you first need its name, you can get the name using the list_skills tool, then use get_skill_detail to get the content. Content will describe what you need to do with. It will often list rules or a sequence of steps to follow. It may often refer to files, which you can read with read_file. The references files will be listed in the Files section of the response from get_skill_detail.

Even if the task may seem trivial, if a skill might seem to provide some additional information about the task, then you should check the instructions via get_skill_detail.


NOTE: list_skills doesn't give the instructions that are contained in the skill. You NEED to call get_skill_detail to get the instructions.

For normal answers (questions, explanations, summaries after tool results), reply in plain text or markdown.

If a tool response includes an error, explain it briefly and suggest a fix."#;

const GEMINI_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";
const STREAM_MAX_ATTEMPTS: u32 = 4;
const DEFAULT_MAX_OUTPUT_TOKENS: i32 = 8192;

/// HTTP client for Gemini `generateContent` and `streamGenerateContent` endpoints.
#[derive(Clone)]
pub struct GenaiClient {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

impl GenaiClient {
    /// Construct a client with explicit API key and model name.
    ///
    /// Logs a warning if the API key is empty.
    pub fn new(api_key: String, model: String) -> Self {
        if api_key.trim().is_empty() {
            tracing::warn!(
                "genai: no apiKey configured; set plugin apiKey or GOOGLE_API_KEY / GEMINI_API_KEY"
            );
        }
        Self {
            http: reqwest::Client::new(),
            api_key,
            model,
        }
    }

    /// Construct from `GOOGLE_API_KEY` or `GEMINI_API_KEY` environment variables.
    pub fn from_env(model: impl Into<String>) -> Self {
        let api_key = std::env::var("GOOGLE_API_KEY")
            .or_else(|_| std::env::var("GEMINI_API_KEY"))
            .unwrap_or_default();
        Self::new(api_key, model.into())
    }

    /// Configured model identifier (e.g. `"gemini-2.0-flash"`).
    pub fn model(&self) -> &str {
        &self.model
    }

    fn request_body(
        contents: Vec<Content>,
        system_instruction: Option<Content>,
        max_output_tokens: i32,
        tool_decls: &[FunctionDeclaration],
    ) -> GenerateContentRequest {
        let max_out = max_output_tokens.max(1);
        let (tools, tool_config) = if tool_decls.is_empty() {
            (None, None)
        } else {
            (
                Some(vec![Tool {
                    function_declarations: tool_decls.to_vec(),
                }]),
                Some(ToolConfig::auto()),
            )
        };
        GenerateContentRequest {
            contents,
            system_instruction,
            generation_config: Some(GenerationConfig {
                temperature: Some(0.35),
                max_output_tokens: Some(max_out),
            }),
            tools,
            tool_config,
        }
    }

    /// One-shot text generation with a custom system prompt (Totschool letter/proposal workers).
    pub async fn generate_text(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, GenaiError> {
        self.generate_text_with_tokens(system_prompt, user_prompt, DEFAULT_MAX_OUTPUT_TOKENS)
            .await
    }

    pub async fn generate_text_with_tokens(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        max_output_tokens: i32,
    ) -> Result<String, GenaiError> {
        let system = if system_prompt.trim().is_empty() {
            None
        } else {
            Some(Content::text(ROLE_USER, system_prompt))
        };
        let content = self
            .generate_content_with_system(
                vec![Content::text(ROLE_USER, user_prompt)],
                system,
                max_output_tokens,
                &[],
            )
            .await?;
        let text = content_text(&content);
        if text.trim().is_empty() {
            return Err(GenaiError::EmptyResponse);
        }
        Ok(text)
    }

    /// Non-streaming `generateContent` with the LLM Assistant default system prompt.
    pub async fn generate_content(
        &self,
        contents: Vec<Content>,
        max_output_tokens: i32,
        tool_decls: &[FunctionDeclaration],
    ) -> Result<Content, GenaiError> {
        self.generate_content_with_system(
            contents,
            Some(Content::text(ROLE_USER, ASSISTANT_SYSTEM_PROMPT)),
            max_output_tokens,
            tool_decls,
        )
        .await
    }

    /// Non-streaming `generateContent` with a custom system instruction and optional tools.
    pub async fn generate_content_with_system(
        &self,
        contents: Vec<Content>,
        system_instruction: Option<Content>,
        max_output_tokens: i32,
        tool_decls: &[FunctionDeclaration],
    ) -> Result<Content, GenaiError> {
        if self.api_key.trim().is_empty() {
            return Err(GenaiError::MissingApiKey);
        }
        let url = format!(
            "{GEMINI_BASE}/models/{}:generateContent?key={}",
            self.model, self.api_key
        );
        let body = Self::request_body(
            contents,
            system_instruction,
            max_output_tokens,
            tool_decls,
        );

        let resp = self.http.post(&url).json(&body).send().await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(GenaiError::Api {
                status: status.as_u16(),
                body: text,
            });
        }

        let parsed: GenerateContentResponse =
            serde_json::from_str(&text).map_err(|e| GenaiError::Json(e.to_string()))?;
        if let Some(err) = parsed.error {
            return Err(GenaiError::ApiMessage {
                message: err.message,
            });
        }
        let content = parsed
            .candidates
            .into_iter()
            .find_map(|c| c.content)
            .ok_or(GenaiError::EmptyResponse)?;
        Ok(content)
    }

    /// Streaming `streamGenerateContent` (SSE).
    ///
    /// Calls `on_chunk` with progressively merged content; returns the final merged
    /// [`Content`]. Retries on quota / 429 errors up to four attempts.
    pub async fn stream_generate_content<F>(
        &self,
        contents: Vec<Content>,
        max_output_tokens: i32,
        tool_decls: &[FunctionDeclaration],
        mut on_chunk: F,
    ) -> Result<Content, GenaiError>
    where
        F: FnMut(&Content) + Send,
    {
        if self.api_key.trim().is_empty() {
            return Err(GenaiError::MissingApiKey);
        }
        let url = format!(
            "{GEMINI_BASE}/models/{}:streamGenerateContent?alt=sse&key={}",
            self.model, self.api_key
        );
        let body = Self::request_body(
            contents,
            Some(Content::text(ROLE_USER, ASSISTANT_SYSTEM_PROMPT)),
            max_output_tokens,
            tool_decls,
        );

        let mut last_err = None;
        for attempt in 0..STREAM_MAX_ATTEMPTS {
            if attempt > 0 {
                let backoff_ms = 500u64.saturating_mul(1u64 << (attempt - 1)).min(12_000);
                sleep(Duration::from_millis(backoff_ms)).await;
            }
            match self.stream_once(&url, &body, &mut on_chunk).await {
                Ok(merged) => return Ok(merged),
                Err(e) if attempt + 1 < STREAM_MAX_ATTEMPTS && is_retryable_quota(&e) => {
                    tracing::warn!(attempt, error = %e, "genai: retrying stream");
                    last_err = Some(e);
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_err.unwrap_or(GenaiError::EmptyResponse))
    }

    async fn stream_once<F>(
        &self,
        url: &str,
        body: &GenerateContentRequest,
        on_chunk: &mut F,
    ) -> Result<Content, GenaiError>
    where
        F: FnMut(&Content) + Send,
    {
        let resp = self.http.post(url).json(body).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(GenaiError::Api {
                status: status.as_u16(),
                body: text,
            });
        }

        let mut merged: Option<Content> = None;
        let mut buffer = String::new();
        let mut stream = resp.bytes_stream();
        while let Some(item) = stream.next().await {
            let chunk = item.map_err(GenaiError::Http)?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(idx) = buffer.find('\n') {
                let line = buffer[..idx].trim_end_matches('\r').to_string();
                buffer = buffer[idx + 1..].to_string();
                if let Some(data) = line.strip_prefix("data:") {
                    let data = data.trim();
                    if data.is_empty() || data == "[DONE]" {
                        continue;
                    }
                    let parsed: GenerateContentResponse = serde_json::from_str(data)
                        .map_err(|e| GenaiError::Json(e.to_string()))?;
                    if let Some(err) = parsed.error {
                        return Err(GenaiError::ApiMessage {
                            message: err.message,
                        });
                    }
                    if let Some(delta) = parsed.candidates.into_iter().find_map(|c| c.content) {
                        merged = Some(merge_content(merged, delta));
                        if let Some(ref m) = merged {
                            on_chunk(m);
                        }
                    }
                }
            }
        }
        let trailing = buffer.trim();
        if let Some(data) = trailing.strip_prefix("data:") {
            let data = data.trim();
            if !data.is_empty() && data != "[DONE]" {
                let parsed: GenerateContentResponse =
                    serde_json::from_str(data).map_err(|e| GenaiError::Json(e.to_string()))?;
                if let Some(delta) = parsed.candidates.into_iter().find_map(|c| c.content) {
                    merged = Some(merge_content(merged, delta));
                    if let Some(ref m) = merged {
                        on_chunk(m);
                    }
                }
            }
        }

        merged.ok_or(GenaiError::EmptyResponse)
    }
}

fn is_retryable_quota(err: &GenaiError) -> bool {
    match err {
        GenaiError::Api { status, body } => {
            *status == 429
                || body.to_lowercase().contains("resource_exhausted")
                || body.to_lowercase().contains("quota")
        }
        GenaiError::ApiMessage { message } => {
            let m = message.to_lowercase();
            m.contains("resource_exhausted") || m.contains("quota") || m.contains("429")
        }
        _ => false,
    }
}

/// Back-compat alias for LLM Assistant streaming merge (see [`crate::genai::merge_content`]).
pub fn merge_assistant_content(dst: Option<Content>, src: Content) -> Content {
    merge_content(dst, src)
}
