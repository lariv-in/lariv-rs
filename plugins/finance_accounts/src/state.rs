use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct AccountsState {
    pub db: DatabaseConnection,
}

impl AccountsState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
