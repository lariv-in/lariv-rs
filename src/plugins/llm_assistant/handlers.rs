//! HTTP handlers for chat, history, skills, cron jobs, preferences, and WebSocket streaming.
pub mod chat;
pub mod chat_upload;
pub mod cron;
pub mod history;
pub mod preferences;
pub mod skills;
pub mod ws;

/// Modal opener query (`?name=…&refresh=table-id`).
pub use crate::web::ModalFormQuery as ModalNameQuery;
