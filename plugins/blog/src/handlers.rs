//! HTTP handlers for blog and tag CRUD.
pub mod blogs;
pub mod tags;

/// Modal opener query (`?name=…&refresh=table-id`). Case-sensitive vs filter `Name`.
pub use lariv_core::web::ModalFormQuery as ModalNameQuery;
