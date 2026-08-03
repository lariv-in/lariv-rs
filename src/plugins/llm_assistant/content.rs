//! Content ↔ DB persistence.

pub mod kinds;
pub mod persist;
pub mod sanitize;

#[cfg(test)]
mod persist_tests;

pub use persist::{PersistError, load_session_contents, save_content};
pub use sanitize::{
    sanitize_content_parts_for_genai_chat, strip_display_name_from_contents, ZWSP,
};
