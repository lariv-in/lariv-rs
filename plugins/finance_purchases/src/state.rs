use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct PurchasesState {
    pub db: DatabaseConnection,
}

impl PurchasesState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
