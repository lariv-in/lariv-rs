use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct GstExportState {
    pub db: DatabaseConnection,
}

impl GstExportState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
