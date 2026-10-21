use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::sync::oneshot;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use xylitol_protocol::Event;
use xylitol_protocol::RpcMessage;

pub fn downlink_server_request(method: impl Into<String>, payload: Value) -> RpcMessage {
    RpcMessage::ServerRequest {
        rpc_id: uuid::Uuid::new_v4().to_string(),
        method: method.into(),
        payload,
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ServerFrame {
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ClientFrame {
    Subscribe { session_id: String, last_seq: u64 },
    ApproveTool { call_id: String, approved: bool },
    AnswerQuestion { call_id: String, answer: String },
    Ping,
}

pub const DEFAULT_JOURNAL_CAPACITY: usize = 10_000;

pub struct EventJournal {
    session_id: String,
    capacity: usize,
    next_seq: AtomicU64,
    buffer: VecDeque<(u64, Event)>,
}

impl EventJournal {
    pub fn new(session_id: impl Into<String>, capacity: usize) -> Self {
        Self {
            session_id: session_id.into(),
            capacity,
            next_seq: AtomicU64::new(1),
            buffer: VecDeque::with_capacity(capacity),
        }
    }

    pub fn with_default_capacity(session_id: impl Into<String>) -> Self {
        Self::new(session_id, DEFAULT_JOURNAL_CAPACITY)
    }

    pub fn append(&mut self, event: Event) -> u64 {
        let seq = self.next_seq.fetch_add(1, Ordering::SeqCst);
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back((seq, event));
        seq
    }

    pub fn current_seq(&self) -> u64 {
        self.next_seq.load(Ordering::SeqCst)
    }

    pub fn max_seq(&self) -> u64 {
        self.buffer.back().map(|(seq, _)| *seq).unwrap_or(0)
    }

    pub fn min_seq(&self) -> u64 {
        self.buffer.front().map(|(seq, _)| *seq).unwrap_or(0)
    }

    pub fn is_full(&self) -> bool {
        self.buffer.len() >= self.capacity
    }

    pub fn replay_from(&self, from_seq: u64) -> Option<Vec<(u64, Event)>> {
        let min = self.min_seq();
        if from_seq > 0 && from_seq < min && self.buffer.len() >= self.capacity {
            return None;
        }
        Some(
            self.buffer
                .iter()
                .filter(|(seq, _)| *seq > from_seq)
                .cloned()
                .collect(),
        )
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

pub const REVERSE_RPC_TIMEOUT: Duration = Duration::from_secs(60);

pub struct ReverseRpcGateway {
    pending: Mutex<HashMap<String, oneshot::Sender<ReverseRpcResult>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReverseRpcResult {
    Approved,
    Denied,
    Answered(String),
    Timeout,
}

impl ReverseRpcGateway {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }

    pub fn register(&self, call_id: String) -> oneshot::Receiver<ReverseRpcResult> {
        let (tx, rx) = oneshot::channel();
        let mut map = self.pending.lock().expect("reverse rpc lock");
        map.insert(call_id, tx);
        rx
    }

    pub fn handle_approve(&self, call_id: &str, approved: bool) -> bool {
        let mut map = self.pending.lock().expect("reverse rpc lock");
        if let Some(tx) = map.remove(call_id) {
            let result = if approved {
                ReverseRpcResult::Approved
            } else {
                ReverseRpcResult::Denied
            };
            let _ = tx.send(result);
            true
        } else {
            false
        }
    }

    pub fn handle_answer(&self, call_id: &str, answer: String) -> bool {
        let mut map = self.pending.lock().expect("reverse rpc lock");
        if let Some(tx) = map.remove(call_id) {
            let _ = tx.send(ReverseRpcResult::Answered(answer));
            true
        } else {
            false
        }
    }

    pub fn handle_respond(&self, rpc_id: &str, payload: &Value) -> bool {
        if let Some(approved) = payload.get("approved").and_then(|v| v.as_bool()) {
            return self.handle_approve(rpc_id, approved);
        }
        if let Some(answer) = payload.get("answer").and_then(|v| v.as_str()) {
            return self.handle_answer(rpc_id, answer.to_string());
        }
        false
    }

    pub fn remove(&self, call_id: &str) {
        let mut map = self.pending.lock().expect("reverse rpc lock");
        map.remove(call_id);
    }

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
    use xylitol_protocol::Event;
    use xylitol_protocol::codec;

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
        j.append(dummy_event());
        j.append(dummy_event());
        j.append(dummy_event());

        let replayed = j.replay_from(1).expect("replay");
        assert_eq!(replayed.len(), 2);
        assert_eq!(replayed[0].0, 2);
        assert_eq!(replayed[1].0, 3);
    }

    #[test]
    fn journal_full_replay() {
        let mut j = EventJournal::with_default_capacity("s0");
        j.append(dummy_event());
        j.append(dummy_event());
        j.append(dummy_event());

        let replayed = j.replay_from(0).expect("replay from 0");
        assert_eq!(replayed.len(), 3);
    }

    #[test]
    fn journal_wrap_triggers_resync() {
        let mut j = EventJournal::new("s0", 3);
        j.append(dummy_event());
        j.append(dummy_event());
        j.append(dummy_event());
        j.append(dummy_event());

        assert!(j.replay_from(1).is_none());
        let replayed = j.replay_from(2).expect("replay from 2");
        assert_eq!(replayed.len(), 2);
        let all = j.replay_from(0).expect("replay from 0");
        assert_eq!(all.len(), 3);
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

        j.append(dummy_event());
        assert_eq!(j.min_seq(), 1);
        assert_eq!(j.max_seq(), 1);

        j.append(dummy_event());
        j.append(dummy_event());
        assert_eq!(j.min_seq(), 1);
        assert_eq!(j.max_seq(), 3);

        j.append(dummy_event());
        assert_eq!(j.min_seq(), 2);
        assert_eq!(j.max_seq(), 4);
    }

    #[test]
    fn four_quadrant_type_tag_is_not_a_wire_frame() {
        let json = codec::encode_to_string(&RpcMessage::ServerRequest {
            rpc_id: "n1".into(),
            method: "session/event".into(),
            payload: serde_json::json!({}),
        })
        .unwrap();
        assert!(json.contains("\"type\":\"server-request\""), "{json}");
        assert!(
            codec::decode_str(&json).is_err(),
            "handshake is host.describe, not a mux type tag"
        );
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

    #[test]
    fn gateway_approve_first_wins() {
        let gw = ReverseRpcGateway::new();
        let mut rx = gw.register("call-1".into());

        assert!(gw.handle_approve("call-1", true));
        assert_eq!(rx.try_recv(), Ok(ReverseRpcResult::Approved));

        assert!(!gw.handle_approve("call-1", false));
    }

    #[test]
    fn gateway_answer() {
        let gw = ReverseRpcGateway::new();
        let mut rx = gw.register("call-2".into());

        assert!(gw.handle_answer("call-2", "42".into()));
        assert_eq!(rx.try_recv(), Ok(ReverseRpcResult::Answered("42".into())));

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
        gw.remove("call-timeout");
        assert!(rx.try_recv().is_err());
    }
}
