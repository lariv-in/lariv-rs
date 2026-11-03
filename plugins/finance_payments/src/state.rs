use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct PaymentsState {
    pub db: DatabaseConnection,
}

impl PaymentsState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
