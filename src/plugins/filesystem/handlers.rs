//! HTTP handlers for VNode list, detail, create, update, delete, and select.
pub mod chat_upload;
pub mod nodes;

/// Modal opener query (`?name=…&refresh=table-id`).
pub use crate::web::ModalFormQuery as ModalNameQuery;
