//! Shared Axum state for blog routes (DB connection).
use sea_orm::DatabaseConnection;

/// Shared Axum state for the blog plugin routes.
#[derive(Clone)]
pub struct BlogState {
    pub db: DatabaseConnection,
}

impl BlogState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
