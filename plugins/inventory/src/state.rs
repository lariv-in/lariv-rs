use sea_orm::DatabaseConnection;

/// Shared Axum state for inventory routes.
#[derive(Clone)]
pub struct InventoryState {
    pub db: DatabaseConnection,
}

impl InventoryState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
