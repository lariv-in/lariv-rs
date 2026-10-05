//! HTTP handlers — public dynamic pages and admin route/builder CRUD.
pub mod builder;
pub mod dynamic;
pub mod preferences;
pub mod routes;

/// Modal opener query (`?name=…&refresh=table-id`).
pub use lariv_core::web::ModalFormQuery as ModalNameQuery;
