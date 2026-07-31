use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct ExportState {
    pub db: DatabaseConnection,
}

impl ExportState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
