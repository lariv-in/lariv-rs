//! In-flight assistant turns that outlive a single WebSocket connection.
//!
//! When the client disconnects mid-turn, the Gemini/tool loop keeps running and
//! persists to the DB. A reconnecting socket can [`LiveTurns::subscribe`] to
//! receive subsequent [`StreamEvent`]s. [`LiveTurns::cancel`] cooperatively
//! aborts an in-flight turn (Stop button) without dropping cleanup.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

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
    /// Turn aborted; restore Send. Any leftover text was already emitted as [`Final`].
    Stopped,
}

struct LiveTurnSlot {
    tx: broadcast::Sender<StreamEvent>,
    cancel: CancellationToken,
}

/// Process-local registry of active turns keyed by session id.
#[derive(Clone, Default)]
pub struct LiveTurns {
    inner: Arc<Mutex<HashMap<i64, LiveTurnSlot>>>,
}

impl LiveTurns {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new turn publisher (call before spawning the turn task).
    pub fn insert(
        &self,
        session_id: i64,
        tx: broadcast::Sender<StreamEvent>,
        cancel: CancellationToken,
    ) {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(session_id, LiveTurnSlot { tx, cancel });
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

    /// Cooperatively abort an in-flight turn. Returns true if a turn was found.
    pub fn cancel(&self, session_id: i64) -> bool {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match guard.get(&session_id) {
            Some(slot) => {
                slot.cancel.cancel();
                true
            }
            None => false,
        }
    }

    /// Subscribe to live events for an in-flight turn, if any.
    pub fn subscribe(&self, session_id: i64) -> Option<broadcast::Receiver<StreamEvent>> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&session_id)
            .map(|slot| slot.tx.subscribe())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_unknown_session_is_false() {
        let live = LiveTurns::new();
        assert!(!live.cancel(1));
    }

    #[test]
    fn cancel_trips_token_without_removing() {
        let live = LiveTurns::new();
        let (tx, _rx) = new_turn_channel();
        let cancel = CancellationToken::new();
        live.insert(7, tx, cancel.clone());
        assert!(live.contains(7));
        assert!(live.cancel(7));
        assert!(cancel.is_cancelled());
        assert!(live.contains(7));
        live.remove(7);
        assert!(!live.contains(7));
    }
}
