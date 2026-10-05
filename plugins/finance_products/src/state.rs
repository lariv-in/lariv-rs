use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct ProductsState {
    pub db: DatabaseConnection,
}

impl ProductsState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
