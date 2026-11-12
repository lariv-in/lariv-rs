//! Shared Axum state for import and export routes (DB connection).
use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct ImportExportState {
    pub db: DatabaseConnection,
}

impl ImportExportState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
