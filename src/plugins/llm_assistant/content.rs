//! Content ↔ DB persistence.

pub mod attachments;
pub mod kinds;
pub mod persist;
pub mod sanitize;

#[cfg(test)]
mod persist_tests;

pub use persist::{PersistError, load_session_contents, save_content};
pub use sanitize::{
    ZWSP, elide_attachment_parts_for_api, sanitize_content_parts_for_genai_chat,
    sanitize_json_for_postgres, strip_nul_chars,
};
