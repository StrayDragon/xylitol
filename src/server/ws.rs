//! WebSocket protocol — event streaming and reverse RPC for the server.
//!
//! Frame protocol (JSON over WebSocket):
//!
//! **Server → Client:**
//! - `ServerHello { version }` — sent on connect after Subscribe
//! - `Ack { seq, request_id }` — acknowledgment of Subscribe
//! - `Event { session_id, seq, event }` — streamed agent event
//! - `ResyncRequired { session_id }` — journal truncated, client needs full resync
//! - `ReverseRpc { rpc_type, call_id, payload }` — approval/question from server
//!
//! **Client → Server:**
//! - `Subscribe { session_id, last_seq }` — subscribe to event stream
//! - `ApproveTool { call_id, approved }` — respond to approval request
//! - `AnswerQuestion { call_id, answer }` — respond to question
//! - `Ping` — keep-alive

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tokio::sync::oneshot;

use serde::{Deserialize, Serialize};

use crate::protocol::Event;

// ── Frame types ────────────────────────────────────────────────────

/// Frame sent from server to client.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerFrame {
    ServerHello {
        version: String,
    },
    Ack {
        seq: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    Event {
        session_id: String,
        seq: u64,
        event: Event,
    },
    ResyncRequired {
        session_id: String,
    },
}

/// Frame sent from client to server.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientFrame {
    Subscribe {
        session_id: String,
        last_seq: u64,
    },
    ApproveTool {
        call_id: String,
        approved: bool,
    },
    AnswerQuestion {
        call_id: String,
        answer: String,
    },
    Ping,
}

// ── Event journal (per-session ring buffer) ────────────────────────

/// Default capacity of the event journal per session.
pub const DEFAULT_JOURNAL_CAPACITY: usize = 10_000;

/// A ring-buffer journal of recent events for a session.
///
/// Each event carries a monotonically increasing sequence number.
/// When the journal wraps around, old events beyond the capacity are lost.
pub struct EventJournal {
    session_id: String,
    capacity: usize,
    /// Monotonic sequence counter for this session.
    next_seq: AtomicU64,
    /// Ring buffer of (seq, Event) pairs.
    buffer: VecDeque<(u64, Event)>,
}

impl EventJournal {
    /// Create a new journal with the given capacity.
    pub fn new(session_id: impl Into<String>, capacity: usize) -> Self {
        Self {
            session_id: session_id.into(),
            capacity,
            next_seq: AtomicU64::new(1),
            buffer: VecDeque::with_capacity(capacity),
        }
    }

    /// Create a journal with default capacity.
    pub fn with_default_capacity(session_id: impl Into<String>) -> Self {
        Self::new(session_id, DEFAULT_JOURNAL_CAPACITY)
    }

    /// Append an event and return its sequence number.
    pub fn append(&mut self, event: Event) -> u64 {
        let seq = self.next_seq.fetch_add(1, Ordering::SeqCst);
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back((seq, event));
        seq
    }

    /// Get the current next sequence number (last assigned + 1).
    pub fn current_seq(&self) -> u64 {
        self.next_seq.load(Ordering::SeqCst)
    }

    /// Get the maximum available seq in the journal.
    /// Returns 0 if journal is empty.
    pub fn max_seq(&self) -> u64 {
        self.buffer.back().map(|(seq, _)| *seq).unwrap_or(0)
    }

    /// Get the minimum available seq in the journal.
    /// Returns 0 if journal is empty.
    pub fn min_seq(&self) -> u64 {
        self.buffer.front().map(|(seq, _)| *seq).unwrap_or(0)
    }

    /// Replay events after `from_seq` (exclusive) to the end of the journal.
    ///
    /// Returns `None` only if the journal has wrapped (capacity reached) AND
    /// `from_seq` is strictly less than the minimum available seq, meaning
    /// the requested events have been evicted. A `from_seq` of `0` always
    /// replays all available events (initial subscription).
    pub fn replay_from(&self, from_seq: u64) -> Option<Vec<(u64, Event)>> {
        let min = self.min_seq();
        if from_seq > 0
            && from_seq < min
            && self.buffer.len() >= self.capacity
        {
            return None; // ResyncRequired — journal wrapped past from_seq
        }
        Some(
            self.buffer
                .iter()
                .filter(|(seq, _)| *seq > from_seq)
                .cloned()
                .collect(),
        )
    }

    /// The session ID this journal belongs to.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

// ── Reverse RPC gateway ────────────────────────────────────────────

/// Default timeout for reverse RPC calls before returning ApprovalTimeout.
pub const REVERSE_RPC_TIMEOUT: Duration = Duration::from_secs(60);

/// Manages reverse RPC calls from server to client (approval/questions).
///
/// Each call is registered with a `call_id` and a oneshot sender. The first
/// client response for a given `call_id` wins; subsequent responses are
/// silently ignored. A timeout (60s) releases the channel with a timeout error.
pub struct ReverseRpcGateway {
    pending: Mutex<HashMap<String, oneshot::Sender<ReverseRpcResult>>>,
}

/// The result of a reverse RPC call.
#[derive(Debug, Clone, PartialEq)]
pub enum ReverseRpcResult {
    Approved,
    Denied,
    Answered(String),
    Timeout,
}

impl ReverseRpcGateway {
    /// Create a new empty gateway.
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// Register a new reverse RPC call, returning a receiver for the result.
    ///
    /// If a call with the same `call_id` already exists, the old one is
    /// replaced (the old receiver will get a cancelled error).
    pub fn register(&self, call_id: String) -> oneshot::Receiver<ReverseRpcResult> {
        let (tx, rx) = oneshot::channel();
        let mut map = self.pending.lock().expect("reverse rpc lock");
        map.insert(call_id, tx);
        rx
    }

    /// Handle an `ApproveTool` client frame.
    ///
    /// Returns `true` if the call_id was found and consumed (first response wins).
    pub fn handle_approve(&self, call_id: &str, approved: bool) -> bool {
        let mut map = self.pending.lock().expect("reverse rpc lock");
        if let Some(tx) = map.remove(call_id) {
            let result = if approved {
                ReverseRpcResult::Approved
            } else {
                ReverseRpcResult::Denied
            };
            let _ = tx.send(result); // ignore if receiver dropped
            true
        } else {
            false // already consumed or never registered
        }
    }

    /// Handle an `AnswerQuestion` client frame.
    ///
    /// Returns `true` if the call_id was found and consumed (first response wins).
    pub fn handle_answer(&self, call_id: &str, answer: String) -> bool {
        let mut map = self.pending.lock().expect("reverse rpc lock");
        if let Some(tx) = map.remove(call_id) {
            let _ = tx.send(ReverseRpcResult::Answered(answer));
            true
        } else {
            false
        }
    }

    /// Remove a pending call_id (e.g., on timeout).
    pub fn remove(&self, call_id: &str) {
        let mut map = self.pending.lock().expect("reverse rpc lock");
        map.remove(call_id);
    }

    /// Number of pending reverse RPC calls.
    pub fn pending_count(&self) -> usize {
        let map = self.pending.lock().expect("reverse rpc lock");
        map.len()
    }
}

impl Default for ReverseRpcGateway {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Event;

    fn dummy_event() -> Event {
        Event::TextDelta {
            text: "hello".into(),
        }
    }

    #[test]
    fn journal_assigns_monotonic_seq() {
        let mut j = EventJournal::with_default_capacity("s0");
        assert_eq!(j.append(dummy_event()), 1);
        assert_eq!(j.append(dummy_event()), 2);
        assert_eq!(j.append(dummy_event()), 3);
        assert_eq!(j.current_seq(), 4);
    }

    #[test]
    fn journal_replay_from_seq() {
        let mut j = EventJournal::with_default_capacity("s0");
        j.append(dummy_event()); // seq 1
        j.append(dummy_event()); // seq 2
        j.append(dummy_event()); // seq 3

        let replayed = j.replay_from(1).expect("replay");
        assert_eq!(replayed.len(), 2); // seq 2, 3
        assert_eq!(replayed[0].0, 2);
        assert_eq!(replayed[1].0, 3);
    }

    #[test]
    fn journal_full_replay() {
        let mut j = EventJournal::with_default_capacity("s0");
        j.append(dummy_event()); // seq 1
        j.append(dummy_event()); // seq 2
        j.append(dummy_event()); // seq 3

        let replayed = j.replay_from(0).expect("replay from 0");
        assert_eq!(replayed.len(), 3);
    }

    #[test]
    fn journal_wrap_triggers_resync() {
        let mut j = EventJournal::new("s0", 3);
        j.append(dummy_event()); // seq 1
        j.append(dummy_event()); // seq 2
        j.append(dummy_event()); // seq 3
        j.append(dummy_event()); // seq 4 — evicts seq 1

        // seq 1 is gone (min=2, capacity=3, len=3, from_seq=1 < 2) → None
        assert!(j.replay_from(1).is_none());
        // seq 2 still available
        let replayed = j.replay_from(2).expect("replay from 2");
        assert_eq!(replayed.len(), 2); // seq 3, 4
        // seq 0 replays all
        let all = j.replay_from(0).expect("replay from 0");
        assert_eq!(all.len(), 3); // seq 2, 3, 4
    }

    #[test]
    fn journal_empty_replay() {
        let j = EventJournal::with_default_capacity("s0");
        let replayed = j.replay_from(0);
        assert_eq!(replayed, Some(vec![]));
    }

    #[test]
    fn journal_min_max_seq() {
        let mut j = EventJournal::new("s0", 3);
        assert_eq!(j.min_seq(), 0);
        assert_eq!(j.max_seq(), 0);

        j.append(dummy_event()); // seq 1
        assert_eq!(j.min_seq(), 1);
        assert_eq!(j.max_seq(), 1);

        j.append(dummy_event()); // seq 2
        j.append(dummy_event()); // seq 3
        assert_eq!(j.min_seq(), 1);
        assert_eq!(j.max_seq(), 3);

        j.append(dummy_event()); // seq 4 — evicts seq 1
        assert_eq!(j.min_seq(), 2);
        assert_eq!(j.max_seq(), 4);
    }

    #[test]
    fn server_frame_serialization() {
        let frame = ServerFrame::ServerHello {
            version: "1.0".into(),
        };
        let json = serde_json::to_string(&frame).unwrap();
        assert!(json.contains("\"type\":\"server_hello\""));
        assert!(json.contains("\"version\":\"1.0\""));
    }

    #[test]
    fn client_frame_deserialization() {
        let json = r#"{"type":"subscribe","session_id":"s0","last_seq":5}"#;
        let frame: ClientFrame = serde_json::from_str(json).unwrap();
        match frame {
            ClientFrame::Subscribe {
                session_id,
                last_seq,
            } => {
                assert_eq!(session_id, "s0");
                assert_eq!(last_seq, 5);
            }
            _ => panic!("expected Subscribe"),
        }
    }

    // ── ReverseRpcGateway tests ───────────────────────────────────

    #[test]
    fn gateway_approve_first_wins() {
        let gw = ReverseRpcGateway::new();
        let mut rx = gw.register("call-1".into());

        // First approve should succeed
        assert!(gw.handle_approve("call-1", true));
        assert_eq!(rx.try_recv(), Ok(ReverseRpcResult::Approved));

        // Second approve for same call_id should be ignored
        assert!(!gw.handle_approve("call-1", false));
    }

    #[test]
    fn gateway_answer() {
        let gw = ReverseRpcGateway::new();
        let mut rx = gw.register("call-2".into());

        assert!(gw.handle_answer("call-2", "42".into()));
        assert_eq!(
            rx.try_recv(),
            Ok(ReverseRpcResult::Answered("42".into()))
        );

        // Second answer ignored
        assert!(!gw.handle_answer("call-2", "43".into()));
    }

    #[test]
    fn gateway_unknown_call_id() {
        let gw = ReverseRpcGateway::new();
        assert!(!gw.handle_approve("nonexistent", true));
        assert!(!gw.handle_answer("nonexistent", "x".into()));
    }

    #[test]
    fn gateway_pending_count() {
        let gw = ReverseRpcGateway::new();
        assert_eq!(gw.pending_count(), 0);
        gw.register("c1".into());
        assert_eq!(gw.pending_count(), 1);
        gw.register("c2".into());
        assert_eq!(gw.pending_count(), 2);
        gw.handle_approve("c1", true);
        assert_eq!(gw.pending_count(), 1);
        gw.remove("c2");
        assert_eq!(gw.pending_count(), 0);
    }

    #[test]
    fn gateway_timeout_via_drop() {
        let gw = ReverseRpcGateway::new();
        let mut rx = gw.register("call-timeout".into());
        // Drop the sender by removing from the map
        gw.remove("call-timeout");
        // Receiver should get an error (sender dropped without sending)
        assert!(rx.try_recv().is_err());
    }
}
