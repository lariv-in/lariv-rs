//! Shared Axum state for import routes (DB connection).
use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct ImportState {
    pub db: DatabaseConnection,
}

impl ImportState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
