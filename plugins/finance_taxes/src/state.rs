use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct TaxesState {
    pub db: DatabaseConnection,
}

impl TaxesState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
