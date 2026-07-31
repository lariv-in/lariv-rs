//! Shared Gemini HTTP client (used by LLM Assistant, Totschool AI workers, etc.).

pub mod client;
pub mod errors;
pub mod types;
pub mod util;

pub use client::{merge_assistant_content, ASSISTANT_SYSTEM_PROMPT, GenaiClient};
pub use errors::GenaiError;
pub use types::*;
pub use util::{content_text, merge_content, part_is_empty};
