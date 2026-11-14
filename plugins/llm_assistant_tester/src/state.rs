//! Shared Axum state for assistant-tester routes (DB connection).
use sea_orm::DatabaseConnection;

/// Shared Axum state for the assistant tester plugin routes.
#[derive(Clone)]
pub struct LlmAssistantTesterState {
    pub db: DatabaseConnection,
}

impl LlmAssistantTesterState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
