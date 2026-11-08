//! Shared state for mdBook publishing (DB and filestore).

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use lariv_plugin_filesystem::storage::DynFilestore;

/// DB connection and blob store captured at mount.
///
/// Book paths and the URL prefix come from [`super::preferences`], not from TOML.
#[derive(Clone)]
pub struct MdBookState {
    pub db: DatabaseConnection,
    pub store: Arc<DynFilestore>,
}

impl MdBookState {
    pub fn new(db: DatabaseConnection, store: Arc<DynFilestore>) -> Self {
        Self { db, store }
    }
}
