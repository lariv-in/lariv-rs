use std::sync::Arc;

use sea_orm::DatabaseConnection;

use super::config::FilesystemConfig;
use super::storage::DynFilestore;

/// Shared Axum state for the filesystem plugin routes.
#[derive(Clone)]
pub struct FilesystemState {
    pub db: DatabaseConnection,
    /// Backend chosen from `[filesystem]` — see [`DynFilestore`].
    pub store: Arc<DynFilestore>,
    pub config: FilesystemConfig,
}

impl FilesystemState {
    pub fn new(db: DatabaseConnection, store: Arc<DynFilestore>, config: FilesystemConfig) -> Self {
        Self { db, store, config }
    }
}
