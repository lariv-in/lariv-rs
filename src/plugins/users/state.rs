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

use crate::datetime::{DatetimeLabel, DatetimeLocalInput};

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
    /// Read-only display label (not for storage round-trips).
    pub fn format_datetime(&self, dt: DateTime<Utc>) -> DatetimeLabel {
        DatetimeLabel::display(dt, &self.timezone)
    }

    /// Read-only short label (not for storage round-trips).
    pub fn format_datetime_short(&self, dt: DateTime<Utc>) -> DatetimeLabel {
        DatetimeLabel::short(dt, &self.timezone)
    }

    /// Read-only seconds label (not for storage round-trips).
    pub fn format_datetime_seconds(&self, dt: DateTime<Utc>) -> DatetimeLabel {
        DatetimeLabel::seconds(dt, &self.timezone)
    }

    /// Prefill a datetime text input from a stored instant (lossy display/edit).
    pub fn datetime_local_input(&self, dt: DateTime<Utc>) -> DatetimeLocalInput {
        DatetimeLocalInput::from_stored(dt, &self.timezone)
    }

    /// Parse a datetime text input into a stored UTC instant (lossy).
    pub fn parse_datetime_local_input(&self, value: &str) -> Option<DateTime<Utc>> {
        DatetimeLocalInput::from_raw(value).to_stored(&self.timezone)
    }
}
