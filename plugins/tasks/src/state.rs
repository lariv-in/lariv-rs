use std::sync::{Arc, RwLock};

use sea_orm::DatabaseConnection;

use crate::push::PushSettings;

/// Shared Axum state for tasks plugin routes.
#[derive(Clone)]
pub struct TasksState {
    pub db: DatabaseConnection,
    /// VAPID material. Shared so a key generated at startup is visible to later requests.
    pub push: Arc<RwLock<PushSettings>>,
}

impl TasksState {
    pub fn new(db: DatabaseConnection, push: PushSettings) -> Self {
        Self {
            db,
            push: Arc::new(RwLock::new(push)),
        }
    }
}
