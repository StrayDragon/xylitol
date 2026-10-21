use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::{broadcast, mpsc, oneshot, watch};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};
use tokio_util::sync::CancellationToken;
use xylitol_protocol::method::{ANSWER_QUESTION, APPROVE_TOOL};
use xylitol_protocol::registry::METHOD_HOST_DESCRIBE;
use xylitol_protocol::{Notification, RpcResult};

use super::json_rpc::{self, Inbound};
use super::{HostClient, HostClientError, MuxStream};

const MUX_PING_INTERVAL: Duration = Duration::from_secs(20);
const MUX_IDLE_TIMEOUT: Duration = Duration::from_secs(40);
const UNARY_TIMEOUT: Duration = Duration::from_secs(30);
const RELOAD_TIMEOUT: Duration = Duration::from_secs(300);
const NEGOTIATE_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLINK_CAP: usize = 16_384;
const OUTGOING_CAP: usize = 64;

#[derive(Clone)]
pub struct HttpWsClient {
    base_url: String,
    ping_interval: Duration,
    idle_timeout: Duration,
    peer: Arc<tokio::sync::Mutex<Option<Arc<SharedWs>>>>,
}

enum Outgoing {
    Text(String),
    Ping,
}

struct SharedWs {
    out: mpsc::Sender<Outgoing>,
    events: broadcast::Sender<Notification>,
    pending: Mutex<HashMap<String, oneshot::Sender<Result<RpcResult, HostClientError>>>>,
    last_err: Mutex<Option<String>>,
    closed: AtomicBool,
    stop: CancellationToken,
    dead: watch::Sender<Option<String>>,
}

impl SharedWs {
    fn new(out: mpsc::Sender<Outgoing>) -> Arc<Self> {
        let (events, _) = broadcast::channel(DOWNLINK_CAP);
        let (dead, _) = watch::channel(None);
        Arc::new(Self {
            out,
            events,
            pending: Mutex::new(HashMap::new()),
            last_err: Mutex::new(None),
            closed: AtomicBool::new(false),
            stop: CancellationToken::new(),
            dead,
        })
    }
}

impl HttpWsClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            ping_interval: MUX_PING_INTERVAL,
            idle_timeout: MUX_IDLE_TIMEOUT,
            peer: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    pub fn with_mux_liveness(mut self, ping_interval: Duration, idle_timeout: Duration) -> Self {
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
        let request = url
            .as_str()
            .into_client_request()
            .map_err(|e| HostClientError::transport(format!("WS request {url}: {e}")))?;
        let (ws_stream, _) = connect_async(request)
            .await
            .map_err(|e| HostClientError::transport(format!("WS connect {url}: {e}")))?;
        let (sink, mut reader) = ws_stream.split();
        let (out_tx, mut out_rx) = mpsc::channel::<Outgoing>(OUTGOING_CAP);
        let peer = SharedWs::new(out_tx.clone());

        let writer_peer = peer.clone();
        let writer_stop = peer.stop.clone();
        tokio::spawn(async move {
            let mut sink = sink;
            loop {
                tokio::select! {
                    _ = writer_stop.cancelled() => break,
                    msg = out_rx.recv() => {
                        let Some(msg) = msg else { break };
                        let send = match msg {
                            Outgoing::Text(text) => sink.send(Message::Text(text.into())).await,
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
        tokio::spawn(async move {
            loop {
                let next = tokio::select! {
                    _ = reader_peer.stop.cancelled() => break,
                    msg = tokio::time::timeout(idle_timeout, reader.next()) => msg,
                };
                let frame = match next {
                    Ok(Some(Ok(frame))) => frame,
                    Ok(Some(Err(_))) | Ok(None) => {
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
                if let Err(reason) = route_frame(&reader_peer, frame) {
                    fail_peer(&reader_peer, &reason);
                    break;
                }
            }
        });

        if let Err(err) = negotiate(&peer).await {
            fail_peer(&peer, &err.to_string());
            return Err(err);
        }
        Ok(peer)
    }
}

fn unary_bound(method: &str) -> Duration {
    if method == "reload_runtime" || method == "reload" {
        RELOAD_TIMEOUT
    } else {
        UNARY_TIMEOUT
    }
}

async fn call(
    peer: &SharedWs,
    id: String,
    name: &str,
    params: &Value,
    bound: Duration,
) -> Result<RpcResult, HostClientError> {
    let (tx, rx) = oneshot::channel();
    peer.pending
        .lock()
        .map_err(|_| HostClientError::transport(format!("{name}: pending lock")))?
        .insert(id.clone(), tx);
    let text = json_rpc::request_text(&id, name, params);
    if peer.out.send(Outgoing::Text(text)).await.is_err() {
        forget(peer, &id);
        return Err(HostClientError::transport(format!("{name}: ws send")));
    }
    match tokio::time::timeout(bound, rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err(HostClientError::transport(format!("{name} cancelled"))),
        Err(_) => {
            forget(peer, &id);
            Err(HostClientError::transport(format!(
                "{name} timed out after {}s",
                bound.as_secs()
            )))
        }
    }
}

fn forget(peer: &SharedWs, id: &str) {
    if let Ok(mut pending) = peer.pending.lock() {
        pending.remove(id);
    }
}

async fn negotiate(peer: &SharedWs) -> Result<(), HostClientError> {
    let result = call(
        peer,
        uuid::Uuid::new_v4().to_string(),
        METHOD_HOST_DESCRIBE,
        &serde_json::json!({}),
        NEGOTIATE_TIMEOUT,
    )
    .await?;
    json_rpc::check_describe(&result)
}

fn route_frame(peer: &SharedWs, frame: Message) -> Result<(), String> {
    match frame {
        Message::Text(text) => match json_rpc::decode_text_frame(text.as_str())? {
            Inbound::Response {
                id: Some(id),
                result,
            } => {
                let waiter = peer.pending.lock().ok().and_then(|mut g| g.remove(&id));
                if let Some(tx) = waiter {
                    let _ = tx.send(Ok(result));
                }
            }
            Inbound::Response { id: None, .. } => {
                log::warn!(target: "xylitol::tui", "host response without id dropped");
            }
            Inbound::Notification(note) => {
                let _ = peer.events.send(note);
            }
            Inbound::Ignored => {}
        },
        Message::Close(_) => return Err("mux closed".into()),
        Message::Binary(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
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
        let peer = self.ensure_peer().await?;
        call(
            &peer,
            uuid::Uuid::new_v4().to_string(),
            method,
            &payload,
            unary_bound(method),
        )
        .await
    }

    async fn respond(&self, call_id: &str, payload: Value) -> Result<(), HostClientError> {
        let mut params = payload;
        if let Some(obj) = params.as_object_mut() {
            obj.entry("call_id")
                .or_insert_with(|| serde_json::json!(call_id));
        }
        let method = if params.get("approved").is_some() {
            APPROVE_TOOL
        } else {
            ANSWER_QUESTION
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
                        Err(broadcast::error::TryRecvError::Lagged(_)) => continue,
                        Err(broadcast::error::TryRecvError::Closed)
                        | Err(broadcast::error::TryRecvError::Empty) => {
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
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => {
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

#[cfg(test)]
mod tests {
    use serde_json::json;
    use xylitol_protocol::{PROTOCOL_VERSION, WIRE_FORMAT_JSONRPC};

    use super::*;

    fn new_peer() -> (Arc<SharedWs>, mpsc::Receiver<Outgoing>) {
        let (out, rx) = mpsc::channel(OUTGOING_CAP);
        (SharedWs::new(out), rx)
    }

    fn park(peer: &SharedWs, id: &str) -> oneshot::Receiver<Result<RpcResult, HostClientError>> {
        let (tx, rx) = oneshot::channel();
        peer.pending
            .lock()
            .expect("pending lock")
            .insert(id.to_string(), tx);
        rx
    }

    fn answer_next(
        peer: Arc<SharedWs>,
        mut out: mpsc::Receiver<Outgoing>,
        result: Value,
    ) -> tokio::task::JoinHandle<Value> {
        tokio::spawn(async move {
            let Some(Outgoing::Text(sent)) = out.recv().await else {
                panic!("request text frame expected");
            };
            let request: Value = serde_json::from_str(&sent).expect("request is json");
            let answer = json!({"jsonrpc": "2.0", "id": request["id"], "result": result});
            assert_eq!(
                route_frame(&peer, Message::text(answer.to_string())),
                Ok(())
            );
            request
        })
    }

    #[test]
    fn binary_frame_is_not_decoded() {
        let (peer, _out) = new_peer();
        let mut waiting = park(&peer, "r1");
        let mut events = peer.events.subscribe();
        let body = br#"{"jsonrpc":"2.0","id":"r1","result":{"ok":true}}"#.to_vec();
        assert_eq!(route_frame(&peer, Message::binary(body)), Ok(()));
        assert!(
            waiting.try_recv().is_err(),
            "binary must not resolve pending"
        );
        assert!(
            peer.pending
                .lock()
                .expect("pending lock")
                .contains_key("r1")
        );
        assert!(events.try_recv().is_err());
    }

    #[test]
    fn binary_notification_is_not_broadcast() {
        let (peer, _out) = new_peer();
        let mut events = peer.events.subscribe();
        let body =
            br#"{"jsonrpc":"2.0","method":"session/resync_required","params":{"session_id":"s0"}}"#
                .to_vec();
        assert_eq!(route_frame(&peer, Message::binary(body)), Ok(()));
        assert!(events.try_recv().is_err());
    }

    #[test]
    fn text_response_resolves_only_its_id() {
        let (peer, _out) = new_peer();
        let mut first = park(&peer, "r1");
        let mut second = park(&peer, "r2");
        let frame = Message::text(r#"{"jsonrpc":"2.0","id":"r2","result":{"seq":2}}"#);
        assert_eq!(route_frame(&peer, frame), Ok(()));
        let resolved = second
            .try_recv()
            .expect("r2 resolved")
            .expect("r2 is a result");
        assert_eq!(resolved, RpcResult::ok_value(json!({"seq": 2})));
        assert!(first.try_recv().is_err(), "r1 stays pending");
        assert!(
            peer.pending
                .lock()
                .expect("pending lock")
                .contains_key("r1")
        );
    }

    #[test]
    fn text_notification_is_broadcast() {
        let (peer, _out) = new_peer();
        let mut events = peer.events.subscribe();
        let frame = Message::text(
            r#"{"jsonrpc":"2.0","method":"session/resync_required","params":{"session_id":"s0"}}"#,
        );
        assert_eq!(route_frame(&peer, frame), Ok(()));
        let note = events.try_recv().expect("notification broadcast");
        assert_eq!(note.method, "session/resync_required");
        assert_eq!(note.params, json!({"session_id": "s0"}));
    }

    #[test]
    fn undecodable_text_ends_the_connection() {
        let (peer, _out) = new_peer();
        assert!(route_frame(&peer, Message::text("{not json")).is_err());
        assert!(route_frame(&peer, Message::text("[]")).is_err());
    }

    #[test]
    fn close_frame_ends_the_connection() {
        let (peer, _out) = new_peer();
        assert_eq!(
            route_frame(&peer, Message::Close(None)),
            Err("mux closed".to_string())
        );
    }

    #[tokio::test]
    async fn call_sends_text_request_and_resolves_by_string_id() {
        let (peer, out) = new_peer();
        let responder = answer_next(peer.clone(), out, json!({"ok": true}));
        let result = call(
            &peer,
            "req-9".into(),
            "get_state",
            &json!({}),
            Duration::from_secs(5),
        )
        .await
        .expect("response resolves the call");
        assert_eq!(result, RpcResult::ok_value(json!({"ok": true})));
        let request = responder.await.expect("responder joined");
        assert_eq!(request["jsonrpc"], "2.0");
        assert_eq!(request["id"], "req-9");
        assert_eq!(request["method"], "get_state");
    }

    #[tokio::test]
    async fn negotiate_rejects_missing_jsonrpc_format() {
        let (peer, out) = new_peer();
        let responder = answer_next(
            peer.clone(),
            out,
            json!({"protocol": PROTOCOL_VERSION, "formats": ["legacy-binary"]}),
        );
        let err = negotiate(&peer).await.expect_err("format mismatch");
        assert!(matches!(err, HostClientError::WireFormatMismatch { .. }));
        let request = responder.await.expect("responder joined");
        assert_eq!(request["method"], METHOD_HOST_DESCRIBE);
    }

    #[tokio::test]
    async fn negotiate_rejects_protocol_mismatch() {
        let (peer, out) = new_peer();
        let responder = answer_next(
            peer.clone(),
            out,
            json!({"protocol": 3, "formats": [WIRE_FORMAT_JSONRPC]}),
        );
        let err = negotiate(&peer).await.expect_err("protocol mismatch");
        assert!(matches!(
            err,
            HostClientError::ProtocolMismatch { got: 3, .. }
        ));
        responder.await.expect("responder joined");
    }

    #[tokio::test]
    async fn negotiate_accepts_matching_describe() {
        let (peer, out) = new_peer();
        let responder = answer_next(
            peer.clone(),
            out,
            json!({"protocol": PROTOCOL_VERSION, "formats": [WIRE_FORMAT_JSONRPC]}),
        );
        negotiate(&peer).await.expect("matching describe accepted");
        responder.await.expect("responder joined");
    }
}
