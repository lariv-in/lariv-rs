use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct BankAccountsState {
    pub db: DatabaseConnection,
}

impl BankAccountsState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
