pub mod builder;
pub mod dynamic;
pub mod routes;

use serde::Deserialize;

/// Modal opener query (`?name=…`).
#[derive(Debug, Deserialize, Default)]
pub struct ModalNameQuery {
    #[serde(default)]
    pub name: Option<String>,
}
