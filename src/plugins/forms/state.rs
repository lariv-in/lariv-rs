use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct FormsState {
    pub db: DatabaseConnection,
}

impl FormsState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
