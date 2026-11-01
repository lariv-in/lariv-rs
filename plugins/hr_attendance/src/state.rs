use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct AttendanceState {
    pub db: DatabaseConnection,
}

impl AttendanceState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
