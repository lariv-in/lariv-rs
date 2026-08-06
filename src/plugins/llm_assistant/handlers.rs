//! HTTP handlers for chat, history, skills, and WebSocket streaming.
pub mod chat;
pub mod history;
pub mod skills;
pub mod ws;

/// Modal opener query (`?name=…&refresh=table-id`).
pub use crate::web::ModalFormQuery as ModalNameQuery;
