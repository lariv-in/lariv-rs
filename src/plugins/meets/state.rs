use std::collections::HashMap;
use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio::sync::RwLock;

use crate::plugins::filesystem::storage::DynFilestore;

use super::config::MeetsConfig;
use super::sfu::SfuRoom;

/// Shared Axum state for meets routes.
#[derive(Clone)]
pub struct MeetsState {
    pub db: DatabaseConnection,
    pub store: Arc<DynFilestore>,
    pub config: MeetsConfig,
    pub anon_secret: Arc<Vec<u8>>,
    pub rooms: Arc<RwLock<HashMap<String, Arc<SfuRoom>>>>,
}

impl MeetsState {
    pub fn new(db: DatabaseConnection, store: Arc<DynFilestore>, config: MeetsConfig) -> Self {
        let anon_secret = if config.anon_cookie_secret.is_empty() {
            let bytes: [u8; 32] = rand::random();
            bytes.to_vec()
        } else {
            config.anon_cookie_secret.as_bytes().to_vec()
        };
        Self {
            db,
            store,
            config,
            anon_secret: Arc::new(anon_secret),
            rooms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn room(&self, code: &str) -> Arc<SfuRoom> {
        {
            let guard = self.rooms.read().await;
            if let Some(existing) = guard.get(code) {
                return Arc::clone(existing);
            }
        }
        let mut guard = self.rooms.write().await;
        Arc::clone(guard.entry(code.to_string()).or_insert_with(|| {
            Arc::new(SfuRoom::new(
                code.to_string(),
                self.config.clone(),
                self.db.clone(),
                Arc::clone(&self.store),
            ))
        }))
    }
}
