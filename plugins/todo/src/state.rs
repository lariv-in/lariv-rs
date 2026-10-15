//! Shared Axum state for todo routes (DB connection).
use sea_orm::DatabaseConnection;

/// Shared Axum state for the todo plugin routes.
#[derive(Clone)]
pub struct TodoState {
    pub db: DatabaseConnection,
}

impl TodoState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
