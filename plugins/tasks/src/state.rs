use sea_orm::DatabaseConnection;

/// Shared Axum state for tasks plugin routes.
#[derive(Clone)]
pub struct TasksState {
    pub db: DatabaseConnection,
}

impl TasksState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
