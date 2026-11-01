use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct HolidaysState {
    pub db: DatabaseConnection,
}

impl HolidaysState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
