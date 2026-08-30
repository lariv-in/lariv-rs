//! Shared Gemini HTTP client (used by LLM Assistant, Totschool AI workers, etc.).
//!
//! Thin wrapper around the Google Generative Language API (`generateContent` and
//! `streamGenerateContent`). Serializes/deserializes Gemini REST shapes from [`types`],
//! handles SSE streaming merge via [`util`], and surfaces errors as [`GenaiError`].
//!
//! # Submodules
//!
//! - [`client`] — [`GenaiClient`] HTTP calls and the LLM Assistant system prompt
//! - [`types`] — request/response Content, Part, Tool, and function-calling types
//! - [`util`] — text extraction and streaming chunk merge helpers
//! - [`errors`] — [`GenaiError`] enum
//!
//! # Examples
//!
//! ```rust ignore
//! use lariv_rs::genai::{GenaiClient, Content, Role};
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

pub mod types;
pub mod util;

#[cfg(feature = "cap-llm")]
pub mod client;
#[cfg(feature = "cap-llm")]
pub mod errors;

#[cfg(feature = "cap-llm")]
pub use client::{ASSISTANT_SYSTEM_PROMPT, GenaiClient, UploadFileTiming, UploadedGeminiFile};
#[cfg(feature = "cap-llm")]
pub use errors::GenaiError;
pub use types::*;
pub use util::{coerce_json_text, content_answer_text, content_text, merge_content, part_is_empty};
