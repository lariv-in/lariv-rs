//! Thin Gemini HTTP client (no separate `p_google_genai` plugin).

pub mod client;
pub mod errors;
pub mod types;

pub use client::{ASSISTANT_SYSTEM_PROMPT, GenaiClient, merge_assistant_content};
pub use errors::GenaiError;
pub use types::*;
