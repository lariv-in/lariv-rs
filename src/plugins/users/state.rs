//! Shared Axum state and authenticated request principal for users routes.
use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::plugins::users::config::UsersConfig;

/// Shared Axum state for the users plugin routes.
#[derive(Clone)]
pub struct UsersState {
    /// Database connection.
    pub db: DatabaseConnection,
    /// Loaded `[users]` config section.
    pub config: UsersConfig,
    /// Resolved HS512 signing key bytes.
    pub signing_key: Arc<Vec<u8>>,
    /// Resolved JWT issuer bytes.
    pub jwt_issuer: Arc<Vec<u8>>,
}

impl UsersState {
    /// Build state from DB connection and config (resolves signing keys).
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

use chrono::{DateTime, Utc};

use crate::datetime::{
    format_datetime_display, format_datetime_local_input, format_datetime_seconds,
    format_datetime_short, parse_datetime_local_input,
};

/// Authenticated request principal.
#[derive(Clone, Debug)]
pub struct AuthContext {
    /// Active user row.
    pub user: crate::plugins::users::entities::User,
    /// Role name string.
    pub role: String,
    /// User timezone (IANA name).
    pub timezone: String,
    /// Whether the user is superuser or has a configured staff role.
    pub is_staff: bool,
}

impl AuthContext {
    pub fn format_datetime(&self, dt: DateTime<Utc>) -> String {
        format_datetime_display(dt, &self.timezone)
    }

    pub fn format_datetime_short(&self, dt: DateTime<Utc>) -> String {
        format_datetime_short(dt, &self.timezone)
    }

    pub fn format_datetime_seconds(&self, dt: DateTime<Utc>) -> String {
        format_datetime_seconds(dt, &self.timezone)
    }

    pub fn format_datetime_local_input(&self, dt: DateTime<Utc>) -> String {
        format_datetime_local_input(dt, &self.timezone)
    }

    pub fn parse_datetime_local_input(&self, value: &str) -> Option<DateTime<Utc>> {
        parse_datetime_local_input(value, &self.timezone)
    }
}
