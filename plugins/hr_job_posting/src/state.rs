use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct JobPostingState {
    pub db: DatabaseConnection,
}

impl JobPostingState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
