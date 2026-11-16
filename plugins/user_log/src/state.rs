//! Shared database connection for the per-user audit log.
use sea_orm::DatabaseConnection;

/// Shared Axum state for the user log plugin.
#[derive(Clone)]
pub struct UserLogState {
    pub db: DatabaseConnection,
}

impl UserLogState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
