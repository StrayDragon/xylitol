//! Product attach client: JSON-RPC unary + notifications on one `WS /rpc`.
//!
//! c2834 tasks 3.1/3.3(实验):`XYLITOL_WIRE_V3=1|true` 时同一连接改说 fory
//! v3 binary 帧(编解码 helper 见 [`wire_v3_client`]);默认关(JSON 产品路径不动,task 5.1
//! 行为不变。开关开时连接期以 binary describe 做格式硬闸协商——host 未
//! 宣告 `fory-v3` 即致命失败,不静默降级。

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

use super::wire_v3_client::WIRE_V3_ENV;
use super::{HostClient, HostClientError, MuxStream, wire_v3_client};

/// Product attach client. Unreachable Host is a hard error (no InProcess fallback).
#[derive(Clone)]
pub struct HttpWsClient {
    base_url: String,
    /// Shared across clones: one TUI = one writer lease.
    writer_token: Arc<Mutex<Option<String>>>,
    ping_interval: std::time::Duration,
    idle_timeout: std::time::Duration,
    peer: Arc<tokio::sync::Mutex<Option<Arc<SharedWs>>>>,
    /// c2834 task 3.3 实验开关覆盖旋钮:`None` 跟随 `XYLITOL_WIRE_V3` env
    /// (测试注入用,避免跨测试 env 竞态)。
    wire_v3: Option<bool>,
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
            writer_token: Arc::new(Mutex::new(None)),
            ping_interval: MUX_PING_INTERVAL,
            idle_timeout: MUX_IDLE_TIMEOUT,
            peer: Arc::new(tokio::sync::Mutex::new(None)),
            wire_v3: None,
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

    /// Test-only: force the wire v3 experiment on/off, bypassing `XYLITOL_WIRE_V3`.
    pub fn with_wire_v3(mut self, on: bool) -> Self {
        self.wire_v3 = Some(on);
        self
    }

    /// c2834 task 3.3:开关解析(默认关;5.1 翻转被租约竞态 blocker 阻塞)。连接模式在 connect 时定格。
    fn wire_v3_on(&self) -> bool {
        self.wire_v3.unwrap_or_else(wire_v3_client::env_enabled)
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
        let writer_token = self.writer_token.clone();
        // c2834:连接的 v3 模式在 connect 时定格(首 binary 帧决定双方形态,
        // 运行中翻转 env 不追认既有连接)。
        let v3 = self.wire_v3_on();
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
                    Ok(Message::Binary(bytes)) if v3 => {
                        // c2834 task 3.1:下行 binary 帧(fory v3)。
                        match wire_v3_client::decode_downlink(&bytes, &mut v3_session_id) {
                            Ok(Some(wire_v3_client::Downlink::Response { rpc_id, result })) => {
                                if let Some(tok) = &result.writer_token
                                    && let Ok(mut slot) = writer_token.lock()
                                {
                                    *slot = Some(tok.clone());
                                }
                                if let Some(tx) = reader_peer
                                    .pending
                                    .lock()
                                    .ok()
                                    .and_then(|mut g| g.remove(&rpc_id.to_string()))
                                {
                                    let _ = tx.send(Ok(result));
                                }
                            }
                            Ok(Some(wire_v3_client::Downlink::Notification(frame))) => {
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
        if v3 && let Err(err) = negotiate_wire_v3(&peer, &url).await {
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
        let (outgoing, pending_key) = if self.wire_v3_on() {
            let token = self.writer_token.lock().ok().and_then(|g| g.clone());
            let bytes = wire_v3_client::client_request_bytes(rpc_id, method, &payload, token)
                .map_err(|e| HostClientError::transport(format!("unary {method}: {e}")))?;
            (
                Outgoing::Binary(bytes),
                wire_v3_client::stable_rpc_id(rpc_id).to_string(),
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
async fn negotiate_wire_v3(peer: &Arc<SharedWs>, url: &str) -> Result<(), HostClientError> {
    const NEGOTIATE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
    // 协商发生在连接私有期(peer 尚未入库共享),专属 rpc_id 0 无碰撞。
    let bytes = wire_v3_client::describe_request_bytes()
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
    if !wire_v3_client::formats_advertise_v3(&result) {
        let formats = result
            .value
            .as_ref()
            .and_then(|v| v.get("formats"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        return Err(HostClientError::transport(format!(
            "{WIRE_V3_ENV}=1 but host {url} does not advertise wire format \"fory-v3\" \
             (describe.formats={formats}); refusing downgrade — unset {WIRE_V3_ENV} or upgrade the host"
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

/// c2834 task 4.2 双路径对拍:同一会话、同一注入事件流,JSON-RPC 路径与
/// v3 路径各自订阅收集,解码回领域 `Event` 后逐事件比较(spec r1908:
/// 对拍点在领域对象层)。返回逐事件收集结果供断言。
/// 从 mux 流收集 n 个 `session/event` 事件(解码回领域 `Event`)。
#[cfg(test)]
async fn collect_events(mux: &mut MuxStream, n: usize) -> Vec<crate::protocol::Event> {
    use futures::StreamExt;
    let mut out = Vec::new();
    while let Ok(Some(Ok(frame))) =
        tokio::time::timeout(std::time::Duration::from_secs(3), mux.next()).await
    {
        if let crate::protocol::RpcMessage::ServerRequest {
            method, payload, ..
        } = &frame
            && method == "session/event"
            && let Some(ev) = payload
                .get("event")
                .cloned()
                .and_then(|v| serde_json::from_value::<crate::protocol::Event>(v).ok())
        {
            out.push(ev);
            if out.len() >= n {
                break;
            }
        }
    }
    out
}

#[cfg(test)]
pub(crate) async fn dual_rail_event_parity()
-> Result<Vec<(crate::protocol::Event, crate::protocol::Event)>, String> {
    use crate::app::server::host::HostState;
    use crate::app::server::runtime::{ServerConfig, serve};

    let host = HostState::for_test().map_err(|e| format!("host: {e}"))?;
    let (_running, port) = serve(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: None,
        },
        host.clone(),
    )
    .await
    .map_err(|e| format!("serve: {e}"))?;
    let url = format!("http://127.0.0.1:{port}");

    // 路径 A:JSON-RPC(默认);路径 B:v3 binary(实验开关注入旋钮)。
    let client_json = HttpWsClient::new(url.clone());
    let client_v3 = HttpWsClient::new(url).with_wire_v3(true);
    let mut mux_json = client_json
        .mux()
        .await
        .map_err(|e| format!("json mux: {e}"))?;
    let mut mux_v3 = client_v3.mux().await.map_err(|e| format!("v3 mux: {e}"))?;

    // 双路径先做一次 describe 握手(v3 连接借此协商并切下行 binary)。
    let d = client_v3
        .unary("host.describe", serde_json::json!({}))
        .await
        .map_err(|e| format!("v3 describe: {e}"))?;
    if !d.ok {
        return Err(format!("v3 describe failed: {:?}", d.error));
    }

    let session = "s-parity";
    for client in [&client_json, &client_v3] {
        let r = client
            .unary(
                "subscribe",
                serde_json::json!({"session_id": session, "last_seq": 0}),
            )
            .await
            .map_err(|e| format!("subscribe: {e}"))?;
        if !r.ok {
            return Err(format!("subscribe failed: {:?}", r.error));
        }
    }

    // 注入代表事件流(流式增量、工具带动态块、投影快照)。
    let injected = vec![
        crate::protocol::Event::TextDelta {
            text: "parity chunk".into(),
        },
        crate::protocol::Event::ToolStart {
            id: "t-parity".into(),
            name: "grep".into(),
            args: serde_json::json!({"pattern": "TODO"}),
        },
        crate::protocol::Event::TurnEnd { turn_index: 1 },
    ];
    for ev in &injected {
        host.slot(session).await.append_and_push(ev.clone()).await;
    }

    let got_json = collect_events(&mut mux_json, injected.len()).await;
    let got_v3 = collect_events(&mut mux_v3, injected.len()).await;
    if got_json.len() != injected.len() {
        return Err(format!("json path collected {} events", got_json.len()));
    }
    if got_v3.len() != injected.len() {
        return Err(format!(
            "v3 path collected {} events: {got_v3:?}",
            got_v3.len()
        ));
    }
    Ok(got_json.into_iter().zip(got_v3).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// c2834 task 4.2:双路径事件流对拍(领域对象层等价,spec r1908)。
    #[tokio::test]
    async fn dual_rail_event_stream_parity() {
        let pairs = dual_rail_event_parity().await.expect("parity run");
        for (json_ev, v3_ev) in pairs {
            assert_eq!(json_ev, v3_ev, "双路径事件领域等价失败");
        }
    }

    /// c2834 task 4.2:unary 应答双路径等价(get_messages 冷恢复投影,spec
    /// r1908/r1810):JSON 路径 result 与 v3 路径 RawOk 原文出自同一
    /// dispatch,语义等价(空会话基准 + 结构断言)。
    #[tokio::test]
    async fn dual_rail_get_messages_equivalence() {
        use crate::app::server::host::HostState;
        use crate::app::server::runtime::{ServerConfig, serve};
        use crate::protocol::wire::v3::{
            ClientRequest, Command as V3Command, Frame, GetMessages, Request, ResponsePayload,
        };

        let host = HostState::for_test().expect("host");
        let (_running, port) = serve(
            ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
                sessions_dir: None,
                registration_path: None,
            },
            host,
        )
        .await
        .expect("serve");
        let url = format!("http://127.0.0.1:{port}/rpc");

        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "get_messages",
            "params": {"type": "get_messages"},
        });
        let json_resp: serde_json::Value = reqwest::Client::new()
            .post(&url)
            .json(&body)
            .send()
            .await
            .expect("json post")
            .json()
            .await
            .expect("json body");
        let json_result = json_resp.get("result").cloned().expect("json result");

        let uplink = Frame::ClientRequest(ClientRequest {
            rpc_id: 1,
            request: Request::Command(V3Command::GetMessages(GetMessages {})),
            writer_token: None,
        })
        .to_bytes()
        .expect("encode");
        let v3_bytes = reqwest::Client::new()
            .post(&url)
            .header("content-type", "application/x-fory-v3")
            .body(uplink)
            .send()
            .await
            .expect("v3 post")
            .bytes()
            .await
            .expect("v3 body");
        let frame = crate::protocol::wire::v3::Frame::from_bytes(&v3_bytes).expect("v3 decode");
        let crate::protocol::wire::v3::Frame::ServerResponse(resp) = frame else {
            unreachable!()
        };
        assert!(resp.ok, "{:?}", resp.error);
        match resp.payload {
            Some(ResponsePayload::RawOk(raw)) => {
                let v3_result: serde_json::Value =
                    serde_json::from_str(&raw.json).expect("RawOk 原文为合法 JSON");
                assert_eq!(v3_result, json_result, "双路径 get_messages 语义等价");
            }
            other => panic!("expected RawOk, got {other:?}"),
        }
    }
    use serde_json::json;

    /// task 3.3:默认(未设 env)开关为关——JSON 路径不受影响;覆盖旋钮
    /// 可双向钉死(5.1 产品面切换在 TUI attach 入口,库默认翻转留作
    /// 租约竞态专项后,见 tasks.md 备注)。
    #[test]
    fn wire_v3_defaults_off() {
        assert!(!HttpWsClient::new("http://127.0.0.1:1").wire_v3_on());
        assert!(
            HttpWsClient::new("http://127.0.0.1:1")
                .with_wire_v3(true)
                .wire_v3_on()
        );
        assert!(
            !HttpWsClient::new("http://127.0.0.1:1")
                .with_wire_v3(false)
                .wire_v3_on()
        );
    }

    /// task 3.3 (c):开关开 + 真服务端——binary describe 握手(格式硬闸
    /// 通过)经 v3 帧 roundtrip,应答还原为 JSON 轨 describe 形状。
    #[tokio::test]
    async fn v3_end_to_end_describe_binary_handshake() {
        use crate::app::server::host::HostState;
        use crate::app::server::runtime::{ServerConfig, serve};

        let host = HostState::for_test().expect("host");
        let (running, port) = serve(
            ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
                sessions_dir: None,
                registration_path: None,
            },
            host,
        )
        .await
        .expect("serve");
        let client = HttpWsClient::new(format!("http://127.0.0.1:{port}")).with_wire_v3(true);
        let result = client
            .unary("host.describe", json!({}))
            .await
            .expect("v3 describe");
        assert!(result.ok, "{result:?}");
        let value = result.value.expect("describe value");
        assert_eq!(value["protocol"], crate::protocol::wire::PROTOCOL_VERSION);
        let formats = value["formats"].as_array().expect("formats");
        assert!(formats.iter().any(|f| f == "fory-v3"), "{formats:?}");
        running.shutdown();
    }

    /// task 3.3 (c):host 未宣告 `fory-v3`(仅 jsonrpc 的 stub)→ 连接致命
    /// 失败:错误指明 `XYLITOL_WIRE_V3` 与不降级语义,不静默回落 JSON。
    #[tokio::test]
    async fn v3_hard_gate_fatals_without_fory_v3_advertisement() {
        use crate::protocol::wire::v3::{
            ClientRequest as V3ClientRequest, DescribeResult, Frame as V3Frame, ResponsePayload,
            ServerResponse as V3ServerResponse,
        };

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let stub = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let mut ws = tokio_tungstenite::accept_async(stream)
                .await
                .expect("upgrade");
            // 首帧 binary describe(rpc_id 0)→ 回仅 jsonrpc 的 v3 应答。
            while let Some(Ok(msg)) = futures::StreamExt::next(&mut ws).await {
                if !msg.is_binary() {
                    continue;
                }
                let bytes = msg.into_data();
                let V3Frame::ClientRequest(V3ClientRequest { rpc_id, .. }) =
                    V3Frame::from_bytes(&bytes).expect("v3 uplink frame")
                else {
                    continue;
                };
                let resp = V3Frame::ServerResponse(V3ServerResponse {
                    rpc_id,
                    ok: true,
                    error: None,
                    payload: Some(ResponsePayload::DescribeResult(DescribeResult {
                        protocol: crate::protocol::wire::PROTOCOL_VERSION,
                        formats: vec!["jsonrpc".into()],
                    })),
                    writer_token: None,
                })
                .to_bytes()
                .expect("encode");
                ws.send(Message::Binary(resp.into())).await.expect("reply");
                break;
            }
        });

        let client = HttpWsClient::new(format!("http://127.0.0.1:{port}")).with_wire_v3(true);
        let err = client
            .unary("host.describe", json!({}))
            .await
            .expect_err("hard gate must fail the connection");
        let msg = err.to_string();
        assert!(
            msg.contains("does not advertise wire format \"fory-v3\""),
            "{msg}"
        );
        assert!(msg.contains("XYLITOL_WIRE_V3"), "{msg}");
        stub.await.expect("stub drained");
    }
}
