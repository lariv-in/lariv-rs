//! HTTP handlers for VNode list, detail, create, update, delete, and select.
pub mod nodes;
pub mod pdf;

/// Modal opener query (`?name=…&refresh=table-id`).
pub use crate::web::ModalFormQuery as ModalNameQuery;
