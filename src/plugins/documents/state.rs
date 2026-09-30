use sea_orm::DatabaseConnection;

/// Shared database connection for document routes.
#[derive(Clone)]
pub struct DocumentsState {
    pub db: DatabaseConnection,
}

impl DocumentsState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
