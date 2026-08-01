use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::plugins::users::config::UsersConfig;

// Shared Axum state for the users plugin routes.
#[derive(Clone)]
pub struct UsersState {
    pub db: DatabaseConnection,
    pub config: UsersConfig,
    pub signing_key: Arc<Vec<u8>>,
    pub jwt_issuer: Arc<Vec<u8>>,
}

impl UsersState {
    pub fn new(db: DatabaseConnection, config: UsersConfig) -> Self {
        let signing_key = Arc::new(config.signing_key_bytes());
        let jwt_issuer = Arc::new(config.jwt_issuer_bytes());
        Self {
            db,
            config,
            signing_key,
            jwt_issuer,
        }
    }
}

// Authenticated request principal (Go `$user` / `$role` / `$tz`).
#[derive(Clone, Debug)]
pub struct AuthContext {
    pub user: crate::plugins::users::entities::User,
    pub role: String,
    pub timezone: String,
    /// Whether the user is superuser or has a configured staff role.
    pub is_staff: bool,
}
