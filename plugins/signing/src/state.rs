use std::sync::Arc;

use sea_orm::DatabaseConnection;

use super::backend::DynKeyBackend;

/// Shared database connection and key backend for signing routes.
#[derive(Clone)]
pub struct SigningState {
    pub db: DatabaseConnection,
    pub keys: Arc<DynKeyBackend>,
}

impl SigningState {
    pub fn new(db: DatabaseConnection, keys: Arc<DynKeyBackend>) -> Self {
        Self { db, keys }
    }
}
