//! Gemini HTTP client for the LLM assistant.
//!
//! Thin wrapper around the Google Generative Language Interactions API
//! (`POST /v1beta/interactions`). Serializes/deserializes local [`Content`] shapes
//! via [`interactions`], handles SSE streaming, and surfaces errors as [`GenaiError`].
//!
//! # Submodules
//!
//! - [`client`] — [`GenaiClient`] HTTP calls and the assistant system prompt
//! - [`interactions`] — Interactions request/response and Content ↔ step converters
//! - [`types`] — persisted Content, Part, Tool, and function-calling types
//! - [`util`] — text extraction and streaming chunk merge helpers
//! - [`errors`] — [`GenaiError`] enum
//!
//! # Examples
//!
//! ```rust,ignore
//! use lariv_plugin_llm_assistant::genai::{GenaiClient, Content, Role};
//!
//! let client = GenaiClient::from_env("gemini-2.0-flash");
//! let text = client
//!     .generate_text("You are helpful.", "Summarize this document.")
//!     .await?;
//!
//! // Multi-turn with tools:
//! let content = client
//!     .generate_content(vec![Content::text(Role::User, "Hello")], 8192, &tool_decls)
//!     .await?;
//! ```

pub mod client;
pub mod errors;
pub mod interactions;
pub mod types;
pub mod util;

pub use client::{
    ASSISTANT_SYSTEM_PROMPT, GenaiClient, GenerateContentResult, GenerateTextResult,
    UploadFileTiming, UploadedGeminiFile,
};
pub use errors::GenaiError;
pub use interactions::{
    InteractionResult, contents_to_interaction_input, function_result_steps_from_content,
    steps_to_model_content, user_content_to_interaction_input,
};
pub use types::*;
pub use util::{
    coerce_json_text, content_answer_text, content_is_thought_only, content_text,
    content_thought_text, merge_content, part_is_empty, part_records_thought,
};
