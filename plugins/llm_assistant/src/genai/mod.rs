//! Gemini HTTP client for the LLM assistant.
//!
//! Thin wrapper around the Google Generative Language API (`generateContent` and
//! `streamGenerateContent`). Serializes/deserializes Gemini REST shapes from [`types`],
//! handles SSE streaming merge via [`util`], and surfaces errors as [`GenaiError`].
//!
//! # Submodules
//!
//! - [`client`] — [`GenaiClient`] HTTP calls and the assistant system prompt
//! - [`types`] — request/response Content, Part, Tool, and function-calling types
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
pub mod types;
pub mod util;

pub use client::{
    ASSISTANT_SYSTEM_PROMPT, GenaiClient, GenerateContentResult, UploadFileTiming,
    UploadedGeminiFile,
};
pub use errors::GenaiError;
pub use types::*;
pub use util::{
    coerce_json_text, content_answer_text, content_is_thought_only, content_text, merge_content,
    part_is_empty,
};
