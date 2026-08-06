//! HTTP handlers — public dynamic pages and admin route/builder CRUD.
pub mod builder;
pub mod dynamic;
pub mod routes;

/// Modal opener query (`?name=…&refresh=table-id`).
pub use crate::web::ModalFormQuery as ModalNameQuery;
