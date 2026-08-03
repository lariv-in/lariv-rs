//! HTTP handlers for blog and tag CRUD.
pub mod blogs;
pub mod tags;

use serde::Deserialize;

/// Modal opener query (`?name=p_blog.XxxForm`). Case-sensitive vs filter `Name`.
#[derive(Debug, Deserialize, Default)]
pub struct ModalNameQuery {
    #[serde(default)]
    pub name: Option<String>,
}
