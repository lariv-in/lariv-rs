use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct ContactsState {
    pub db: DatabaseConnection,
}

impl ContactsState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
