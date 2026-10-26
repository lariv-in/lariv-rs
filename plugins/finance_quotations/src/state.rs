use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct QuotationsState {
    pub db: DatabaseConnection,
}

impl QuotationsState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
