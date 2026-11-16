//! Shared database connection for the audit log routes.
use sea_orm::DatabaseConnection;

/// Shared Axum state for the logging plugin.
#[derive(Clone)]
pub struct LoggingState {
    pub db: DatabaseConnection,
}

impl LoggingState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
