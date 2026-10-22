use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::{Mutex, watch};

use xylitol_protocol::RpcResult;

pub const LEDGER_CAP: usize = 128;

struct Entry {
    method: String,
    payload: Value,
    tx: watch::Sender<Option<RpcResult>>,
    _rx_keepalive: watch::Receiver<Option<RpcResult>>,
}

impl Entry {
    fn new(method: &str, payload: &Value) -> Self {
        let (tx, rx) = watch::channel(None);
        Self {
            method: method.to_string(),
            payload: payload.clone(),
            tx,
            _rx_keepalive: rx,
        }
    }

    fn matches(&self, method: &str, payload: &Value) -> bool {
        self.method == method && &self.payload == payload
    }

    async fn result(&self) -> RpcResult {
        let mut rx = self.tx.subscribe();
        loop {
            if let Some(result) = rx.borrow().as_ref() {
                return result.clone();
            }
            if rx.changed().await.is_err() {
                return RpcResult::error("internal_error", "idempotent first execution vanished");
            }
        }
    }
}

pub struct Reservation {
    entry: Arc<Entry>,
    done: std::cell::Cell<bool>,
}

impl Reservation {
    pub fn complete(self, result: RpcResult) {
        self.done.set(true);
        let _ = self.entry.tx.send(Some(result));
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        if !self.done.get() {
            let _ = self.entry.tx.send(Some(RpcResult::error(
                "internal_error",
                "idempotent execution abandoned",
            )));
        }
    }
}

pub enum Admission {
    Proceed(Reservation),
    Replay(RpcResult),
    Conflict,
}

pub fn conflict_result() -> RpcResult {
    RpcResult::error(
        "idempotency_conflict",
        "request id already admitted with a different method or payload",
    )
}

#[derive(Default)]
pub struct IdempotencyLedger {
    inner: Mutex<LedgerInner>,
}

#[derive(Default)]
struct LedgerInner {
    entries: HashMap<String, Arc<Entry>>,
    order: VecDeque<String>,
}

impl IdempotencyLedger {
    pub async fn admit(&self, rpc_id: Option<&str>, method: &str, payload: &Value) -> Admission {
        let Some(rpc_id) = rpc_id else {
            return Admission::Proceed(Reservation {
                entry: Arc::new(Entry::new(method, payload)),
                done: std::cell::Cell::new(false),
            });
        };
        let duplicate = {
            let mut inner = self.inner.lock().await;
            match inner.entries.get(rpc_id).cloned() {
                Some(existing) => {
                    if !existing.matches(method, payload) {
                        return Admission::Conflict;
                    }
                    existing
                }
                None => {
                    let entry = Arc::new(Entry::new(method, payload));
                    inner.entries.insert(rpc_id.to_string(), entry.clone());
                    inner.order.push_back(rpc_id.to_string());
                    while inner.order.len() > LEDGER_CAP {
                        let Some(oldest) = inner.order.pop_front() else {
                            break;
                        };
                        inner.entries.remove(&oldest);
                    }
                    return Admission::Proceed(Reservation {
                        entry,
                        done: std::cell::Cell::new(false),
                    });
                }
            }
        };
        Admission::Replay(duplicate.result().await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    async fn run(
        ledger: &IdempotencyLedger,
        rpc_id: &str,
        method: &str,
        payload: &Value,
    ) -> RpcResult {
        match ledger.admit(Some(rpc_id), method, payload).await {
            Admission::Proceed(r) => {
                let result = RpcResult::ok_value(json!({ "ran": method }));
                r.complete(result.clone());
                result
            }
            Admission::Replay(r) => r,
            Admission::Conflict => RpcResult::error("idempotency_conflict", "conflict"),
        }
    }

    #[tokio::test]
    async fn first_wins_duplicate_replays() {
        let ledger = IdempotencyLedger::default();
        let p = json!({ "message": "hi" });
        let first = run(&ledger, "r1", "steer", &p).await;
        let second = run(&ledger, "r1", "steer", &p).await;
        assert_eq!(first, second);
        assert_eq!(second.value.unwrap(), json!({ "ran": "steer" }));
    }

    #[tokio::test]
    async fn same_key_different_request_conflicts() {
        let ledger = IdempotencyLedger::default();
        let _ = run(&ledger, "r1", "steer", &json!({ "message": "a" })).await;
        assert!(matches!(
            ledger
                .admit(Some("r1"), "steer", &json!({ "message": "b" }))
                .await,
            Admission::Conflict
        ));
        assert!(matches!(
            ledger.admit(Some("r1"), "abort", &json!({})).await,
            Admission::Conflict
        ));
    }

    #[tokio::test]
    async fn duplicate_waits_for_inflight_and_replays() {
        let ledger = Arc::new(IdempotencyLedger::default());
        let p = json!({ "command": "sleep 1" });
        let Admission::Proceed(reservation) = ledger.admit(Some("r1"), "bash", &p).await else {
            panic!("first must proceed");
        };
        let l2 = ledger.clone();
        let waiter = tokio::spawn(async move {
            match l2.admit(Some("r1"), "bash", &p).await {
                Admission::Replay(r) => r,
                _ => panic!("duplicate must replay"),
            }
        });
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert!(!waiter.is_finished(), "waiter must block while in-flight");
        reservation.complete(RpcResult::ok_value(json!({ "done": true })));
        let replayed = waiter.await.expect("join waiter");
        assert_eq!(replayed.value.unwrap(), json!({ "done": true }));
    }

    #[tokio::test]
    async fn oldest_entry_evicted_beyond_cap() {
        let ledger = IdempotencyLedger::default();
        for i in 0..LEDGER_CAP {
            let _ = run(&ledger, &format!("r{i}"), "get_state", &json!({})).await;
        }
        assert!(matches!(
            ledger.admit(Some("r0"), "get_state", &json!({})).await,
            Admission::Replay(_)
        ));
        let _ = run(&ledger, &format!("r{LEDGER_CAP}"), "get_state", &json!({})).await;
        assert!(matches!(
            ledger.admit(Some("r0"), "get_state", &json!({})).await,
            Admission::Proceed(_)
        ));
    }

    #[tokio::test]
    async fn none_rpc_id_always_proceeds() {
        let ledger = IdempotencyLedger::default();
        for _ in 0..2 {
            assert!(matches!(
                ledger.admit(None, "get_state", &json!({})).await,
                Admission::Proceed(_)
            ));
        }
    }
}
