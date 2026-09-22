use std::collections::HashMap;
use std::sync::Arc;

use moq_relay::Cluster;
use rand::RngExt;
use sea_orm::DatabaseConnection;
use tokio::sync::{Mutex, RwLock};

use crate::plugins::filesystem::storage::DynFilestore;

use super::config::MeetsConfig;
use super::moq_auth::MoqAuthState;
use super::recording::room_subscriber::RoomRecorder;

/// Shared Axum state for meets routes.
#[derive(Clone)]
pub struct MeetsState {
    pub db: DatabaseConnection,
    pub store: Arc<DynFilestore>,
    pub config: MeetsConfig,
    pub anon_secret: Arc<Vec<u8>>,
    pub moq_auth: Arc<MoqAuthState>,
    pub relay_cluster: Arc<RwLock<Option<Cluster>>>,
    pub recorders: Arc<Mutex<HashMap<String, Arc<RoomRecorder>>>>,
}

impl MeetsState {
    pub fn new(db: DatabaseConnection, store: Arc<DynFilestore>, config: MeetsConfig) -> Self {
        if config.transport.enabled && config.transport.public_url.is_empty() {
            tracing::warn!(
                "meets: transport.publicUrl is unset; clients will use relative MoQ relay URLs"
            );
        }
        let anon_secret = if config.anon_cookie_secret.is_empty() {
            let bytes: [u8; 32] = rand::rng().random();
            bytes.to_vec()
        } else {
            config.anon_cookie_secret.as_bytes().to_vec()
        };
        let moq_auth = Arc::new(MoqAuthState::load(&config).expect("meets: load MoQ auth key"));
        Self {
            db,
            store,
            config,
            anon_secret: Arc::new(anon_secret),
            moq_auth: Arc::clone(&moq_auth),
            relay_cluster: Arc::new(RwLock::new(None)),
            recorders: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// MoQ relay dial URL and JWT for a joined participant.
    pub fn moq_auth_for_joined(
        &self,
        room_code: &str,
        joined_user_id: i64,
    ) -> anyhow::Result<(String, String)> {
        let jwt = self
            .moq_auth
            .token_for_participant(room_code, joined_user_id)?;
        let url = self.config.transport.room_relay_url(room_code);
        Ok((url, jwt))
    }

    pub async fn relay_cluster(&self) -> Option<Cluster> {
        self.relay_cluster.read().await.clone()
    }

    pub async fn ensure_room_recorder(&self, room_code: &str, created_by_id: i64) {
        let cluster = self.relay_cluster().await;
        super::recording::room_subscriber::ensure_room_recorder(
            &self.recorders,
            cluster,
            self.db.clone(),
            Arc::clone(&self.store),
            room_code,
            created_by_id,
        )
        .await;
    }

    pub async fn stop_room_recorder(&self, room_code: &str) {
        super::recording::room_subscriber::stop_room_recorder(&self.recorders, room_code).await;
    }
}
