//! Product attach client: unary + notifications on one `WS /rpc`.
//!
//! Default carrier is the product binary frame. `with_envelope(false)` and
//! `XYLITOL_ENVELOPE_V3` are test injection; Host rejects JSON text as an illegal
//! envelope. Handshake fails closed if the host does not advertise `fory-v3`.

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

use xylitol_protocol::RpcMessage;
use xylitol_protocol::RpcResult;
use xylitol_protocol::codec;

use super::envelope_client::ENVELOPE_FORMAT_ENV;
use super::{HostClient, HostClientError, MuxStream, envelope_client};

/// Product attach client. Unreachable Host is a hard error (no InProcess fallback).
#[derive(Clone)]
pub struct HttpWsClient {
    base_url: String,
    ping_interval: std::time::Duration,
    idle_timeout: std::time::Duration,
    peer: Arc<tokio::sync::Mutex<Option<Arc<SharedWs>>>>,
    /// Test injection: `None` follows `XYLITOL_ENVELOPE_V3` (unset = product on).
    envelope: Option<bool>,
}

/// c2425 mux liveness knobs.
const MUX_PING_INTERVAL: std::time::Duration = std::time::Duration::from_secs(20);
/// Two silent ping windows mark the link half-open.
const MUX_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(40);

enum Outgoing {
    Json(String),
    /// c2834: fory v3 binary 帧(开关开时唯一上行形态)。
    Binary(Vec<u8>),
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
            ping_interval: MUX_PING_INTERVAL,
            idle_timeout: MUX_IDLE_TIMEOUT,
            peer: Arc::new(tokio::sync::Mutex::new(None)),
            envelope: None,
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

    /// Test-only: force the product carrier on/off, bypassing `XYLITOL_ENVELOPE_V3`.
    pub fn with_envelope(mut self, on: bool) -> Self {
        self.envelope = Some(on);
        self
    }

    /// Unset env → product carrier on. Connect-time freeze.
    pub fn envelope_on_for_test(&self) -> bool {
        self.envelope_on()
    }

    fn envelope_on(&self) -> bool {
        self.envelope
            .unwrap_or_else(|| match std::env::var_os(ENVELOPE_FORMAT_ENV) {
                None => true,
                Some(_) => envelope_client::env_enabled(),
            })
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
        let request = url
            .as_str()
            .into_client_request()
            .map_err(|e| HostClientError::transport(format!("WS request {url}: {e}")))?;
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

        let writer_peer = peer.clone();
        tokio::spawn(async move {
            let mut sink = sink;
            loop {
                tokio::select! {
                    _ = stop.cancelled() => break,
                    msg = out_rx.recv() => {
                        let Some(msg) = msg else { break };
                        let send = match msg {
                            Outgoing::Json(text) => sink.send(Message::Text(text.into())).await,
                            Outgoing::Binary(bytes) => {
                                sink.send(Message::Binary(bytes.into())).await
                            }
                            Outgoing::Ping => sink.send(Message::Ping(Default::default())).await,
                        };
                        if send.is_err() {
                            fail_peer(&writer_peer, "mux closed");
                            break;
                        }
                    }
                }
            }
        });

        let ping_interval = self.ping_interval;
        let ping_stop = peer.stop.clone();
        let ping_peer = peer.clone();
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
                            fail_peer(&ping_peer, "mux closed");
                            return;
                        }
                    }
                }
            }
        });

        let idle_timeout = self.idle_timeout;
        let reader_peer = peer.clone();
        // c2834:连接的 v3 模式在 connect 时定格(首 binary 帧决定双方形态,
        // 运行中翻转 env 不追认既有连接)。
        let v3 = self.envelope_on();
        tokio::spawn(async move {
            // v3 Event payload 的会话归属跟踪(Subscribed 帧填充)。
            let mut v3_session_id = String::new();
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
                    Ok(Message::Text(text)) => match codec::decode_str(text.as_str()) {
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
                    },
                    Ok(Message::Binary(bytes)) if v3 => {
                        // c2834 task 3.1:下行 binary 帧(fory v3)。
                        match envelope_client::decode_downlink(&bytes, &mut v3_session_id) {
                            Ok(Some(envelope_client::Downlink::Response { rpc_id, result })) => {
                                if let Some(tx) = reader_peer
                                    .pending
                                    .lock()
                                    .ok()
                                    .and_then(|mut g| g.remove(&rpc_id.to_string()))
                                {
                                    let _ = tx.send(Ok(result));
                                }
                            }
                            Ok(Some(envelope_client::Downlink::Notification(frame))) => {
                                let _ = reader_peer.events.send(frame);
                            }
                            Ok(None) => {}
                            Err(e) => {
                                fail_peer(&reader_peer, &e);
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

        // c2834 task 3.3:开关开时首帧 binary describe 做格式硬闸(旧 host
        // 未宣告 fory-v3 = 致命,不静默降级、不重试风暴)。
        if v3 && let Err(err) = negotiate_envelope(&peer, &url).await {
            fail_peer(&peer, &err.to_string());
            return Err(err);
        }

        Ok(peer)
    }

    async fn rpc_unary(
        &self,
        rpc_id: &str,
        method: &str,
        payload: Value,
    ) -> Result<RpcResult, HostClientError> {
        let bound = if method == "reload_runtime" || method == "reload" {
            std::time::Duration::from_secs(300)
        } else {
            std::time::Duration::from_secs(30)
        };
        // c2834 task 3.1:开关开时上行 fory binary 帧;pending 挂在数值
        // rpc_id(数字形态直通 / 非数字稳定哈希)之下,与 reader 的
        // `rpc_id.to_string()` 回填对齐。JSON 路径仍挂信封 rpcId,不动。
        let (outgoing, pending_key) = if self.envelope_on() {
            let bytes = envelope_client::client_request_bytes(rpc_id, method, &payload)
                .map_err(|e| HostClientError::transport(format!("unary {method}: {e}")))?;
            (
                Outgoing::Binary(bytes),
                envelope_client::stable_rpc_id(rpc_id).to_string(),
            )
        } else {
            let encoded = codec::jsonrpc_request(rpc_id, method, payload).to_string();
            (Outgoing::Json(encoded), rpc_id.to_string())
        };
        let peer = self.ensure_peer().await?;
        let (tx, rx) = oneshot::channel();
        peer.pending
            .lock()
            .map_err(|_| HostClientError::transport("writer pending lock"))?
            .insert(pending_key.clone(), tx);
        peer.out
            .send(outgoing)
            .await
            .map_err(|_| HostClientError::transport(format!("unary {method}: ws send")))?;
        match tokio::time::timeout(bound, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(HostClientError::transport(format!(
                "unary {method} cancelled"
            ))),
            Err(_) => {
                if let Ok(mut pending) = peer.pending.lock() {
                    pending.remove(&pending_key);
                }
                Err(HostClientError::transport(format!(
                    "unary {method} timed out after {}s",
                    bound.as_secs()
                )))
            }
        }
    }
}

/// c2834 task 3.3:v3 格式硬闸协商。
///
/// 开关开的连接以 binary `host.describe` 作为首帧(同时触发服务端把该
/// 连接下行切到 v3),describe 应答未宣告 `fory-v3`(旧 host 无 `formats`
/// 字段 = 仅 jsonrpc)即致命失败——协议版本硬闸语义:不静默降级、不重试
/// 风暴,错误信息指明退出路径。调用方负责 fail_peer 收尾半建连接。
async fn negotiate_envelope(peer: &Arc<SharedWs>, url: &str) -> Result<(), HostClientError> {
    const NEGOTIATE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
    // 协商发生在连接私有期(peer 尚未入库共享),专属 rpc_id 0 无碰撞。
    let bytes = envelope_client::describe_request_bytes()
        .map_err(|e| HostClientError::transport(format!("v3 negotiate: {e}")))?;
    let (tx, rx) = oneshot::channel();
    peer.pending
        .lock()
        .map_err(|_| HostClientError::transport("v3 negotiate: pending lock"))?
        .insert("0".to_string(), tx);
    peer.out
        .send(Outgoing::Binary(bytes))
        .await
        .map_err(|_| HostClientError::transport("v3 negotiate: ws send"))?;
    let result = match tokio::time::timeout(NEGOTIATE_TIMEOUT, rx).await {
        Ok(Ok(result)) => result?,
        Ok(Err(_)) => return Err(HostClientError::transport("v3 negotiate: cancelled")),
        Err(_) => {
            if let Ok(mut pending) = peer.pending.lock() {
                pending.remove("0");
            }
            return Err(HostClientError::transport("v3 negotiate: timed out"));
        }
    };
    if !result.ok {
        return Err(HostClientError::transport(match result.error {
            Some(e) => format!("v3 negotiate: describe error {}: {}", e.code, e.details),
            None => "v3 negotiate: describe failed".into(),
        }));
    }
    if !envelope_client::formats_advertise_envelope(&result) {
        let formats = result
            .value
            .as_ref()
            .and_then(|v| v.get("formats"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        return Err(HostClientError::transport(format!(
            "{ENVELOPE_FORMAT_ENV}=1 but host {url} does not advertise wire format \"fory-v3\" \
             (describe.formats={formats}); refusing downgrade — unset {ENVELOPE_FORMAT_ENV} or upgrade the host"
        )));
    }
    Ok(())
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

    async fn reset_mux(&self) -> Result<(), HostClientError> {
        let mut slot = self.peer.lock().await;
        if let Some(old) = slot.take() {
            old.stop.cancel();
            old.closed.store(true, Ordering::SeqCst);
            let _ = old.dead.send(Some("mux reset for session switch".into()));
        }
        Ok(())
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
