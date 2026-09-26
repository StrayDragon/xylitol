//! Product attach client: JSON-RPC unary + notifications on one `WS /rpc`.

use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};
use tokio_util::sync::CancellationToken;

use crate::protocol::RpcMessage;
use crate::protocol::RpcResult;
use crate::protocol::wire::codec;

use super::{HostClient, HostClientError, MuxStream};

/// Product attach client. Unreachable Host is a hard error (no InProcess fallback).
#[derive(Clone)]
pub struct HttpWsClient {
    base_url: String,
    /// Shared across clones: one TUI = one writer lease.
    writer_token: Arc<Mutex<Option<String>>>,
    ping_interval: std::time::Duration,
    idle_timeout: std::time::Duration,
    peer: Arc<tokio::sync::Mutex<Option<Arc<SharedWs>>>>,
}

/// c2425 mux liveness knobs.
const MUX_PING_INTERVAL: std::time::Duration = std::time::Duration::from_secs(20);
/// Two silent ping windows mark the link half-open.
const MUX_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(40);

enum Outgoing {
    Json(String),
    Ping,
}

struct SharedWs {
    out: mpsc::Sender<Outgoing>,
    events: tokio::sync::broadcast::Sender<RpcMessage>,
    pending: Mutex<HashMap<String, oneshot::Sender<Result<RpcResult, HostClientError>>>>,
    last_err: Mutex<Option<String>>,
    closed: AtomicBool,
    stop: CancellationToken,
    dead: tokio::sync::watch::Sender<Option<String>>,
}

impl HttpWsClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            writer_token: Arc::new(Mutex::new(None)),
            ping_interval: MUX_PING_INTERVAL,
            idle_timeout: MUX_IDLE_TIMEOUT,
            peer: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Test/BDD: inject ping/idle so half-open detection finishes in milliseconds.
    pub fn with_mux_liveness(
        mut self,
        ping_interval: std::time::Duration,
        idle_timeout: std::time::Duration,
    ) -> Self {
        self.ping_interval = ping_interval;
        self.idle_timeout = idle_timeout;
        self
    }

    fn ws_mux_url(&self) -> String {
        let ws_base = self
            .base_url
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        format!("{ws_base}/rpc")
    }

    async fn ensure_peer(&self) -> Result<Arc<SharedWs>, HostClientError> {
        let mut slot = self.peer.lock().await;
        if let Some(peer) = slot.as_ref()
            && !peer.closed.load(Ordering::SeqCst)
        {
            return Ok(peer.clone());
        }
        if let Some(old) = slot.take() {
            old.stop.cancel();
            old.closed.store(true, Ordering::SeqCst);
        }
        let peer = self.connect_peer().await?;
        *slot = Some(peer.clone());
        Ok(peer)
    }

    async fn connect_peer(&self) -> Result<Arc<SharedWs>, HostClientError> {
        let url = self.ws_mux_url();
        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|e| HostClientError::transport(format!("WS request {url}: {e}")))?;
        if let Some(tok) = self.writer_token.lock().ok().and_then(|g| g.clone())
            && let Ok(value) = http::HeaderValue::from_str(&tok)
        {
            request.headers_mut().insert("X-Writer-Token", value);
        }
        let (ws_stream, _) = connect_async(request)
            .await
            .map_err(|e| HostClientError::transport(format!("WS connect {url}: {e}")))?;
        let (sink, mut reader) = ws_stream.split();
        let (out_tx, mut out_rx) = mpsc::channel::<Outgoing>(64);
        let (events, _) = tokio::sync::broadcast::channel(16_384);
        let (dead, _) = tokio::sync::watch::channel(None);
        let stop = CancellationToken::new();
        let peer = Arc::new(SharedWs {
            out: out_tx.clone(),
            events: events.clone(),
            pending: Mutex::new(HashMap::new()),
            last_err: Mutex::new(None),
            closed: AtomicBool::new(false),
            stop: stop.clone(),
            dead,
        });

        tokio::spawn(async move {
            let mut sink = sink;
            loop {
                tokio::select! {
                    _ = stop.cancelled() => break,
                    msg = out_rx.recv() => {
                        let Some(msg) = msg else { break };
                        let send = match msg {
                            Outgoing::Json(text) => sink.send(Message::Text(text.into())).await,
                            Outgoing::Ping => sink.send(Message::Ping(Default::default())).await,
                        };
                        if send.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let ping_interval = self.ping_interval;
        let ping_stop = peer.stop.clone();
        let ping_out = out_tx;
        tokio::spawn(async move {
            let mut ping = tokio::time::interval(ping_interval);
            ping.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            ping.tick().await;
            loop {
                tokio::select! {
                    _ = ping_stop.cancelled() => return,
                    _ = ping.tick() => {
                        if ping_out.send(Outgoing::Ping).await.is_err() {
                            return;
                        }
                    }
                }
            }
        });

        let idle_timeout = self.idle_timeout;
        let reader_peer = peer.clone();
        let writer_token = self.writer_token.clone();
        tokio::spawn(async move {
            loop {
                let msg = tokio::select! {
                    _ = reader_peer.stop.cancelled() => break,
                    msg = tokio::time::timeout(idle_timeout, reader.next()) => msg,
                };
                let msg = match msg {
                    Ok(Some(m)) => m,
                    Ok(None) => {
                        fail_peer(&reader_peer, "mux closed");
                        break;
                    }
                    Err(_) => {
                        fail_peer(
                            &reader_peer,
                            &format!("mux idle: no frames within {}ms", idle_timeout.as_millis()),
                        );
                        break;
                    }
                };
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Some(tok) = serde_json::from_str::<Value>(text.as_str())
                            .ok()
                            .and_then(|v| {
                                v.get("writerToken")
                                    .and_then(Value::as_str)
                                    .map(str::to_string)
                            })
                            && let Ok(mut slot) = writer_token.lock()
                        {
                            *slot = Some(tok);
                        }
                        match codec::decode_str(text.as_str()) {
                            Ok(RpcMessage::ServerResponse { rpc_id, result }) => {
                                if let Some(tx) = reader_peer
                                    .pending
                                    .lock()
                                    .ok()
                                    .and_then(|mut g| g.remove(&rpc_id))
                                {
                                    let _ = tx.send(Ok(result));
                                }
                            }
                            Ok(frame) => {
                                let _ = reader_peer.events.send(frame);
                            }
                            Err(e) => {
                                fail_peer(&reader_peer, &e.to_string());
                                break;
                            }
                        }
                    }
                    Ok(Message::Close(_)) | Err(_) => {
                        fail_peer(&reader_peer, "mux closed");
                        break;
                    }
                    Ok(_) => {}
                }
            }
        });

        Ok(peer)
    }

    async fn rpc_unary(
        &self,
        rpc_id: &str,
        method: &str,
        payload: Value,
    ) -> Result<RpcResult, HostClientError> {
        let encoded = codec::jsonrpc_request(rpc_id, method, payload).to_string();
        let bound = if method == "reload_runtime" || method == "reload" {
            std::time::Duration::from_secs(300)
        } else {
            std::time::Duration::from_secs(30)
        };
        let peer = self.ensure_peer().await?;
        let (tx, rx) = oneshot::channel();
        peer.pending
            .lock()
            .map_err(|_| HostClientError::transport("writer pending lock"))?
            .insert(rpc_id.to_string(), tx);
        peer.out
            .send(Outgoing::Json(encoded))
            .await
            .map_err(|_| HostClientError::transport(format!("unary {method}: ws send")))?;
        match tokio::time::timeout(bound, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(HostClientError::transport(format!(
                "unary {method} cancelled"
            ))),
            Err(_) => {
                if let Ok(mut pending) = peer.pending.lock() {
                    pending.remove(rpc_id);
                }
                Err(HostClientError::transport(format!(
                    "unary {method} timed out after {}s",
                    bound.as_secs()
                )))
            }
        }
    }
}

fn fail_peer(peer: &SharedWs, msg: &str) {
    peer.closed.store(true, Ordering::SeqCst);
    if let Ok(mut slot) = peer.last_err.lock() {
        *slot = Some(msg.to_string());
    }
    let _ = peer.dead.send_replace(Some(msg.to_string()));
    if let Ok(mut pending) = peer.pending.lock() {
        for (_, tx) in pending.drain() {
            let _ = tx.send(Err(HostClientError::transport(msg.to_string())));
        }
    }
    peer.stop.cancel();
}

#[async_trait]
impl HostClient for HttpWsClient {
    async fn unary(&self, method: &str, payload: Value) -> Result<RpcResult, HostClientError> {
        let rpc_id = uuid::Uuid::new_v4().to_string();
        self.rpc_unary(&rpc_id, method, payload).await
    }

    async fn unary_with_id(
        &self,
        rpc_id: &str,
        method: &str,
        payload: Value,
    ) -> Result<RpcResult, HostClientError> {
        self.rpc_unary(rpc_id, method, payload).await
    }

    async fn respond(&self, rpc_id: &str, payload: Value) -> Result<(), HostClientError> {
        let mut params = payload;
        if let Some(obj) = params.as_object_mut() {
            obj.entry("call_id")
                .or_insert_with(|| serde_json::json!(rpc_id));
        }
        let method = if params.get("approved").is_some() {
            "approve_tool"
        } else {
            "answer_question"
        };
        self.unary(method, params).await.map(|_| ())
    }

    async fn mux(&self) -> Result<MuxStream, HostClientError> {
        let peer = self.ensure_peer().await?;
        if peer.closed.load(Ordering::SeqCst) {
            let msg = peer
                .last_err
                .lock()
                .ok()
                .and_then(|g| g.clone())
                .unwrap_or_else(|| "mux closed".into());
            return Err(HostClientError::transport(msg));
        }
        let mut rx = peer.events.subscribe();
        let mut dead = peer.dead.subscribe();
        Ok(Box::pin(async_stream::stream! {
            loop {
                let closed = dead.borrow().clone();
                if closed.is_some() {
                    match rx.try_recv() {
                        Ok(frame) => {
                            yield Ok(frame);
                            continue;
                        }
                        Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::TryRecvError::Closed)
                        | Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                            yield Err(HostClientError::transport(
                                closed.unwrap_or_else(|| "mux closed".into()),
                            ));
                            break;
                        }
                    }
                }
                tokio::select! {
                    changed = dead.changed() => {
                        let _ = changed;
                    }
                    frame = rx.recv() => match frame {
                        Ok(frame) => yield Ok(frame),
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            let msg = {
                                let guard = dead.borrow();
                                guard.clone().unwrap_or_else(|| "mux closed".into())
                            };
                            yield Err(HostClientError::transport(msg));
                            break;
                        }
                    }
                }
            }
        }))
    }
}
