use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct FiscalYearState {
    pub db: DatabaseConnection,
}

impl FiscalYearState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
