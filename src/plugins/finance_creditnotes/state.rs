use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct CreditnotesState {
    pub db: DatabaseConnection,
}

impl CreditnotesState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
