//! Shared database connection for the per-invoice audit log.

use sea_orm::DatabaseConnection;

/// Shared Axum state for the invoice log plugin.
#[derive(Clone)]
pub struct InvoiceLogState {
    pub db: DatabaseConnection,
}

impl InvoiceLogState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
