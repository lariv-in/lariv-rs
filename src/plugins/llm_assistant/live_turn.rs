//! In-flight assistant turns that outlive a single WebSocket connection.
//!
//! When the client disconnects mid-turn, the Gemini/tool loop keeps running and
//! persists to the DB. A reconnecting socket can [`LiveTurns::subscribe`] to
//! receive subsequent [`StreamEvent`]s. [`LiveTurns::cancel`] cooperatively
//! aborts an in-flight turn (Stop button) without dropping cleanup.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::Value as JsonValue;
use tokio::sync::{broadcast, oneshot};
use tokio_util::sync::CancellationToken;

use crate::llm_tools::HitlGate;

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
    /// HITL Rune function waiting for Run / Deny.
    HitlPending {
        id: String,
        name: String,
        args: JsonValue,
    },
    /// HITL decision applied (card is replaced in the transcript).
    HitlResolved {
        id: String,
        name: String,
        approved: bool,
    },
    Final(Content),
    /// Turn aborted; restore Send. Any leftover text was already emitted as [`Final`].
    Stopped,
}

/// Snapshot of a pending HITL prompt (for WS reconnect).
#[derive(Debug, Clone)]
pub struct PendingHitl {
    pub id: String,
    pub name: String,
    pub args: JsonValue,
}

enum HitlDecision {
    Approve,
    Deny,
}

struct LiveTurnSlot {
    tx: broadcast::Sender<StreamEvent>,
    cancel: CancellationToken,
    pending_hitl: Option<PendingHitl>,
    hitl_reply: Option<oneshot::Sender<HitlDecision>>,
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
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).insert(
            session_id,
            LiveTurnSlot {
                tx,
                cancel,
                pending_hitl: None,
                hitl_reply: None,
            },
        );
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
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match guard.get_mut(&session_id) {
            Some(slot) => {
                slot.cancel.cancel();
                slot.pending_hitl = None;
                let _ = slot.hitl_reply.take();
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

    /// Pending HITL card to re-render after WebSocket reconnect.
    pub fn pending_hitl(&self, session_id: i64) -> Option<PendingHitl> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&session_id)
            .and_then(|slot| slot.pending_hitl.clone())
    }

    /// Gate that blocks Rune HITL invokes until [`Self::resolve_hitl`] or cancel.
    pub fn hitl_gate(&self, session_id: i64) -> HitlGate {
        let live = self.clone();
        Arc::new(move |name, args| {
            crate::rune_env::block_on_async(live.request_approval(session_id, name, args.clone()))
        })
    }

    /// Apply a Run / Deny click from the chat UI. Returns true when the id matched.
    pub fn resolve_hitl(&self, session_id: i64, id: &str, approved: bool) -> bool {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(slot) = guard.get_mut(&session_id) else {
            return false;
        };
        let matches = slot
            .pending_hitl
            .as_ref()
            .is_some_and(|pending| pending.id == id);
        if !matches {
            return false;
        }
        let name = slot
            .pending_hitl
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_default();
        slot.pending_hitl = None;
        let reply = slot.hitl_reply.take();
        let tx = slot.tx.clone();
        drop(guard);
        if let Some(reply) = reply {
            let decision = if approved {
                HitlDecision::Approve
            } else {
                HitlDecision::Deny
            };
            let _ = reply.send(decision);
        }
        emit(
            &tx,
            StreamEvent::HitlResolved {
                id: id.to_string(),
                name,
                approved,
            },
        );
        true
    }

    async fn request_approval(
        &self,
        session_id: i64,
        name: &str,
        args: JsonValue,
    ) -> Result<(), String> {
        let id = uuid::Uuid::new_v4().to_string();
        let (reply_tx, reply_rx) = oneshot::channel();
        let (event_tx, cancel) = {
            let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let slot = guard
                .get_mut(&session_id)
                .ok_or_else(|| "requires human approval".to_string())?;
            slot.pending_hitl = Some(PendingHitl {
                id: id.clone(),
                name: name.to_string(),
                args: args.clone(),
            });
            slot.hitl_reply = Some(reply_tx);
            (slot.tx.clone(), slot.cancel.clone())
        };
        emit(
            &event_tx,
            StreamEvent::HitlPending {
                id: id.clone(),
                name: name.to_string(),
                args,
            },
        );

        tokio::select! {
            _ = cancel.cancelled() => {
                self.clear_pending(session_id);
                Err("cancelled".into())
            }
            decision = reply_rx => {
                match decision {
                    Ok(HitlDecision::Approve) => Ok(()),
                    Ok(HitlDecision::Deny) => Err("denied".into()),
                    Err(_) => Err("cancelled".into()),
                }
            }
        }
    }

    fn clear_pending(&self, session_id: i64) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(slot) = guard.get_mut(&session_id) {
            slot.pending_hitl = None;
            slot.hitl_reply = None;
        }
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

    #[test]
    fn resolve_hitl_unknown_is_false() {
        let live = LiveTurns::new();
        assert!(!live.resolve_hitl(1, "abc", true));
    }
}
