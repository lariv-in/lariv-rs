use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct HrState {
    pub db: DatabaseConnection,
}

impl HrState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
