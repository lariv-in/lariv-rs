//! Thin Gemini HTTP client (`generateContent`, `streamGenerateContent`, `models.list`).
//!
//! [`GenaiClient`] wraps reqwest calls to `generativelanguage.googleapis.com`. Use
//! [`GenaiClient::generate_text`] for one-shot prompts (Totschool workers) or
//! [`GenaiClient::generate_content`] / [`GenaiClient::stream_generate_content`] for
//! multi-turn LLM Assistant chat with optional function declarations.
//! [`GenaiClient::list_generate_content_models`] / [`GenaiClient::list_embed_content_models`]
//! list models by supported action (`generateContent` / `embedContent`).
//!
//! # Configuration
//!
//! - API key: constructor argument, [`GenaiClient::with_api_key`], or `GOOGLE_API_KEY` /
//!   `GEMINI_API_KEY` via [`GenaiClient::from_env`]. LLM Assistant loads the key from DB preferences.
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
use serde::Deserialize;
use tokio::time::{Duration, sleep};

use super::errors::GenaiError;
use super::types::{
    Content, FunctionDeclaration, GenerateContentRequest, GenerateContentResponse,
    GenerationConfig, Role, Tool, ToolConfig,
};
use super::util::{content_answer_text, content_text, merge_content};

/// Default system prompt for the LLM Assistant plugin (skills, tools, multimodal guidance).
pub const ASSISTANT_SYSTEM_PROMPT: &str = r#"You are LLM Assistant inside the Lariv app. You help operators search the public web via Google Programmable Search and read specific pages with the read_webpage tool.

You are a multimodal assistant. You can see, analyze, and process any files, documents, or images attached by the user.

CRITICAL: You have access to various registered skills that help you handle tasks. You MUST check the list of available skills (by calling the list_skills tool) before generating your response to see if an existing skill is suited to the user's request. Checking for available skills is your absolute highest priority.

To properly use a skill, you first need its name, you can get the name using the list_skills tool, then use get_skill_detail to get the content. Content will describe what you need to do with. It will often list rules or a sequence of steps to follow. It may often refer to files, which you can read with read_file. The references files will be listed in the Files section of the response from get_skill_detail.

Even if the task may seem trivial, if a skill might seem to provide some additional information about the task, then you should check the instructions via get_skill_detail.

To change an existing skill, call get_skill_detail first, then edit_skill with the current name and only the fields that should change (new_name, description, content, file_paths).

NOTE: list_skills doesn't give the instructions that are contained in the skill. You NEED to call get_skill_detail to get the instructions.

After google_search, use read_webpage on a result URL when you need that page's full content rather than the snippet.

For normal answers (questions, explanations, summaries after tool results), reply in plain text or markdown.

If a tool response includes an error, explain it briefly and suggest a fix."#;

const GEMINI_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";
const STREAM_MAX_ATTEMPTS: u32 = 4;
const DEFAULT_MAX_OUTPUT_TOKENS: i32 = 8192;

/// HTTP client for Gemini `generateContent`, `streamGenerateContent`, and `models.list`.
#[derive(Clone)]
pub struct GenaiClient {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

impl GenaiClient {
    /// Construct a client with explicit API key and model name.
    ///
    /// Empty keys are allowed at construction (LLM Assistant loads the key from DB
    /// preferences per request). Requests still fail with [`GenaiError::MissingApiKey`].
    pub fn new(api_key: String, model: String) -> Self {
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

    /// Clone this client with a different API key (same HTTP client and model).
    pub fn with_api_key(&self, api_key: impl Into<String>) -> Self {
        Self {
            http: self.http.clone(),
            api_key: api_key.into(),
            model: self.model.clone(),
        }
    }

    /// Clone this client with a different model (same HTTP client and API key).
    pub fn with_model(&self, model: impl Into<String>) -> Self {
        Self {
            http: self.http.clone(),
            api_key: self.api_key.clone(),
            model: model.into(),
        }
    }

    /// Configured model identifier (e.g. `"gemini-2.0-flash"`).
    pub fn model(&self) -> &str {
        &self.model
    }

    /// List models that support `generateContent`.
    ///
    /// Returns `(id, display_name)` pairs. `id` is the short name used in
    /// `models/{id}:generateContent` (the `models/` prefix is stripped).
    pub async fn list_generate_content_models(&self) -> Result<Vec<(String, String)>, GenaiError> {
        self.list_models_for_action("generateContent").await
    }

    /// List models that support `embedContent`.
    ///
    /// Returns `(id, display_name)` pairs. `id` is the short name used in
    /// `models/{id}:embedContent` (the `models/` prefix is stripped).
    pub async fn list_embed_content_models(&self) -> Result<Vec<(String, String)>, GenaiError> {
        self.list_models_for_action("embedContent").await
    }

    async fn list_models_for_action(
        &self,
        action: &str,
    ) -> Result<Vec<(String, String)>, GenaiError> {
        if self.api_key.trim().is_empty() {
            return Err(GenaiError::MissingApiKey);
        }
        let mut page_token = String::new();
        let mut out = Vec::new();
        loop {
            let mut req = self
                .http
                .get(format!("{GEMINI_BASE}/models"))
                .query(&[("key", self.api_key.as_str()), ("pageSize", "100")]);
            if !page_token.is_empty() {
                req = req.query(&[("pageToken", page_token.as_str())]);
            }
            let resp = req.send().await?;
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(GenaiError::Api {
                    status: status.as_u16(),
                    body: text,
                });
            }
            let parsed: ListModelsResponse =
                serde_json::from_str(&text).map_err(|e| GenaiError::Json(e.to_string()))?;
            for model in parsed.models {
                if let Some(choice) = listed_model_choice(model, action) {
                    out.push(choice);
                }
            }
            page_token = parsed.next_page_token;
            if page_token.is_empty() {
                break;
            }
        }
        out.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
        Ok(out)
    }

    fn request_body(
        contents: Vec<Content>,
        system_instruction: Option<Content>,
        max_output_tokens: i32,
        tool_decls: &[FunctionDeclaration],
    ) -> GenerateContentRequest {
        Self::request_body_with_config(
            contents,
            system_instruction,
            GenerationConfig {
                temperature: Some(0.35),
                max_output_tokens: Some(max_output_tokens.max(1)),
                response_mime_type: None,
                response_schema: None,
                response_json_schema: None,
            },
            tool_decls,
        )
    }

    fn request_body_with_config(
        contents: Vec<Content>,
        system_instruction: Option<Content>,
        generation_config: GenerationConfig,
        tool_decls: &[FunctionDeclaration],
    ) -> GenerateContentRequest {
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
            generation_config: Some(generation_config),
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
            Some(Content::text(Role::User, system_prompt))
        };
        let content = self
            .generate_content_with_system(
                vec![Content::text(Role::User, user_prompt)],
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

    /// One-shot JSON generation with a response schema (`responseMimeType: application/json`).
    pub async fn generate_json(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        schema: serde_json::Value,
        max_output_tokens: i32,
    ) -> Result<String, GenaiError> {
        if self.api_key.trim().is_empty() {
            return Err(GenaiError::MissingApiKey);
        }
        let url = format!(
            "{GEMINI_BASE}/models/{}:generateContent?key={}",
            self.model, self.api_key
        );
        let system = if system_prompt.trim().is_empty() {
            None
        } else {
            Some(Content::text(Role::User, system_prompt))
        };
        let body = Self::request_body_with_config(
            vec![Content::text(Role::User, user_prompt)],
            system,
            GenerationConfig {
                temperature: Some(0.2),
                max_output_tokens: Some(max_output_tokens.max(1)),
                response_mime_type: Some("application/json".into()),
                // Prefer OpenAPI `responseSchema` on the REST API; it is more widely
                // applied than `responseJsonSchema` across model versions.
                response_schema: Some(schema),
                response_json_schema: None,
            },
            &[],
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
        // Skip model "thought" parts — concatenating them breaks JSON parsing.
        let text = content_answer_text(&content);
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
            Some(Content::text(Role::User, ASSISTANT_SYSTEM_PROMPT)),
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
        let body = Self::request_body(contents, system_instruction, max_output_tokens, tool_decls);

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
            Some(Content::text(Role::User, ASSISTANT_SYSTEM_PROMPT)),
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
                    let parsed: GenerateContentResponse =
                        serde_json::from_str(data).map_err(|e| GenaiError::Json(e.to_string()))?;
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListModelsResponse {
    #[serde(default)]
    models: Vec<ListedModel>,
    #[serde(default)]
    next_page_token: String,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListedModel {
    #[serde(default)]
    name: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    supported_generation_methods: Vec<String>,
    /// Newer Gemini list responses use `supportedActions` instead of
    /// `supportedGenerationMethods`.
    #[serde(default)]
    supported_actions: Vec<String>,
}

fn listed_model_choice(model: ListedModel, action: &str) -> Option<(String, String)> {
    let id = model
        .name
        .strip_prefix("models/")
        .unwrap_or(&model.name)
        .trim()
        .to_string();
    if id.is_empty() {
        return None;
    }
    if !supports_action(&model, action) {
        return None;
    }
    let label = if model.display_name.trim().is_empty() {
        id.clone()
    } else {
        model.display_name
    };
    Some((id, label))
}

fn supports_action(model: &ListedModel, action: &str) -> bool {
    let methods = model
        .supported_generation_methods
        .iter()
        .chain(model.supported_actions.iter());
    let mut any = false;
    for method in methods {
        any = true;
        if method.eq_ignore_ascii_case(action) {
            return true;
        }
    }
    // Some list payloads omit capability fields; fall back on name heuristics.
    if any {
        return false;
    }
    if action.eq_ignore_ascii_case("generateContent") {
        !id_looks_like_embedder(&model.name)
    } else if action.eq_ignore_ascii_case("embedContent") {
        id_looks_like_embedder(&model.name)
    } else {
        false
    }
}

fn id_looks_like_embedder(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("embed") || lower.contains("aqa") || lower.contains("gecko")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_models_accepts_supported_actions() {
        let json = r#"{
            "models": [
                {
                    "name": "models/gemini-2.5-flash",
                    "displayName": "Gemini 2.5 Flash",
                    "supportedActions": ["generateContent", "countTokens"]
                },
                {
                    "name": "models/text-embedding-004",
                    "displayName": "Text Embedding",
                    "supportedActions": ["embedContent"]
                }
            ]
        }"#;
        let parsed: ListModelsResponse = serde_json::from_str(json).unwrap();
        let generate: Vec<_> = parsed
            .models
            .iter()
            .cloned()
            .filter_map(|m| listed_model_choice(m, "generateContent"))
            .collect();
        let embed: Vec<_> = parsed
            .models
            .into_iter()
            .filter_map(|m| listed_model_choice(m, "embedContent"))
            .collect();
        assert_eq!(
            generate,
            vec![("gemini-2.5-flash".into(), "Gemini 2.5 Flash".into())]
        );
        assert_eq!(
            embed,
            vec![("text-embedding-004".into(), "Text Embedding".into())]
        );
    }

    #[test]
    fn list_models_keeps_gemini_when_capabilities_omitted() {
        let model = ListedModel {
            name: "models/gemini-2.5-pro".into(),
            display_name: "Gemini 2.5 Pro".into(),
            ..Default::default()
        };
        assert_eq!(
            listed_model_choice(model, "generateContent"),
            Some(("gemini-2.5-pro".into(), "Gemini 2.5 Pro".into()))
        );
    }

    #[test]
    fn list_models_keeps_embedder_when_capabilities_omitted() {
        let model = ListedModel {
            name: "models/gemini-embedding-001".into(),
            display_name: "Gemini Embedding".into(),
            ..Default::default()
        };
        assert_eq!(
            listed_model_choice(model, "embedContent"),
            Some(("gemini-embedding-001".into(), "Gemini Embedding".into()))
        );
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
