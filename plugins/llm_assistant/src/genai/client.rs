//! Thin Gemini HTTP client (Interactions API, `models.list`, Files API).
//!
//! [`GenaiClient`] wraps reqwest calls to `generativelanguage.googleapis.com`. Use
//! [`GenaiClient::generate_text`] for one-shot prompts or
//! [`GenaiClient::generate_content`] / [`GenaiClient::stream_generate_content`] for
//! multi-turn LLM Assistant chat with optional function declarations (via
//! `POST /v1beta/interactions`).
//! [`GenaiClient::list_generate_content_models`] / [`GenaiClient::list_embed_content_models`]
//! list models by supported action (`generateContent` / `embedContent`).
//! [`GenaiClient::upload_file`] uploads bytes via the Files API and waits until `ACTIVE`
//! so callers can reference the file with [`crate::genai::FileData`] instead of inline base64.
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
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::{Duration, sleep};

use chrono::{DateTime, Utc};

use super::errors::GenaiError;
use super::interactions::{
    CreateInteractionRequest, Interaction, InteractionGenerationConfig, InteractionSseEvent,
    InteractionTool, StreamAccumulator, contents_to_interaction_input, json_response_format,
    steps_to_model_content, user_content_to_interaction_input,
};
use super::types::{Content, FunctionDeclaration, Role, UsageMetadata};
use super::util::{coerce_json_text, content_answer_text, content_text};

/// Default system prompt for the LLM Assistant plugin (skills, tools, multimodal guidance).
pub const ASSISTANT_SYSTEM_PROMPT: &str = r#"You are LLM Assistant inside the Lariv app. You help operators search the public web via Google Programmable Search and read specific pages with the read_webpage tool.

You are a multimodal assistant. You can see, analyze, and process any files, documents, or images attached by the user. To include a virtual file (VNode) in the conversation, call attach_vnode_to_context with its path or vnode_id. That tool places the document into this turn as a file_data part so you can read it. Files already uploaded to Gemini are reused while the Files API cache is still valid; if the tool reports already_in_context, you do not need to attach that file again unless you can no longer see its contents.

CRITICAL: You have access to various registered skills that help you handle tasks. You MUST check the list of available skills (by calling the list_skills tool) before generating your response to see if an existing skill is suited to the user's request. Checking for available skills is your absolute highest priority.

To properly use a skill, you first need its name, you can get the name using the list_skills tool, then use get_skill_detail to get the content. Content will describe what you need to do with. It will often list rules or a sequence of steps to follow. It may often refer to files, which will be listed in the Files section of the response from get_skill_detail.

When calling a Rune environment function from run_rune / run_rune_file, you MUST know its schema (arguments and return type). If you do not, call get_rune_env with that identifier first. Do not guess argument names, types, or return shapes. Use list_rune_env to discover names, then get_rune_env for each name you will call.

Even if the task may seem trivial, if a skill might seem to provide some additional information about the task, then you should check the instructions via get_skill_detail.

To create a new skill, call create_skill with name, content (instructions), and optional description and file_paths.

To change an existing skill, call get_skill_detail first, then edit_skill with the current name and only the fields that should change (new_name, description, content, file_paths).

NOTE: list_skills doesn't give the instructions that are contained in the skill. You NEED to call get_skill_detail to get the instructions.

After google_search, use read_webpage on a result URL when you need that page's full content rather than the snippet.

To save a public file into the virtual filesystem, call download_file with the http(s) URL and the destination VNode path. Then call attach_vnode_to_context (or read_vnode for UTF-8 text) if you need to inspect the file.

To read a tabular VNode (csv, tsv, json, ndjson, parquet, ipc, avro, or a spreadsheet: xls, xlsx, xlsm, xlsb, xla, xlam, ods), call read_table, query_table, or list_sheets from run_rune. Call get_rune_env for the schema before using one. query_table runs one Polars SQL statement against the table named data.

To create an .xlsx workbook, use the xlsx Rune module from run_rune (xlsx::Workbook, Worksheet, Format, Chart, and the other rust_xlsxwriter types). Call xlsx_api(()) before using it and follow that catalog. Workbook.save writes a new VNode and fails if that path already exists.

When the user asks about the current date or time, call get_current_datetime. Do not guess. Pass an IANA timezone when the user names one; otherwise UTC is used.

For normal answers (questions, explanations, summaries after tool results), reply in plain text or markdown.

If a tool response includes an error, explain it briefly and suggest a fix."#;

const GEMINI_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";
const GEMINI_UPLOAD_BASE: &str = "https://generativelanguage.googleapis.com/upload/v1beta";
const STREAM_MAX_ATTEMPTS: u32 = 4;
const DEFAULT_MAX_OUTPUT_TOKENS: i32 = 8192;
const FILE_POLL_ATTEMPTS: u32 = 40;
const FILE_POLL_INITIAL_MS: u64 = 250;

/// A file uploaded through the Gemini Files API and ready for `file_data` parts.
#[derive(Debug, Clone)]
pub struct UploadedGeminiFile {
    /// Resource name (`files/…`).
    pub name: String,
    /// Absolute URI for [`crate::genai::FileData::file_uri`].
    pub uri: String,
    pub mime_type: String,
    pub display_name: String,
    /// Processing state (`PROCESSING`, `ACTIVE`, `FAILED`, …).
    pub state: String,
    /// Files API expiry, when the resource includes `expirationTime`.
    pub expiration_time: Option<DateTime<Utc>>,
}

impl UploadedGeminiFile {
    pub fn state_is_active(&self) -> bool {
        self.state.eq_ignore_ascii_case("ACTIVE")
    }

    pub fn state_is_failed(&self) -> bool {
        self.state.eq_ignore_ascii_case("FAILED")
    }
}

/// Timing breakdown for [`GenaiClient::upload_file_timed`] (latency debugging).
#[derive(Debug, Clone, Copy, Default)]
pub struct UploadFileTiming {
    /// Resumable session start HTTP round-trip.
    pub start_ms: u128,
    /// Raw bytes upload HTTP round-trip.
    pub bytes_ms: u128,
    /// Time spent polling until `ACTIVE` (0 if already active).
    pub poll_ms: u128,
}

impl UploadFileTiming {
    pub fn total_ms(self) -> u128 {
        self.start_ms
            .saturating_add(self.bytes_ms)
            .saturating_add(self.poll_ms)
    }
}

/// Merged model turn plus optional [`UsageMetadata`] from Interactions.
#[derive(Debug, Clone)]
pub struct GenerateContentResult {
    pub content: Content,
    pub usage: Option<UsageMetadata>,
    /// Server interaction id when `store=true` (for `previous_interaction_id`).
    pub interaction_id: Option<String>,
}

/// One-shot text plus optional [`UsageMetadata`] from Interactions.
#[derive(Debug, Clone)]
pub struct GenerateTextResult {
    pub text: String,
    pub usage: Option<UsageMetadata>,
}

/// HTTP client for Gemini Interactions, Files API, and `models.list`.
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

    /// Upload bytes via the Gemini Files API and wait until the file is `ACTIVE`.
    ///
    /// Returns a URI suitable for [`crate::genai::FileData`] so generate/stream calls can
    /// reference the file without embedding base64 in every request body.
    pub async fn upload_file(
        &self,
        display_name: &str,
        mime_type: &str,
        bytes: &[u8],
    ) -> Result<UploadedGeminiFile, GenaiError> {
        Ok(self
            .upload_file_timed(display_name, mime_type, bytes)
            .await?
            .0)
    }

    /// Like [`Self::upload_file`], but also returns phase timings for latency debugging.
    pub async fn upload_file_timed(
        &self,
        display_name: &str,
        mime_type: &str,
        bytes: &[u8],
    ) -> Result<(UploadedGeminiFile, UploadFileTiming), GenaiError> {
        if self.api_key.trim().is_empty() {
            return Err(GenaiError::MissingApiKey);
        }
        let mime = if mime_type.trim().is_empty() {
            "application/octet-stream"
        } else {
            mime_type.trim()
        };
        let (uploaded, mut timing) = self
            .upload_file_resumable_timed(display_name, mime, bytes)
            .await?;
        let poll_start = std::time::Instant::now();
        let ready = self.wait_file_active(&uploaded).await?;
        timing.poll_ms = poll_start.elapsed().as_millis();
        Ok((ready, timing))
    }

    /// Serialize an Interactions request body and return its JSON byte length.
    ///
    /// Used by latency tests to compare `inline_data` vs `file_data` payload sizes
    /// across simulated tool rounds (no network).
    pub fn generate_request_json_len(
        contents: Vec<Content>,
        max_output_tokens: i32,
        tool_decls: &[FunctionDeclaration],
    ) -> Result<usize, GenaiError> {
        let body = Self::interaction_request(
            "model",
            contents_to_interaction_input(&contents),
            Some(ASSISTANT_SYSTEM_PROMPT),
            max_output_tokens,
            tool_decls,
            None,
            None,
            false,
            false,
        );
        let bytes = serde_json::to_vec(&body).map_err(|e| GenaiError::Json(e.to_string()))?;
        Ok(bytes.len())
    }

    async fn upload_file_resumable_timed(
        &self,
        display_name: &str,
        mime_type: &str,
        bytes: &[u8],
    ) -> Result<(UploadedGeminiFile, UploadFileTiming), GenaiError> {
        let mut timing = UploadFileTiming::default();
        let start_url = format!("{GEMINI_UPLOAD_BASE}/files?key={}", self.api_key);
        let start_at = std::time::Instant::now();
        let start = self
            .http
            .post(&start_url)
            .header("X-Goog-Upload-Protocol", "resumable")
            .header("X-Goog-Upload-Command", "start")
            .header(
                "X-Goog-Upload-Header-Content-Length",
                bytes.len().to_string(),
            )
            .header("X-Goog-Upload-Header-Content-Type", mime_type)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "file": { "display_name": display_name }
            }))
            .send()
            .await?;
        timing.start_ms = start_at.elapsed().as_millis();
        let start_status = start.status();
        let upload_url = start
            .headers()
            .get("x-goog-upload-url")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        if !start_status.is_success() || upload_url.is_none() {
            let body = start.text().await.unwrap_or_default();
            return Err(GenaiError::Api {
                status: start_status.as_u16(),
                body: if body.is_empty() {
                    "missing X-Goog-Upload-URL".into()
                } else {
                    body
                },
            });
        }
        let upload_url = upload_url.expect("checked above");

        let bytes_at = std::time::Instant::now();
        let upload = self
            .http
            .post(&upload_url)
            .header("Content-Length", bytes.len().to_string())
            .header("X-Goog-Upload-Offset", "0")
            .header("X-Goog-Upload-Command", "upload, finalize")
            .body(bytes.to_vec())
            .send()
            .await?;
        timing.bytes_ms = bytes_at.elapsed().as_millis();
        let status = upload.status();
        let text = upload.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(GenaiError::Api {
                status: status.as_u16(),
                body: text,
            });
        }
        let mut file = parse_file_resource(&text)?;
        if file.mime_type.is_empty() {
            file.mime_type = mime_type.to_string();
        }
        if file.display_name.is_empty() {
            file.display_name = display_name.to_string();
        }
        Ok((file, timing))
    }

    async fn wait_file_active(
        &self,
        uploaded: &UploadedGeminiFile,
    ) -> Result<UploadedGeminiFile, GenaiError> {
        let mut current = uploaded.clone();
        if current.state_is_active() {
            return Ok(current);
        }
        let mut delay_ms = FILE_POLL_INITIAL_MS;
        for _ in 0..FILE_POLL_ATTEMPTS {
            if current.state_is_failed() {
                return Err(GenaiError::FileProcessing(format!(
                    "{} failed processing",
                    current.name
                )));
            }
            sleep(Duration::from_millis(delay_ms)).await;
            current = self.get_file(&current.name).await?;
            if current.state_is_active() {
                return Ok(current);
            }
            delay_ms = (delay_ms.saturating_mul(2)).min(5_000);
        }
        Err(GenaiError::FileProcessing(format!(
            "{} still not ACTIVE after polling (last state={})",
            uploaded.name,
            if current.state.is_empty() {
                "unknown"
            } else {
                &current.state
            }
        )))
    }

    /// Fetch a Files API resource by name (`files/…`) or URI.
    ///
    /// Used to check that a previously uploaded file is still `ACTIVE` before
    /// reusing its URI instead of uploading again.
    pub async fn get_file(&self, name: &str) -> Result<UploadedGeminiFile, GenaiError> {
        let resource = file_resource_name(name);
        let url = format!("{GEMINI_BASE}/{resource}?key={}", self.api_key);
        let resp = self.http.get(&url).send().await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(GenaiError::Api {
                status: status.as_u16(),
                body: text,
            });
        }
        parse_file_resource(&text)
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

    #[allow(clippy::too_many_arguments)]
    fn interaction_request(
        model: &str,
        input: Value,
        system_instruction: Option<&str>,
        max_output_tokens: i32,
        tool_decls: &[FunctionDeclaration],
        previous_interaction_id: Option<String>,
        response_format: Option<Value>,
        include_thoughts: bool,
        stream: bool,
    ) -> CreateInteractionRequest {
        let tools = if tool_decls.is_empty() {
            None
        } else {
            Some(tool_decls.iter().map(InteractionTool::from).collect())
        };
        let mut generation_config = InteractionGenerationConfig {
            max_output_tokens: Some(max_output_tokens.max(1)),
            thinking_level: None,
            thinking_summaries: None,
            tool_choice: if tools.is_some() {
                Some("auto".into())
            } else {
                None
            },
        };
        if include_thoughts {
            generation_config.thinking_summaries = Some("auto".into());
        }
        CreateInteractionRequest {
            model: model.to_string(),
            input,
            system_instruction: system_instruction
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            tools,
            generation_config: Some(generation_config),
            response_format,
            previous_interaction_id: previous_interaction_id
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            store: Some(true),
            stream: stream.then_some(true),
        }
    }

    /// Build Interactions input: full window when bootstrapping, single turn when chaining.
    fn input_for_call(contents: &[Content], previous_interaction_id: Option<&str>) -> Value {
        if previous_interaction_id.is_some() {
            match contents {
                [only] => user_content_to_interaction_input(only),
                _ => contents
                    .last()
                    .map(user_content_to_interaction_input)
                    .unwrap_or(Value::Array(vec![])),
            }
        } else {
            contents_to_interaction_input(contents)
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
        Ok(self
            .generate_text_with_usage(system_prompt, user_prompt, max_output_tokens)
            .await?
            .text)
    }

    /// Like [`Self::generate_text_with_tokens`], but also returns Interactions `usage`.
    pub async fn generate_text_with_usage(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        max_output_tokens: i32,
    ) -> Result<GenerateTextResult, GenaiError> {
        let system = if system_prompt.trim().is_empty() {
            None
        } else {
            Some(system_prompt)
        };
        let generated = self
            .create_interaction(
                vec![Content::text(Role::User, user_prompt)],
                system,
                max_output_tokens,
                &[],
                None,
                None,
                false,
            )
            .await?;
        let text = content_text(&generated.content);
        if text.trim().is_empty() {
            return Err(GenaiError::EmptyResponse);
        }
        Ok(GenerateTextResult {
            text,
            usage: generated.usage,
        })
    }

    /// One-shot JSON generation with Interactions `response_format`.
    ///
    /// Thinking is disabled when the model supports it (`thinking_level: minimal`).
    pub async fn generate_json(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        schema: serde_json::Value,
        max_output_tokens: i32,
    ) -> Result<String, GenaiError> {
        Ok(self
            .generate_json_with_usage(system_prompt, user_prompt, schema, max_output_tokens)
            .await?
            .text)
    }

    /// Like [`Self::generate_json`], but also returns Interactions `usage`.
    pub async fn generate_json_with_usage(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        schema: serde_json::Value,
        max_output_tokens: i32,
    ) -> Result<GenerateTextResult, GenaiError> {
        let system = if system_prompt.trim().is_empty() {
            None
        } else {
            Some(system_prompt)
        };
        let generated = self
            .create_interaction(
                vec![Content::text(Role::User, user_prompt)],
                system,
                max_output_tokens,
                &[],
                None,
                Some(json_response_format(schema)),
                false,
            )
            .await?;
        let text = content_answer_text(&generated.content);
        if text.trim().is_empty() {
            return Err(GenaiError::EmptyResponse);
        }
        Ok(GenerateTextResult {
            text: coerce_json_text(&text).to_string(),
            usage: generated.usage,
        })
    }

    /// Non-streaming Interactions create with the LLM Assistant default system prompt.
    pub async fn generate_content(
        &self,
        contents: Vec<Content>,
        max_output_tokens: i32,
        tool_decls: &[FunctionDeclaration],
    ) -> Result<Content, GenaiError> {
        Ok(self
            .generate_content_with_usage(contents, max_output_tokens, tool_decls, None)
            .await?
            .content)
    }

    /// Non-streaming Interactions create; returns content, usage, and interaction id.
    pub async fn generate_content_with_usage(
        &self,
        contents: Vec<Content>,
        max_output_tokens: i32,
        tool_decls: &[FunctionDeclaration],
        previous_interaction_id: Option<String>,
    ) -> Result<GenerateContentResult, GenaiError> {
        self.create_interaction(
            contents,
            Some(ASSISTANT_SYSTEM_PROMPT),
            max_output_tokens,
            tool_decls,
            previous_interaction_id,
            None,
            false,
        )
        .await
    }

    /// Non-streaming Interactions create with a custom system instruction string.
    pub async fn generate_content_with_system(
        &self,
        contents: Vec<Content>,
        system_instruction: Option<&str>,
        max_output_tokens: i32,
        tool_decls: &[FunctionDeclaration],
    ) -> Result<Content, GenaiError> {
        Ok(self
            .create_interaction(
                contents,
                system_instruction,
                max_output_tokens,
                tool_decls,
                None,
                None,
                false,
            )
            .await?
            .content)
    }

    #[allow(clippy::too_many_arguments)]
    async fn create_interaction(
        &self,
        contents: Vec<Content>,
        system_instruction: Option<&str>,
        max_output_tokens: i32,
        tool_decls: &[FunctionDeclaration],
        previous_interaction_id: Option<String>,
        response_format: Option<Value>,
        include_thoughts: bool,
    ) -> Result<GenerateContentResult, GenaiError> {
        if self.api_key.trim().is_empty() {
            return Err(GenaiError::MissingApiKey);
        }
        let url = format!("{GEMINI_BASE}/interactions");
        let input = Self::input_for_call(&contents, previous_interaction_id.as_deref());
        let disable_thinking = response_format.is_some();
        let mut body = Self::interaction_request(
            &self.model,
            input,
            system_instruction,
            max_output_tokens,
            tool_decls,
            previous_interaction_id,
            response_format,
            include_thoughts,
            false,
        );
        body = self
            .finish_interaction_request(body, disable_thinking, include_thoughts)
            .await;

        let resp = self
            .http
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(GenaiError::Api {
                status: status.as_u16(),
                body: text,
            });
        }

        let parsed: Interaction =
            serde_json::from_str(&text).map_err(|e| GenaiError::Json(e.to_string()))?;
        if let Some(err) = parsed.error {
            return Err(GenaiError::ApiMessage {
                message: err.message,
            });
        }
        let usage = parsed.usage.as_ref().map(|u| u.to_usage_metadata());
        let content = steps_to_model_content(&parsed.steps);
        if content.parts.is_empty() {
            if let Some(text) = parsed.output_text.filter(|t| !t.trim().is_empty()) {
                return Ok(GenerateContentResult {
                    content: Content::text(Role::Model, text),
                    usage,
                    interaction_id: (!parsed.id.is_empty()).then_some(parsed.id),
                });
            }
            return Err(GenaiError::EmptyResponse);
        }
        Ok(GenerateContentResult {
            content,
            usage,
            interaction_id: (!parsed.id.is_empty()).then_some(parsed.id),
        })
    }

    /// Streaming Interactions create (SSE).
    ///
    /// Calls `on_chunk` with progressively merged content; returns the final merged
    /// [`Content`]. Retries on quota / 429 errors up to four attempts.
    pub async fn stream_generate_content<F>(
        &self,
        contents: Vec<Content>,
        max_output_tokens: i32,
        tool_decls: &[FunctionDeclaration],
        on_chunk: F,
    ) -> Result<Content, GenaiError>
    where
        F: FnMut(&Content) + Send,
    {
        Ok(self
            .stream_generate_content_with_usage(
                contents,
                max_output_tokens,
                tool_decls,
                false,
                None,
                on_chunk,
            )
            .await?
            .content)
    }

    /// Like [`Self::stream_generate_content`], but also returns usage and interaction id.
    ///
    /// `include_thoughts` asks for `thinking_summaries: auto`.
    /// `previous_interaction_id` chains a turn; when set, `contents` is only the new input.
    pub async fn stream_generate_content_with_usage<F>(
        &self,
        contents: Vec<Content>,
        max_output_tokens: i32,
        tool_decls: &[FunctionDeclaration],
        include_thoughts: bool,
        previous_interaction_id: Option<String>,
        mut on_chunk: F,
    ) -> Result<GenerateContentResult, GenaiError>
    where
        F: FnMut(&Content) + Send,
    {
        if self.api_key.trim().is_empty() {
            return Err(GenaiError::MissingApiKey);
        }
        let url = format!("{GEMINI_BASE}/interactions");
        let input = Self::input_for_call(&contents, previous_interaction_id.as_deref());
        let mut body = Self::interaction_request(
            &self.model,
            input,
            Some(ASSISTANT_SYSTEM_PROMPT),
            max_output_tokens,
            tool_decls,
            previous_interaction_id,
            None,
            include_thoughts,
            true,
        );
        body = self
            .finish_interaction_request(body, false, include_thoughts)
            .await;

        let mut last_err = None;
        for attempt in 0..STREAM_MAX_ATTEMPTS {
            if attempt > 0 {
                let backoff_ms = 500u64.saturating_mul(1u64 << (attempt - 1)).min(12_000);
                sleep(Duration::from_millis(backoff_ms)).await;
            }
            match self.stream_once(&url, &body, &mut on_chunk).await {
                Ok(result) => return Ok(result),
                Err(e) if attempt + 1 < STREAM_MAX_ATTEMPTS && is_retryable_quota(&e) => {
                    tracing::warn!(attempt, error = %e, "genai: retrying stream");
                    last_err = Some(e);
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_err.unwrap_or(GenaiError::EmptyResponse))
    }

    /// Count tokens for `contents` plus the LLM Assistant system prompt.
    pub async fn count_tokens(&self, contents: Vec<Content>) -> Result<u32, GenaiError> {
        self.count_tokens_with_system(
            contents,
            Some(Content::text(Role::User, ASSISTANT_SYSTEM_PROMPT)),
        )
        .await
    }

    /// `models/{id}:countTokens` — used to fill the composer meter for stored sessions.
    pub async fn count_tokens_with_system(
        &self,
        contents: Vec<Content>,
        system_instruction: Option<Content>,
    ) -> Result<u32, GenaiError> {
        if self.api_key.trim().is_empty() {
            return Err(GenaiError::MissingApiKey);
        }
        if contents.is_empty() {
            return Ok(0);
        }
        let url = format!(
            "{GEMINI_BASE}/models/{}:countTokens?key={}",
            self.model, self.api_key
        );
        let body = CountTokensRequest {
            contents,
            system_instruction,
        };
        let resp = self.http.post(&url).json(&body).send().await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(GenaiError::Api {
                status: status.as_u16(),
                body: text,
            });
        }
        let parsed: CountTokensResponse =
            serde_json::from_str(&text).map_err(|e| GenaiError::Json(e.to_string()))?;
        if parsed.total_tokens < 0 {
            Ok(0)
        } else {
            Ok(parsed.total_tokens as u32)
        }
    }

    /// `models/{id}` `inputTokenLimit` for the configured chat model.
    pub async fn model_input_token_limit(&self) -> Result<u32, GenaiError> {
        let features = self.model_features().await?;
        if features.input_token_limit > 0 {
            Ok(features.input_token_limit as u32)
        } else {
            Err(GenaiError::Json("model inputTokenLimit missing".into()))
        }
    }

    /// Apply thinking controls using `models.get` `thinking`, then the model id.
    async fn finish_interaction_request(
        &self,
        mut body: CreateInteractionRequest,
        disable_thinking: bool,
        include_thoughts: bool,
    ) -> CreateInteractionRequest {
        let wants_thinking = disable_thinking || include_thoughts;
        let features = if wants_thinking {
            self.model_features().await.ok()
        } else {
            None
        };
        let Some(config) = body.generation_config.as_mut() else {
            return body;
        };
        if !supports_thinking(&self.model, features.as_ref()) {
            config.thinking_level = None;
            config.thinking_summaries = None;
            return body;
        }
        if disable_thinking {
            config.thinking_level = Some("minimal".into());
            config.thinking_summaries = Some("none".into());
        } else if include_thoughts {
            config.thinking_summaries = Some("auto".into());
        }
        body
    }

    async fn model_features(&self) -> Result<ModelFeatures, GenaiError> {
        if self.api_key.trim().is_empty() {
            return Err(GenaiError::MissingApiKey);
        }
        let key = model_cache_key(&self.model);
        if let Some(hit) = cached_model_features(&key) {
            return Ok(hit);
        }
        let url = format!("{GEMINI_BASE}/models/{}?key={}", self.model, self.api_key);
        let resp = self.http.get(&url).send().await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(GenaiError::Api {
                status: status.as_u16(),
                body: text,
            });
        }
        let parsed: ModelResource =
            serde_json::from_str(&text).map_err(|e| GenaiError::Json(e.to_string()))?;
        let features = ModelFeatures {
            input_token_limit: parsed.input_token_limit,
            thinking: parsed.thinking,
        };
        store_model_features(key, features.clone());
        Ok(features)
    }

    async fn stream_once<F>(
        &self,
        url: &str,
        body: &CreateInteractionRequest,
        on_chunk: &mut F,
    ) -> Result<GenerateContentResult, GenaiError>
    where
        F: FnMut(&Content) + Send,
    {
        let resp = self
            .http
            .post(url)
            .header("x-goog-api-key", &self.api_key)
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(GenaiError::Api {
                status: status.as_u16(),
                body: text,
            });
        }

        let mut acc = StreamAccumulator::default();
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
                    let parsed: InteractionSseEvent =
                        serde_json::from_str(data).map_err(|e| GenaiError::Json(e.to_string()))?;
                    if let Some(ix) = &parsed.interaction
                        && let Some(err) = &ix.error
                    {
                        return Err(GenaiError::ApiMessage {
                            message: err.message.clone(),
                        });
                    }
                    acc.on_event(&parsed);
                    let content = acc.content();
                    if !content.parts.is_empty() {
                        on_chunk(&content);
                    }
                }
            }
        }
        let trailing = buffer.trim();
        if let Some(data) = trailing.strip_prefix("data:") {
            let data = data.trim();
            if !data.is_empty() && data != "[DONE]" {
                let parsed: InteractionSseEvent =
                    serde_json::from_str(data).map_err(|e| GenaiError::Json(e.to_string()))?;
                acc.on_event(&parsed);
                let content = acc.content();
                if !content.parts.is_empty() {
                    on_chunk(&content);
                }
            }
        }

        let result = acc.into_result();
        if result.content.parts.is_empty() {
            return Err(GenaiError::EmptyResponse);
        }
        Ok(GenerateContentResult {
            content: result.content,
            usage: result.usage,
            interaction_id: (!result.id.is_empty()).then_some(result.id),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileResourceWire {
    #[serde(default)]
    name: String,
    #[serde(default)]
    uri: String,
    #[serde(default)]
    mime_type: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    expiration_time: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileUploadEnvelope {
    file: Option<FileResourceWire>,
}

fn parse_file_resource(text: &str) -> Result<UploadedGeminiFile, GenaiError> {
    if let Ok(env) = serde_json::from_str::<FileUploadEnvelope>(text) {
        if let Some(file) = env.file {
            return file_from_wire(file);
        }
    }
    let file: FileResourceWire =
        serde_json::from_str(text).map_err(|e| GenaiError::Json(e.to_string()))?;
    file_from_wire(file)
}

fn file_from_wire(file: FileResourceWire) -> Result<UploadedGeminiFile, GenaiError> {
    if file.uri.trim().is_empty() && file.name.trim().is_empty() {
        return Err(GenaiError::Json(
            "Gemini file response missing uri/name".into(),
        ));
    }
    let name = if file.name.trim().is_empty() {
        file_resource_name(&file.uri)
    } else {
        file.name
    };
    let uri = if file.uri.trim().is_empty() {
        format!("{GEMINI_BASE}/{name}")
    } else {
        file.uri
    };
    Ok(UploadedGeminiFile {
        name,
        uri,
        mime_type: file.mime_type,
        display_name: file.display_name,
        state: file.state,
        expiration_time: parse_expiration_time(&file.expiration_time),
    })
}

fn parse_expiration_time(raw: &str) -> Option<DateTime<Utc>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    DateTime::parse_from_rfc3339(trimmed)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn file_resource_name(name_or_uri: &str) -> String {
    if let Some(id) = name_or_uri.strip_prefix("files/") {
        return format!("files/{id}");
    }
    if let Some(id) = name_or_uri.rsplit("/files/").next() {
        if !id.is_empty() && !id.contains('/') {
            return format!("files/{id}");
        }
    }
    format!("files/{}", name_or_uri.trim_start_matches('/'))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListModelsResponse {
    #[serde(default)]
    models: Vec<ListedModel>,
    #[serde(default)]
    next_page_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CountTokensRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<Content>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CountTokensResponse {
    #[serde(default)]
    total_tokens: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelResource {
    #[serde(default)]
    #[allow(dead_code)]
    name: String,
    #[serde(default)]
    input_token_limit: i32,
    /// Official `Model.thinking`: whether this model accepts `thinkingConfig`.
    #[serde(default)]
    thinking: Option<bool>,
}

#[derive(Debug, Clone)]
struct ModelFeatures {
    input_token_limit: i32,
    thinking: Option<bool>,
}

fn model_feature_cache()
-> &'static std::sync::Mutex<std::collections::HashMap<String, ModelFeatures>> {
    static CACHE: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, ModelFeatures>>,
    > = std::sync::OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn model_cache_key(model: &str) -> String {
    let trimmed = model.trim();
    let id = trimmed.strip_prefix("models/").unwrap_or(trimmed);
    id.to_ascii_lowercase()
}

fn cached_model_features(key: &str) -> Option<ModelFeatures> {
    model_feature_cache()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .get(key)
        .cloned()
}

fn store_model_features(key: String, features: ModelFeatures) {
    model_feature_cache()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .insert(key, features);
}

fn supports_thinking(model: &str, features: Option<&ModelFeatures>) -> bool {
    match features.and_then(|features| features.thinking) {
        Some(thinking) => thinking,
        None => model_id_supports_thinking(model),
    }
}

/// Used when `models.get` omits `thinking` or the lookup fails.
fn model_id_supports_thinking(model: &str) -> bool {
    let id = model_cache_key(model);
    if id.contains("thinking") || id.starts_with("gemma-4") {
        return true;
    }
    if id.starts_with("gemma") {
        return false;
    }
    match gemini_generation(&id) {
        Some(major) if major >= 3 => true,
        Some(2) => id.starts_with("gemini-2.5"),
        _ => false,
    }
}

/// Leading generation of a `gemini-*` id. `gemini-2.5-flash` is 2, `gemini-4-argon` is 4.
fn gemini_generation(id: &str) -> Option<u32> {
    let rest = id.strip_prefix("gemini-")?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
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
    fn parse_file_upload_envelope() {
        let json = r#"{
            "file": {
                "name": "files/abc123",
                "uri": "https://generativelanguage.googleapis.com/v1beta/files/abc123",
                "mimeType": "application/pdf",
                "displayName": "invoice.pdf",
                "state": "ACTIVE"
            }
        }"#;
        let file = parse_file_resource(json).expect("parse");
        assert_eq!(file.name, "files/abc123");
        assert!(file.uri.ends_with("/files/abc123"));
        assert_eq!(file.mime_type, "application/pdf");
        assert_eq!(file.display_name, "invoice.pdf");
        assert!(file.state_is_active());
    }

    #[test]
    fn parse_file_get_top_level() {
        let json = r#"{
            "name": "files/xyz",
            "uri": "https://generativelanguage.googleapis.com/v1beta/files/xyz",
            "mimeType": "image/png",
            "state": "PROCESSING"
        }"#;
        let file = parse_file_resource(json).expect("parse");
        assert_eq!(file.name, "files/xyz");
        assert!(!file.state_is_active());
        assert!(!file.state_is_failed());
        assert!(file.expiration_time.is_none());
    }

    #[test]
    fn parse_file_expiration_time() {
        let json = r#"{
            "name": "files/abc123",
            "uri": "https://generativelanguage.googleapis.com/v1beta/files/abc123",
            "mimeType": "application/pdf",
            "state": "ACTIVE",
            "expirationTime": "2026-09-10T12:00:00Z"
        }"#;
        let file = parse_file_resource(json).expect("parse");
        let exp = file.expiration_time.expect("expiration");
        assert_eq!(exp.to_rfc3339(), "2026-09-10T12:00:00+00:00");
    }

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

    #[test]
    fn supports_thinking_respects_model_flag() {
        let features = ModelFeatures {
            input_token_limit: 8192,
            thinking: Some(false),
        };
        assert!(!supports_thinking("gemma-3-27b-it", Some(&features)));
        let features = ModelFeatures {
            input_token_limit: 8192,
            thinking: Some(true),
        };
        assert!(supports_thinking("gemini-2.5-flash", Some(&features)));
    }

    #[test]
    fn omitted_thinking_flag_uses_model_id() {
        let features = ModelFeatures {
            input_token_limit: 8192,
            thinking: None,
        };
        assert!(!supports_thinking("gemma-2-9b-it", Some(&features)));
        assert!(supports_thinking("gemini-2.5-flash", Some(&features)));
        assert!(supports_thinking("gemini-4-argon", Some(&features)));
        assert!(supports_thinking("gemma-4-26b-a4b-it", Some(&features)));
    }

    #[test]
    fn interaction_request_is_snake_case_and_stores() {
        let body = GenaiClient::interaction_request(
            "gemini-2.5-flash",
            serde_json::json!("hello"),
            Some("sys"),
            1024,
            &[],
            Some("int_1".into()),
            None,
            true,
            true,
        );
        let v = serde_json::to_value(&body).expect("json");
        assert_eq!(v["model"], "gemini-2.5-flash");
        assert_eq!(v["system_instruction"], "sys");
        assert_eq!(v["previous_interaction_id"], "int_1");
        assert_eq!(v["store"], true);
        assert_eq!(v["stream"], true);
        assert_eq!(v["generation_config"]["max_output_tokens"], 1024);
        assert_eq!(v["generation_config"]["thinking_summaries"], "auto");
        assert!(v.get("temperature").is_none());
    }

    #[test]
    fn input_for_call_chains_last_user_only() {
        let contents = vec![
            Content::text(Role::User, "Hi"),
            Content::text(Role::Model, "Hello"),
            Content::text(Role::User, "Name?"),
        ];
        let chained = GenaiClient::input_for_call(&contents, Some("int_1"));
        assert_eq!(chained["type"], "text");
        assert_eq!(chained["text"], "Name?");
        let bootstrap = GenaiClient::input_for_call(&contents, None);
        assert!(bootstrap.as_array().unwrap().len() >= 3);
    }

    #[test]
    fn model_resource_reads_thinking_flag() {
        let parsed: ModelResource = serde_json::from_str(
            r#"{"name":"models/gemma-3-27b-it","inputTokenLimit":8192,"thinking":false}"#,
        )
        .expect("model");
        assert_eq!(parsed.thinking, Some(false));
    }

    #[test]
    fn model_resource_reads_input_token_limit() {
        let parsed: ModelResource = serde_json::from_str(
            r#"{"name":"models/gemini-2.5-flash","inputTokenLimit":1048576,"outputTokenLimit":65536}"#,
        )
        .unwrap();
        assert_eq!(parsed.input_token_limit, 1_048_576);
    }

    #[test]
    fn count_tokens_response_reads_total() {
        let parsed: CountTokensResponse = serde_json::from_str(r#"{"totalTokens":2048}"#).unwrap();
        assert_eq!(parsed.total_tokens, 2048);
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
