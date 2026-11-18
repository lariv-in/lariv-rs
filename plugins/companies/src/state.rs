use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct CompaniesState {
    pub db: DatabaseConnection,
}

impl CompaniesState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
