//! HTTP handlers for VNode list, detail, create, update, delete, and select.
pub mod chat_upload;
pub mod nodes;

use serde::Deserialize;

/// Modal opener query (`?name=p_filesystem.XxxForm`).
#[derive(Debug, Deserialize, Default)]
pub struct ModalNameQuery {
    #[serde(default)]
    pub name: Option<String>,
}
