//! In-flight assistant turns that outlive a single WebSocket connection.
//!
//! When the client disconnects mid-turn, the Gemini/tool loop keeps running and
//! persists to the DB. A reconnecting socket can [`LiveTurns::subscribe`] to
//! receive subsequent [`StreamEvent`]s.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;

use super::genai::Content;

const EVENT_CAPACITY: usize = 256;

/// Events emitted during a streaming turn (WS layer builds OOB HTML).
#[derive(Debug, Clone)]
pub enum StreamEvent {
    UserSaved {
        session_id: i64,
        user: Content,
        /// Set when this prompt became the session title.
        title: Option<String>,
    },
    /// Live stream chunks (UI no longer shows a stream panel).
    Partial(Content),
    /// Model turn that includes function calls (args shown in transcript).
    ToolCall(Content),
    Tool(Content),
    Final(Content),
}

/// Process-local registry of active turns keyed by session id.
#[derive(Clone, Default)]
pub struct LiveTurns {
    inner: Arc<Mutex<HashMap<i64, broadcast::Sender<StreamEvent>>>>,
}

impl LiveTurns {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new turn publisher (call before spawning the turn task).
    pub fn insert(&self, session_id: i64, tx: broadcast::Sender<StreamEvent>) {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(session_id, tx);
    }

    pub fn remove(&self, session_id: i64) {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&session_id);
    }

    pub fn contains(&self, session_id: i64) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(&session_id)
    }

    /// Subscribe to live events for an in-flight turn, if any.
    pub fn subscribe(&self, session_id: i64) -> Option<broadcast::Receiver<StreamEvent>> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&session_id)
            .map(|tx| tx.subscribe())
    }
}

/// Create a broadcast channel for a new turn.
pub fn new_turn_channel() -> (
    broadcast::Sender<StreamEvent>,
    broadcast::Receiver<StreamEvent>,
) {
    broadcast::channel(EVENT_CAPACITY)
}

/// Emit a stream event; zero receivers (detached client) is normal.
pub fn emit(tx: &broadcast::Sender<StreamEvent>, event: StreamEvent) {
    let _ = tx.send(event);
}
