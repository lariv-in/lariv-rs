use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct OvertimeState {
    pub db: DatabaseConnection,
}

impl OvertimeState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
