//! Shared Axum state for OTP routes (DB connection).
use std::sync::Arc;

use sea_orm::DatabaseConnection;

use super::otp::MemoryCache;

/// Shared Axum state for the OTP plugin routes.
#[derive(Clone)]
pub struct OtpState {
    pub db: DatabaseConnection,
    pub cache: Arc<MemoryCache>,
}

impl OtpState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            cache: Arc::new(MemoryCache::new()),
        }
    }
}
