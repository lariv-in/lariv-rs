//! Gemini REST Content/Part shapes (camelCase), enough for persist + generateContent.
//!
//! Serde types matching the [Generative Language API](https://ai.google.dev/api) wire format.
//! Used for chat history persistence, request bodies, and streaming response parsing.
//!
//! # Core message types
//!
//! - [`Content`] / [`Part`] — multi-part messages (text, inline files, function calls)
//! - [`FunctionDeclaration`] / [`Tool`] / [`ToolConfig`] — function calling schema
//! - [`GenerateContentRequest`] / [`GenerateContentResponse`] — API envelope
//!
//! # Roles
//!
//! - [`Role`] — user/model turn roles on [`Content::role`]

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

/// Producer of a [`Content`] turn. Gemini wire values are `"user"` and `"model"`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Role {
    User,
    #[default]
    Model,
}

impl Role {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Model => "model",
        }
    }

    /// Parse a stored or wire role.
    ///
    /// Empty, `"model"`, and `"assistant"` map to [`Role::Model`].
    pub fn parse(raw: &str) -> Self {
        match raw.trim() {
            s if s.eq_ignore_ascii_case("user") => Self::User,
            _ => Self::Model,
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Role {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Role {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(Self::parse(&raw))
    }
}

/// One message in a Gemini conversation (role + ordered parts).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    #[serde(default)]
    pub role: Role,
    #[serde(default)]
    pub parts: Vec<Part>,
}

impl Content {
    /// Build a single-text-part message.
    ///
    /// # Examples
    ///
    /// ```rust ignore
    /// let msg = Content::text(Role::User, "Hello, model!");
    /// ```
    pub fn text(role: Role, text: impl Into<String>) -> Self {
        Self {
            role,
            parts: vec![Part {
                text: Some(text.into()),
                ..Default::default()
            }],
        }
    }
}

/// One part of a [`Content`] message (text, media, function call/response, etc.).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<Blob>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<FileData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_response: Option<FunctionResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable_code: Option<ExecutableCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_execution_result: Option<CodeExecutionResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call: Option<ToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_response: Option<ToolResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_resolution: Option<PartMediaResolution>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub thought: bool,
    /// Base64-encoded thought signature (Gemini wire format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_metadata: Option<VideoMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_metadata: Option<Value>,
    /// Filename / label for UI and persistence. Not a Gemini wire field —
    /// `displayName` on `inline_data` / `file_data` is Vertex-only and the
    /// Gemini Developer API rejects it with HTTP 400.
    #[serde(skip)]
    pub display_name: String,
    /// Filesystem VNode id when this part came from a VNode attachment.
    /// Local-only (not a Gemini wire field).
    #[serde(skip)]
    pub vnode_id: Option<i64>,
}

/// Gemini Developer API `Blob` (`inline_data`). Wire fields are `mimeType` + `data` only.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Blob {
    #[serde(default)]
    pub mime_type: String,
    /// Base64-encoded bytes on the wire; decoded when persisting.
    #[serde(default)]
    pub data: String,
}

/// Gemini Developer API `FileData`. Wire fields are `fileUri` + `mimeType` only.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileData {
    #[serde(default)]
    pub file_uri: String,
    #[serde(default)]
    pub mime_type: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCall {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub will_continue: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub will_continue: Option<bool>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub scheduling: String,
    #[serde(default, skip_serializing_if = "String::is_empty", rename = "id")]
    pub function_response_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<FunctionResponsePart>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionResponsePart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<FunctionResponseBlob>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<FunctionResponseFileData>,
    /// Filename / label for UI and persistence. Not a Gemini wire field.
    #[serde(skip)]
    pub display_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionResponseBlob {
    #[serde(default)]
    pub mime_type: String,
    #[serde(default)]
    pub data: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionResponseFileData {
    #[serde(default)]
    pub file_uri: String,
    #[serde(default)]
    pub mime_type: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutableCode {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub language: String,
    #[serde(default, skip_serializing_if = "String::is_empty", rename = "id")]
    pub executable_code_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeExecutionResult {
    #[serde(default)]
    pub outcome: String,
    #[serde(default)]
    pub output: String,
    #[serde(default, skip_serializing_if = "String::is_empty", rename = "id")]
    pub executable_code_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    #[serde(default, skip_serializing_if = "String::is_empty", rename = "id")]
    pub tool_call_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tool_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResponse {
    #[serde(default, skip_serializing_if = "String::is_empty", rename = "id")]
    pub tool_call_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tool_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartMediaResolution {
    #[serde(default)]
    pub level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_tokens: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadata {
    /// Duration as seconds string or number on wire; we store nanoseconds in DB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_offset: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_offset: Option<Value>,
}

/// Gemini function declaration for tool / function calling.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionDeclaration {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

/// Tool wrapper holding function declarations (sent in [`GenerateContentRequest::tools`]).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCallingConfig {
    /// Gemini mode: `"AUTO"`, `"ANY"`, `"NONE"`.
    pub mode: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfig {
    pub function_calling_config: FunctionCallingConfig,
}

impl ToolConfig {
    /// Function-calling mode `"AUTO"` (model decides when to invoke tools).
    pub fn auto() -> Self {
        Self {
            function_calling_config: FunctionCallingConfig {
                mode: "AUTO".into(),
            },
        }
    }
}

/// Request body for `generateContent` / `streamGenerateContent`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
    /// When set (e.g. `"application/json"`), Gemini returns JSON-only output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,
    /// OpenAPI-subset schema enforced with [`Self::response_mime_type`] `"application/json"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<serde_json::Value>,
    /// JSON Schema alternative to [`Self::response_schema`] (omit one when setting the other).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_json_schema: Option<serde_json::Value>,
    /// Gemini 2.5+ thinking controls. Set `thinking_budget: 0` to disable thinking
    /// (required for reliable short structured JSON on Flash).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<ThinkingConfig>,
}

/// Gemini 2.5 `thinkingConfig` (Flash supports disabling via `thinking_budget: 0`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budget: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_thoughts: Option<bool>,
}

impl ThinkingConfig {
    /// Disable thinking (Gemini 2.5 Flash). Keeps output tokens for the answer.
    pub fn disabled() -> Self {
        Self {
            thinking_budget: Some(0),
            include_thoughts: Some(false),
        }
    }
}

/// Parsed response envelope from Gemini (candidates or top-level error).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    #[serde(default)]
    pub candidates: Vec<Candidate>,
    #[serde(default)]
    pub error: Option<ApiErrorBody>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    #[serde(default)]
    pub content: Option<Content>,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorBody {
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub code: i32,
    #[serde(default)]
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinking_config_disabled_serializes() {
        let cfg = GenerationConfig {
            temperature: Some(0.2),
            max_output_tokens: Some(256),
            response_mime_type: Some("application/json".into()),
            response_schema: None,
            response_json_schema: Some(serde_json::json!({
                "type": "object",
                "properties": { "act": { "type": "boolean" } },
                "required": ["act"]
            })),
            thinking_config: Some(ThinkingConfig::disabled()),
        };
        let v = serde_json::to_value(&cfg).unwrap();
        assert_eq!(v["responseMimeType"], "application/json");
        assert!(v.get("responseSchema").is_none());
        assert!(v.get("responseJsonSchema").is_some());
        assert_eq!(v["thinkingConfig"]["thinkingBudget"], 0);
        assert_eq!(v["thinkingConfig"]["includeThoughts"], false);
    }

    #[test]
    fn blob_wire_json_has_only_mime_type_and_data() {
        let blob = Blob {
            mime_type: "application/pdf".into(),
            data: "abc".into(),
        };
        let v = serde_json::to_value(&blob).unwrap();
        assert_eq!(
            v,
            serde_json::json!({"mimeType": "application/pdf", "data": "abc"})
        );
        assert!(v.get("displayName").is_none());
    }

    #[test]
    fn role_serializes_as_gemini_wire_strings() {
        assert_eq!(serde_json::to_value(Role::User).unwrap(), "user");
        assert_eq!(serde_json::to_value(Role::Model).unwrap(), "model");
        assert_eq!(
            serde_json::from_value::<Role>(serde_json::json!("user")).unwrap(),
            Role::User
        );
        assert_eq!(
            serde_json::from_value::<Role>(serde_json::json!("model")).unwrap(),
            Role::Model
        );
        assert_eq!(
            serde_json::from_value::<Role>(serde_json::json!("assistant")).unwrap(),
            Role::Model
        );
        assert_eq!(
            serde_json::from_value::<Role>(serde_json::json!("")).unwrap(),
            Role::Model
        );
        assert_eq!(Role::parse("USER"), Role::User);
    }

    #[test]
    fn part_display_name_is_not_serialized_onto_inline_data() {
        let part = Part {
            inline_data: Some(Blob {
                mime_type: "application/pdf".into(),
                data: "abc".into(),
            }),
            display_name: "po.pdf".into(),
            ..Default::default()
        };
        let v = serde_json::to_value(&part).unwrap();
        assert!(v.get("displayName").is_none());
        assert!(v["inlineData"].get("displayName").is_none());
    }
}
