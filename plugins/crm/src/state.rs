use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct CrmState {
    pub db: DatabaseConnection,
}

impl CrmState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
