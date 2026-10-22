use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct ServicesState {
    pub db: DatabaseConnection,
}

impl ServicesState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
