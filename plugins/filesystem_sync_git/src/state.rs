use std::sync::Arc;

use sea_orm::DatabaseConnection;

use lariv_plugin_filesystem::storage::DynFilestore;

/// DB connection and blob store captured at mount.
#[derive(Clone)]
pub struct FilesystemSyncGitState {
    pub db: DatabaseConnection,
    pub store: Arc<DynFilestore>,
}

impl FilesystemSyncGitState {
    pub fn new(db: DatabaseConnection, store: Arc<DynFilestore>) -> Self {
        Self { db, store }
    }
}
