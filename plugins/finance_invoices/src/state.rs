use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct InvoicesState {
    pub db: DatabaseConnection,
}

impl InvoicesState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
