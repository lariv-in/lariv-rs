use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct LeavesState {
    pub db: DatabaseConnection,
}

impl LeavesState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
