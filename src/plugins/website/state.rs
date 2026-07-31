use std::sync::Arc;

use sea_orm::DatabaseConnection;

use super::config::WebsiteConfig;
use crate::plugins::filesystem::storage::DynFilestore;

/// Shared Axum state for the website plugin routes.
#[derive(Clone)]
pub struct WebsiteState {
    pub db: DatabaseConnection,
    pub store: Arc<DynFilestore>,
    pub config: WebsiteConfig,
}

impl WebsiteState {
    pub fn new(
        db: DatabaseConnection,
        store: Arc<DynFilestore>,
        config: WebsiteConfig,
    ) -> Self {
        Self { db, store, config }
    }
}
