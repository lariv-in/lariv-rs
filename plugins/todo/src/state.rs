//! Shared Axum state for todo routes (DB connection and per-page fanout).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use sea_orm::DatabaseConnection;
use tokio::sync::{OwnedMutexGuard, broadcast};

use crate::sync::ServerEvent;

const FANOUT_CAPACITY: usize = 256;

/// Shared Axum state for the todo plugin routes.
#[derive(Clone)]
pub struct TodoState {
    pub db: DatabaseConnection,
    pub hub: TodoHub,
}

impl TodoState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            hub: TodoHub::new(),
        }
    }
}

/// Per-page broadcast plus a write lock so a snapshot cannot race an update.
#[derive(Clone)]
pub struct TodoHub {
    inner: Arc<Mutex<HashMap<i64, UserSlot>>>,
}

#[derive(Clone)]
struct UserSlot {
    tx: broadcast::Sender<ServerEvent>,
    write: Arc<tokio::sync::Mutex<()>>,
}

impl TodoHub {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Subscribe before applying a sync batch so live events are not missed.
    pub fn subscribe(&self, page_id: i64) -> broadcast::Receiver<ServerEvent> {
        self.slot(page_id).tx.subscribe()
    }

    /// Serializes writes and snapshot reads for one page.
    pub async fn lock_page(&self, page_id: i64) -> OwnedMutexGuard<()> {
        self.slot(page_id).write.clone().lock_owned().await
    }

    pub fn publish(&self, page_id: i64, event: ServerEvent) {
        if self.slot(page_id).tx.send(event).is_err() {
            tracing::debug!(page_id, "todo: no websocket subscribers");
        }
    }

    fn slot(&self, page_id: i64) -> UserSlot {
        let mut map = self.inner.lock().unwrap_or_else(|err| err.into_inner());
        map.entry(page_id)
            .or_insert_with(|| {
                let (tx, _rx) = broadcast::channel(FANOUT_CAPACITY);
                UserSlot {
                    tx,
                    write: Arc::new(tokio::sync::Mutex::new(())),
                }
            })
            .clone()
    }
}

impl Default for TodoHub {
    fn default() -> Self {
        Self::new()
    }
}
