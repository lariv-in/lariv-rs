//! HTTP handlers for chat, history, skills, and WebSocket streaming.
pub mod chat;
pub mod history;
pub mod skills;
pub mod ws;

use serde::Deserialize;

/// Modal opener query (`?name=…`).
#[derive(Debug, Deserialize, Default)]
pub struct ModalNameQuery {
    #[serde(default)]
    pub name: Option<String>,
}
