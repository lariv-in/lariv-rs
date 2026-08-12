use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct CustomerState {
    pub db: DatabaseConnection,
}

impl CustomerState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
