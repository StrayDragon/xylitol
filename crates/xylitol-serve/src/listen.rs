use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use futures::{Sink, SinkExt, Stream, StreamExt};
use salvo::conn::tcp::TcpAcceptor;
use salvo::http::StatusCode;
use salvo::prelude::*;
use salvo::server::ServerHandle;
use salvo::websocket::{Message, WebSocket, WebSocketUpgrade};
use tokio::sync::mpsc;
use xylitol_protocol::Notification;
use xylitol_protocol::auth;

use crate::lifecycle::{DEFAULT_IDLE_GRACE, ServeLifecycle};

pub const DEFAULT_WS_KEEPALIVE: Duration = Duration::from_secs(20);
use crate::occupancy::downlink_text;
use crate::registration::{self, Registration};
use crate::wire::{CONTENT_TYPE, MAX_BODY_BYTES, Reply};

pub const MUX_CHAN_CAP: usize = 16_384;

const SHUTDOWN_GRACE: Duration = Duration::from_secs(30);

const WS_UNSUPPORTED_DATA: u16 = 1003;
const WS_MESSAGE_TOO_BIG: u16 = 1009;

pub type MuxSink = mpsc::Sender<Notification>;

#[async_trait]
pub trait OccupiedHost: Send + Sync + 'static {
    fn is_shutting_down(&self) -> bool;
    fn mark_shutting_down(&self);
    async fn register_unbound_mux(&self, tx: MuxSink) -> u64;
    async fn dispatch_text(self: Arc<Self>, body: &[u8], mux_id: Option<u64>) -> Reply;
    async fn prune_mux(self: Arc<Self>, clean_close: bool);
}

#[derive(Clone, Debug)]
pub struct ListenConfig {
    pub host: String,
    pub port: u16,
    pub credential: Option<String>,
}

impl Default for ListenConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 18790,
            credential: None,
        }
    }
}

pub struct RunningListener {
    handle: ServerHandle,
    gateway: Arc<Gateway>,
    evicted: tokio::sync::watch::Receiver<bool>,
    registration_path: Option<PathBuf>,
    credential: Option<String>,
}

impl RunningListener {
    fn finalize(&self, remove_file: bool) {
        self.gateway.mark_shutting_down();
        if remove_file && let Some(path) = &self.registration_path {
            registration::remove_registration(path);
        }
        self.handle.stop_graceful(SHUTDOWN_GRACE);
    }

    pub fn shutdown(&self) {
        self.finalize(false);
    }

    pub fn set_lifecycle(&self, lifecycle: ServeLifecycle, idle_timeout: Duration) {
        self.gateway.set_lifecycle(lifecycle, idle_timeout);
    }

    pub fn set_ws_keepalive(&self, keepalive: Option<Duration>) {
        self.gateway.set_ws_keepalive(keepalive);
    }

    pub async fn evicted(&mut self) {
        let _ = self.evicted.changed().await;
    }

    pub fn subscribe_evicted(&self) -> tokio::sync::watch::Receiver<bool> {
        self.evicted.clone()
    }

    pub fn subscribe_idle(&self) -> tokio::sync::watch::Receiver<bool> {
        self.gateway.subscribe_idle()
    }

    pub async fn last_client_idle(&self) {
        self.gateway.wait_until_idle().await;
    }

    pub fn arm_registration(
        &mut self,
        url: String,
        registration_path: PathBuf,
        interval: Duration,
    ) -> std::io::Result<()> {
        let own = Registration::own_with_token(url, self.credential.clone())
            .with_lifecycle(self.gateway.lifecycle());
        registration::write_registration(&registration_path, &own)?;
        self.registration_path = Some(registration_path.clone());
        let (tx, rx) = tokio::sync::watch::channel(false);
        self.evicted = rx;
        tokio::spawn(registration::run_self_check(
            registration_path,
            own,
            interval,
            move || {
                let _ = tx.send(true);
            },
        ));
        Ok(())
    }
}

impl Drop for RunningListener {
    fn drop(&mut self) {
        self.finalize(true);
    }
}

pub async fn bind_listen(
    config: &ListenConfig,
    gateway: Arc<Gateway>,
) -> Result<(RunningListener, u16), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", config.host, config.port);
    let std_listener = std::net::TcpListener::bind(&addr).map_err(|e| {
        Box::new(std::io::Error::new(e.kind(), format!("bind {addr}: {e}")))
            as Box<dyn std::error::Error>
    })?;
    std_listener.set_nonblocking(true)?;
    let local = std_listener.local_addr()?;
    let actual_port = local.port();

    let tokio_listener = tokio::net::TcpListener::from_std(std_listener)?;
    let acceptor = TcpAcceptor::try_from(tokio_listener)?;
    let credential = config
        .credential
        .clone()
        .filter(|token| !token.trim().is_empty());
    gateway.set_auth(credential.clone());
    let router = router(gateway.clone());
    let server = Server::new(acceptor);
    let handle = server.handle();
    let shutdown_handle = handle.clone();
    let shutdown_gateway = gateway.clone();
    tokio::spawn(async move {
        shutdown_signal(shutdown_handle, shutdown_gateway).await;
    });
    tokio::spawn(async move {
        server.serve(router).await;
    });

    let (evict_tx, evict_rx) = tokio::sync::watch::channel(false);
    drop(evict_tx);
    Ok((
        RunningListener {
            handle,
            gateway,
            evicted: evict_rx,
            registration_path: None,
            credential,
        },
        actual_port,
    ))
}

pub async fn serve(
    config: ListenConfig,
    host: Arc<dyn OccupiedHost>,
) -> Result<(RunningListener, u16), Box<dyn std::error::Error>> {
    let gateway = Gateway::starting();
    gateway.set_host(host);
    bind_listen(&config, gateway).await
}

pub async fn serve_registered(
    config: ListenConfig,
    host: Arc<dyn OccupiedHost>,
    registration_path: PathBuf,
) -> Result<(RunningListener, u16), Box<dyn std::error::Error>> {
    let gateway = Gateway::starting();
    gateway.set_host(host);
    let (mut running, port) = bind_listen(&config, gateway).await?;
    running
        .arm_registration(
            format!("http://{}:{port}", config.host),
            registration_path,
            Duration::from_secs(5),
        )
        .map_err(Box::new)?;
    Ok((running, port))
}

async fn shutdown_signal(handle: ServerHandle, gateway: Arc<Gateway>) {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM");
        let mut sigquit = signal(SignalKind::quit()).expect("install SIGQUIT");
        tokio::select! {
            _ = sigterm.recv() => {}
            _ = sigquit.recv() => {}
        }
    };

    #[cfg(not(unix))]
    let terminate = async {
        std::future::pending::<()>().await;
    };

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
    gateway.mark_shutting_down();
    handle.stop_graceful(SHUTDOWN_GRACE);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Phase {
    Starting = 0,
    Ready = 1,
    Stopping = 2,
    Failed = 3,
}

impl Phase {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn from_u8(raw: u8) -> Phase {
        match raw {
            0 => Phase::Starting,
            1 => Phase::Ready,
            2 => Phase::Stopping,
            3 => Phase::Failed,
            _ => panic!("unknown gateway phase byte: {raw}"),
        }
    }

    fn status_str(self) -> &'static str {
        match self {
            Phase::Starting => "starting",
            Phase::Ready => "ok",
            Phase::Stopping => "stopping",
            Phase::Failed => "failed",
        }
    }
}

pub struct Gateway {
    phase: AtomicU8,
    host: OnceLock<Arc<dyn OccupiedHost>>,
    auth: OnceLock<String>,
    mux_clients: AtomicUsize,
    idle_gen: AtomicU64,
    lifecycle: AtomicU8,
    idle_timeout_ms: AtomicU64,
    ws_keepalive_ms: AtomicU64,
    idle_tx: tokio::sync::watch::Sender<bool>,
}

impl Gateway {
    pub fn starting() -> Arc<Self> {
        let (idle_tx, _) = tokio::sync::watch::channel(false);
        Arc::new(Self {
            phase: AtomicU8::new(Phase::Starting.as_u8()),
            host: OnceLock::new(),
            auth: OnceLock::new(),
            mux_clients: AtomicUsize::new(0),
            idle_gen: AtomicU64::new(0),
            lifecycle: AtomicU8::new(ServeLifecycle::Persistent.as_u8()),
            idle_timeout_ms: AtomicU64::new(DEFAULT_IDLE_GRACE.as_millis() as u64),
            ws_keepalive_ms: AtomicU64::new(DEFAULT_WS_KEEPALIVE.as_millis() as u64),
            idle_tx,
        })
    }

    pub fn set_lifecycle(&self, lifecycle: ServeLifecycle, idle_timeout: Duration) {
        self.lifecycle.store(lifecycle.as_u8(), Ordering::SeqCst);
        self.idle_timeout_ms
            .store(idle_timeout.as_millis() as u64, Ordering::SeqCst);
        if lifecycle.is_persistent() {
            let _ = self.idle_tx.send(false);
        }
    }

    pub fn set_ws_keepalive(&self, keepalive: Option<Duration>) {
        let ms = keepalive.map(|d| d.as_millis() as u64).unwrap_or(0);
        self.ws_keepalive_ms.store(ms, Ordering::SeqCst);
    }

    pub fn ws_keepalive(&self) -> Option<Duration> {
        let ms = self.ws_keepalive_ms.load(Ordering::SeqCst);
        (ms > 0).then(|| Duration::from_millis(ms))
    }

    pub fn lifecycle(&self) -> ServeLifecycle {
        ServeLifecycle::from_u8(self.lifecycle.load(Ordering::SeqCst))
    }

    pub fn subscribe_idle(&self) -> tokio::sync::watch::Receiver<bool> {
        self.idle_tx.subscribe()
    }

    pub async fn wait_until_idle(&self) {
        let mut rx = self.subscribe_idle();
        let _ = rx.wait_for(|idle| *idle).await;
    }

    pub fn mux_opened(&self) {
        self.mux_clients.fetch_add(1, Ordering::SeqCst);
        self.idle_gen.fetch_add(1, Ordering::SeqCst);
        let _ = self.idle_tx.send(false);
    }

    pub fn mux_closed(self: &Arc<Self>) {
        let mut current = self.mux_clients.load(Ordering::SeqCst);
        loop {
            if current == 0 {
                return;
            }
            match self.mux_clients.compare_exchange(
                current,
                current - 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
        if current == 1 {
            self.schedule_idle();
        }
    }

    fn schedule_idle(self: &Arc<Self>) {
        if self.lifecycle() != ServeLifecycle::LastClient {
            return;
        }
        let timeout = Duration::from_millis(self.idle_timeout_ms.load(Ordering::SeqCst));
        let generation = self.idle_gen.fetch_add(1, Ordering::SeqCst) + 1;
        let this = Arc::clone(self);
        tokio::spawn(async move {
            if !timeout.is_zero() {
                tokio::time::sleep(timeout).await;
            }
            if this.idle_gen.load(Ordering::SeqCst) == generation
                && this.mux_clients.load(Ordering::SeqCst) == 0
            {
                let _ = this.idle_tx.send(true);
            }
        });
    }

    pub fn set_auth(&self, credential: Option<String>) {
        if let Some(token) = credential {
            let _ = self.auth.set(token);
        }
    }

    pub fn auth(&self) -> Option<&str> {
        self.auth.get().map(String::as_str)
    }

    pub fn set_host(&self, host: Arc<dyn OccupiedHost>) {
        let _ = self.host.set(host);
        self.phase.store(Phase::Ready.as_u8(), Ordering::SeqCst);
    }

    pub fn mark_shutting_down(&self) {
        if let Some(host) = self.host() {
            host.mark_shutting_down();
        }
        self.phase.store(Phase::Stopping.as_u8(), Ordering::SeqCst);
    }

    pub fn mark_failed(&self) {
        self.phase.store(Phase::Failed.as_u8(), Ordering::SeqCst);
    }

    pub fn phase(&self) -> Phase {
        Phase::from_u8(self.phase.load(Ordering::SeqCst))
    }

    pub fn host(&self) -> Option<Arc<dyn OccupiedHost>> {
        self.host.get().cloned()
    }
}

fn gateway_from(depot: &Depot) -> Option<Arc<Gateway>> {
    depot.get_typed::<Arc<Gateway>>().ok().cloned()
}

fn render_unavailable(res: &mut Response, phase: Phase) {
    res.status_code(StatusCode::SERVICE_UNAVAILABLE);
    if phase == Phase::Starting {
        let _ = res.add_header("retry-after", "1", true);
        res.render(Json(serde_json::json!({
            "status": phase.status_str(),
            "retry_after": 1,
        })));
    } else {
        res.render(Json(serde_json::json!({ "status": phase.status_str() })));
    }
}

struct GatewayHoop(Arc<Gateway>);

#[async_trait]
impl Handler for GatewayHoop {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        depot.insert_typed(self.0.clone());
        let is_healthz = req.uri().path().trim_end_matches('/') == "/healthz";
        match self.0.phase() {
            Phase::Ready | Phase::Stopping => {
                ctrl.call_next(req, depot, res).await;
            }
            _ if is_healthz => {
                ctrl.call_next(req, depot, res).await;
            }
            phase => {
                render_unavailable(res, phase);
                ctrl.skip_rest();
            }
        }
    }
}

pub fn router(gateway: Arc<Gateway>) -> Router {
    Router::new()
        .hoop(AuthHoop(gateway.clone()))
        .hoop(GatewayHoop(gateway))
        .push(Router::with_path("healthz").get(healthz))
        .push(Router::with_path("rpc").post(rpc).get(mux_upgrade))
}

struct AuthHoop(Arc<Gateway>);

#[async_trait]
impl Handler for AuthHoop {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        let Some(expected) = self.0.auth() else {
            ctrl.call_next(req, depot, res).await;
            return;
        };
        if presented_token(req)
            .is_some_and(|token| constant_time_eq(token.as_bytes(), expected.as_bytes()))
        {
            ctrl.call_next(req, depot, res).await;
        } else {
            res.status_code(StatusCode::UNAUTHORIZED);
            let _ = res.add_header(
                "WWW-Authenticate",
                format!("Basic realm=\"{}\"", auth::AUTH_REALM),
                true,
            );
            res.render(Json(serde_json::json!({ "error": "unauthorized" })));
            ctrl.skip_rest();
        }
    }
}

fn presented_token(req: &Request) -> Option<String> {
    if let Some(value) = req
        .headers()
        .get(auth::AUTHORIZATION_HEADER)
        .and_then(|value| value.to_str().ok())
        && let Some(token) = auth::token_from_authorization(value)
    {
        return Some(token);
    }
    let is_ws_upgrade =
        req.method().as_str() == "GET" && req.uri().path().trim_end_matches('/') == "/rpc";
    if !is_ws_upgrade {
        return None;
    }
    let query = req.uri().query()?;
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(key, _)| key == auth::AUTH_TOKEN_QUERY)
        .map(|(_, value)| value.into_owned())
        .filter(|token| !token.is_empty())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

fn healthz_body(status: &str, extra: serde_json::Value) -> serde_json::Value {
    let mut v = serde_json::json!({
        "status": status,
        "pid": std::process::id(),
        "version": env!("CARGO_PKG_VERSION"),
    });
    if let (Some(obj), Some(extra)) = (v.as_object_mut(), extra.as_object()) {
        for (k, val) in extra {
            obj.insert(k.clone(), val.clone());
        }
    }
    v
}

#[handler]
async fn healthz(depot: &mut Depot, res: &mut Response) {
    let Some(gateway) = gateway_from(depot) else {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        return;
    };
    let phase = gateway.phase();
    let status = phase.status_str();
    match phase {
        Phase::Starting => {
            res.status_code(StatusCode::SERVICE_UNAVAILABLE);
            let _ = res.add_header("retry-after", "1", true);
            res.render(Json(healthz_body(
                status,
                serde_json::json!({ "retry_after": 1 }),
            )));
        }
        Phase::Ready => {
            res.status_code(StatusCode::OK);
            res.render(Json(healthz_body(status, serde_json::json!({}))));
        }
        Phase::Stopping | Phase::Failed => {
            res.status_code(StatusCode::SERVICE_UNAVAILABLE);
            res.render(Json(healthz_body(status, serde_json::json!({}))));
        }
    }
}

#[handler]
async fn rpc(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    let Some(gateway) = gateway_from(depot) else {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        return;
    };
    let Some(host) = gateway.host() else {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        return;
    };
    let reply = match req.payload_with_max_size(MAX_BODY_BYTES).await {
        Ok(body) => host.dispatch_text(body, None).await,
        Err(salvo::http::ParseError::PayloadTooLarge) => Reply::too_large(),
        Err(_) => Reply::unreadable(),
    };
    write_reply(res, &reply);
}

fn write_reply(res: &mut Response, reply: &Reply) {
    res.status_code(reply.status());
    if let Some(body) = reply.body() {
        let _ = res.add_header("content-type", CONTENT_TYPE, true);
        res.body(body.into_bytes());
    }
}

#[handler]
async fn mux_upgrade(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let Some(gateway) = gateway_from(depot) else {
        return Err(StatusError::internal_server_error());
    };
    let Some(host) = gateway.host() else {
        return Err(StatusError::internal_server_error());
    };
    let gateway = gateway.clone();
    WebSocketUpgrade::new()
        .check_origin(mux_origin_allowed)
        .max_message_size(MAX_BODY_BYTES)
        .upgrade(req, res, move |ws| async move {
            gateway.mux_opened();
            struct IdleGuard(Arc<Gateway>);
            impl Drop for IdleGuard {
                fn drop(&mut self) {
                    self.0.mux_closed();
                }
            }
            let _guard = IdleGuard(gateway.clone());
            let keepalive = gateway.ws_keepalive();
            handle_mux(ws, host, keepalive).await;
        })
        .await
}

fn mux_origin_allowed(origin: Option<&str>) -> bool {
    let Some(raw) = origin.filter(|s| !s.is_empty()) else {
        return true;
    };
    let Ok(url) = url::Url::parse(raw) else {
        return false;
    };
    match url.host() {
        Some(url::Host::Ipv4(addr)) => addr.is_loopback(),
        Some(url::Host::Ipv6(addr)) => addr.is_loopback(),
        Some(url::Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
        None => false,
    }
}

async fn handle_mux(ws: WebSocket, host: Arc<dyn OccupiedHost>, keepalive: Option<Duration>) {
    let (sink, stream) = ws.split();
    serve_mux(host, stream, sink, keepalive).await;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameKind {
    Text,
    Unsupported,
    Ping,
    Control,
}

fn frame_kind(frame: &Message) -> FrameKind {
    if frame.is_text() {
        FrameKind::Text
    } else if frame.is_binary() {
        FrameKind::Unsupported
    } else if frame.is_ping() {
        FrameKind::Ping
    } else {
        FrameKind::Control
    }
}

async fn serve_mux<S, K>(
    host: Arc<dyn OccupiedHost>,
    mut inbound: S,
    mut outbound: K,
    keepalive: Option<Duration>,
) where
    S: Stream<Item = Result<Message, salvo::Error>> + Unpin,
    K: Sink<Message> + Unpin,
{
    let (tx, mut rx) = mpsc::channel::<Notification>(MUX_CHAN_CAP);
    let (reply_tx, mut reply_rx) = mpsc::channel::<String>(MUX_CHAN_CAP);
    let (pong_tx, mut pong_rx) = mpsc::channel::<Vec<u8>>(8);
    let (ping_tx, mut ping_rx) = mpsc::channel::<Vec<u8>>(8);
    let mux_id = host.register_unbound_mux(tx).await;
    let clean_close = AtomicBool::new(false);
    let unsupported = AtomicBool::new(false);
    let inbound_error = AtomicBool::new(false);
    let last_seen = std::sync::Mutex::new(tokio::time::Instant::now());

    let send_loop = async {
        loop {
            tokio::select! {
                note = rx.recv() => {
                    let Some(note) = note else { break };
                    if outbound.send(Message::text(downlink_text(&note))).await.is_err() {
                        break;
                    }
                }
                text = reply_rx.recv() => {
                    let Some(text) = text else { break };
                    if outbound.send(Message::text(text)).await.is_err() {
                        break;
                    }
                }
                payload = pong_rx.recv() => {
                    let Some(payload) = payload else { break };
                    if outbound.send(Message::pong(payload)).await.is_err() {
                        break;
                    }
                }
                payload = ping_rx.recv() => {
                    let Some(payload) = payload else { break };
                    if outbound.send(Message::ping(payload)).await.is_err() {
                        break;
                    }
                }
            }
        }
    };

    let keepalive_loop = async {
        let Some(period) = keepalive else {
            std::future::pending::<()>().await;
            unreachable!()
        };
        let mut tick = tokio::time::interval(period);
        loop {
            tick.tick().await;
            let silent_for = {
                let seen = last_seen.lock().expect("last_seen");
                seen.elapsed()
            };
            if silent_for >= period * 2 {
                break;
            }
            if ping_tx.send(Vec::new()).await.is_err() {
                break;
            }
        }
    };

    let recv_loop = async {
        while let Some(item) = inbound.next().await {
            let Ok(frame) = item else {
                inbound_error.store(true, Ordering::SeqCst);
                break;
            };
            {
                let mut seen = last_seen.lock().expect("last_seen");
                *seen = tokio::time::Instant::now();
            }
            if frame.is_close() {
                clean_close.store(true, Ordering::SeqCst);
                break;
            }
            match frame_kind(&frame) {
                FrameKind::Text => {
                    let reply = host
                        .clone()
                        .dispatch_text(frame.as_bytes(), Some(mux_id))
                        .await;
                    let Some(text) = reply.body() else {
                        continue;
                    };
                    if reply_tx.send(text).await.is_err() {
                        break;
                    }
                }
                FrameKind::Unsupported => {
                    unsupported.store(true, Ordering::SeqCst);
                    break;
                }
                FrameKind::Ping => {
                    if pong_tx.send(frame.as_bytes().to_vec()).await.is_err() {
                        break;
                    }
                }
                FrameKind::Control => {}
            }
        }
    };

    tokio::select! {
        _ = send_loop => {}
        _ = recv_loop => {}
        _ = keepalive_loop => {}
    }

    if unsupported.load(Ordering::SeqCst) {
        while let Ok(text) = reply_rx.try_recv() {
            if outbound.send(Message::text(text)).await.is_err() {
                break;
            }
        }
        let _ = outbound
            .send(Message::close_with(WS_UNSUPPORTED_DATA, "unsupported data"))
            .await;
        let _ = outbound.close().await;
    } else if inbound_error.load(Ordering::SeqCst) {
        let _ = outbound
            .send(Message::close_with(WS_MESSAGE_TOO_BIG, "message too big"))
            .await;
        let _ = outbound.close().await;
    }
    host.prune_mux(clean_close.load(Ordering::SeqCst)).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    use futures::channel::mpsc as futures_mpsc;
    use salvo::test::{ResponseExt, TestClient};
    use serde_json::{Value, json};
    use xylitol_protocol::message::METHOD_NOT_FOUND;

    #[derive(Default)]
    struct FakeHost {
        shutting_down: AtomicBool,
        muxes: std::sync::Mutex<Vec<MuxSink>>,
        dispatched: AtomicUsize,
        last_prune: std::sync::Mutex<Option<bool>>,
    }

    #[async_trait]
    impl OccupiedHost for FakeHost {
        fn is_shutting_down(&self) -> bool {
            self.shutting_down.load(Ordering::Relaxed)
        }

        fn mark_shutting_down(&self) {
            self.shutting_down.store(true, Ordering::Relaxed);
        }

        async fn register_unbound_mux(&self, tx: MuxSink) -> u64 {
            let mut muxes = self.muxes.lock().expect("muxes");
            let id = muxes.len() as u64 + 1;
            muxes.push(tx);
            id
        }

        async fn dispatch_text(self: Arc<Self>, body: &[u8], _mux_id: Option<u64>) -> Reply {
            let dispatched = &self.dispatched;
            crate::occupancy::respond_to_body(body, |req| {
                dispatched.fetch_add(1, Ordering::SeqCst);
                std::future::ready(fake_response(req))
            })
            .await
        }

        async fn prune_mux(self: Arc<Self>, clean_close: bool) {
            *self.last_prune.lock().expect("prune") = Some(clean_close);
        }
    }

    fn fake_response(req: xylitol_protocol::Request) -> xylitol_protocol::Response {
        let outcome = if req.method == "host.describe" {
            xylitol_protocol::Outcome::Result(json!({
                "protocol": xylitol_protocol::PROTOCOL_VERSION,
                "formats": [xylitol_protocol::WIRE_FORMAT_JSONRPC],
            }))
        } else {
            xylitol_protocol::Outcome::Error(xylitol_protocol::ErrorObject::new(
                METHOD_NOT_FOUND,
                "Method not found",
                "unregistered_method",
            ))
        };
        xylitol_protocol::Response {
            id: Some(req.id),
            outcome,
        }
    }

    fn describe_request(id: &str) -> String {
        json!({"jsonrpc": "2.0", "id": id, "method": "host.describe", "params": {}}).to_string()
    }

    fn service(host: &Arc<FakeHost>) -> Service {
        let gateway = Gateway::starting();
        gateway.set_host(host.clone());
        Service::new(router(gateway))
    }

    fn json_of(frame: &Message) -> Value {
        serde_json::from_str(frame.as_str().expect("text frame")).expect("json frame")
    }

    async fn next_frame(rx: &mut futures_mpsc::UnboundedReceiver<Message>) -> Message {
        tokio::time::timeout(Duration::from_secs(5), rx.next())
            .await
            .expect("frame within 5s")
            .expect("connection still open")
    }

    struct WsHarness {
        host: Arc<FakeHost>,
        to_server: futures_mpsc::UnboundedSender<Result<Message, salvo::Error>>,
        from_server: futures_mpsc::UnboundedReceiver<Message>,
        task: tokio::task::JoinHandle<()>,
    }

    fn ws_harness() -> WsHarness {
        ws_harness_with_keepalive(None)
    }

    fn ws_harness_with_keepalive(keepalive: Option<Duration>) -> WsHarness {
        let host = Arc::new(FakeHost::default());
        let (to_server, inbound) = futures_mpsc::unbounded();
        let (outbound, from_server) = futures_mpsc::unbounded();
        let dyn_host: Arc<dyn OccupiedHost> = host.clone();
        let task = tokio::spawn(serve_mux(dyn_host, inbound, outbound, keepalive));
        WsHarness {
            host,
            to_server,
            from_server,
            task,
        }
    }

    #[tokio::test(start_paused = true)]
    async fn keepalive_pings_then_closes_a_silent_client() {
        let mut harness = ws_harness_with_keepalive(Some(Duration::from_millis(100)));
        let ping = next_frame(&mut harness.from_server).await;
        assert!(
            ping.is_ping(),
            "first keepalive tick must send a ping: {ping:?}"
        );
        tokio::time::advance(Duration::from_millis(250)).await;
        for _ in 0..64 {
            tokio::task::yield_now().await;
            if harness.host.last_prune.lock().expect("prune").is_some() {
                break;
            }
            tokio::time::advance(Duration::from_millis(10)).await;
        }
        let pruned = *harness.host.last_prune.lock().expect("prune");
        assert_eq!(
            pruned,
            Some(false),
            "a silent client must be pruned as an unclean disconnect"
        );
        drop(harness);
    }

    #[tokio::test(start_paused = true)]
    async fn keepalive_keeps_a_responsive_client_alive() {
        let mut harness = ws_harness_with_keepalive(Some(Duration::from_millis(100)));
        let ping = next_frame(&mut harness.from_server).await;
        assert!(
            ping.is_ping(),
            "first keepalive tick must send a ping: {ping:?}"
        );

        harness
            .to_server
            .unbounded_send(Ok(Message::text(describe_request("k1"))))
            .expect("send describe");
        let _reply = next_frame(&mut harness.from_server).await;
        tokio::time::advance(Duration::from_millis(90)).await;
        tokio::task::yield_now().await;
        harness
            .to_server
            .unbounded_send(Ok(Message::text(describe_request("k2"))))
            .expect("send describe again");
        let _reply2 = next_frame(&mut harness.from_server).await;

        let pruned = *harness.host.last_prune.lock().expect("prune");
        assert!(pruned.is_none(), "responsive client must stay connected");
        harness.to_server.send(Ok(Message::close())).await.ok();
        drop(harness);
    }

    #[tokio::test]
    async fn keepalive_disabled_sends_no_pings() {
        let mut harness = ws_harness_with_keepalive(None);
        harness
            .to_server
            .unbounded_send(Ok(Message::text(describe_request("k3"))))
            .expect("send describe");
        let reply = next_frame(&mut harness.from_server).await;
        assert!(
            reply.is_text(),
            "a describe reply must arrive without any ping: {reply:?}"
        );
        harness
            .to_server
            .unbounded_send(Ok(Message::ping(vec![1u8])))
            .expect("send client ping");
        let pong = next_frame(&mut harness.from_server).await;
        assert!(
            pong.is_pong(),
            "client ping must still be answered: {pong:?}"
        );
        drop(harness);
    }
    #[tokio::test]
    async fn auth_gate_rejects_missing_and_accepts_discovered_credential() {
        let gateway = Gateway::starting();
        gateway.set_auth(Some("test-token".into()));
        gateway.set_host(Arc::new(FakeHost::default()));
        let service = Service::new(router(gateway));

        let resp = TestClient::get("http://127.0.0.1:0/healthz")
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::UNAUTHORIZED);
        assert!(
            resp.headers()
                .get("www-authenticate")
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.contains("Basic")),
            "missing WWW-Authenticate challenge"
        );

        let resp = TestClient::get("http://127.0.0.1:0/healthz")
            .add_header(
                auth::AUTHORIZATION_HEADER,
                auth::basic_authorization("test-token"),
                true,
            )
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::OK);
    }

    #[test]
    fn phase_round_trips_through_the_byte_representation() {
        for phase in [
            Phase::Starting,
            Phase::Ready,
            Phase::Stopping,
            Phase::Failed,
        ] {
            assert_eq!(Phase::from_u8(phase.as_u8()), phase);
        }
    }

    #[tokio::test]
    async fn dropping_listener_removes_the_registration_file() {
        let dir = std::env::temp_dir().join(format!("xylitol-listen-{}", uuid::Uuid::new_v4()));
        let path = dir.join("serve.json");
        let config = ListenConfig {
            port: 0,
            ..Default::default()
        };
        let gateway = Gateway::starting();
        gateway.set_host(Arc::new(FakeHost::default()));
        let (mut running, _port) = bind_listen(&config, gateway)
            .await
            .expect("bind ephemeral listener");
        running
            .arm_registration(
                "http://127.0.0.1:0".into(),
                path.clone(),
                Duration::from_secs(5),
            )
            .expect("arm registration");
        assert!(path.exists());
        drop(running);
        assert!(
            !path.exists(),
            "registration file must be removed on shutdown"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn lifecycle_round_trips_through_the_byte_representation() {
        for lifecycle in [ServeLifecycle::Persistent, ServeLifecycle::LastClient] {
            assert_eq!(ServeLifecycle::from_u8(lifecycle.as_u8()), lifecycle);
        }
    }

    #[test]
    #[should_panic(expected = "unknown serve lifecycle byte")]
    fn lifecycle_from_u8_rejects_unknown_bytes() {
        let _ = ServeLifecycle::from_u8(7);
    }

    #[test]
    #[should_panic(expected = "unknown gateway phase byte")]
    fn phase_from_u8_rejects_unknown_bytes() {
        let _ = Phase::from_u8(9);
    }

    #[test]
    fn phase_status_strings_are_pinned() {
        assert_eq!(Phase::Starting.status_str(), "starting");
        assert_eq!(Phase::Ready.status_str(), "ok");
        assert_eq!(Phase::Stopping.status_str(), "stopping");
        assert_eq!(Phase::Failed.status_str(), "failed");
    }

    #[tokio::test]
    async fn healthz_reports_stopping_after_shutdown_mark() {
        let gateway = Gateway::starting();
        gateway.set_host(Arc::new(FakeHost::default()));
        let service = Service::new(router(gateway.clone()));
        gateway.mark_shutting_down();
        let mut resp = TestClient::get("http://127.0.0.1:0/healthz")
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::SERVICE_UNAVAILABLE);
        let body = resp.take_string().await.unwrap();
        assert!(body.contains("\"stopping\""), "{body}");
    }

    #[test]
    fn mark_shutting_down_moves_phase_to_stopping() {
        let gateway = Gateway::starting();
        gateway.set_host(Arc::new(FakeHost::default()));
        assert_eq!(gateway.phase(), Phase::Ready);
        gateway.mark_shutting_down();
        assert_eq!(gateway.phase(), Phase::Stopping);
        assert!(gateway.host().is_some_and(|h| h.is_shutting_down()));
    }

    #[test]
    fn constant_time_eq_compares_bytes() {
        assert!(constant_time_eq(b"token", b"token"));
        assert!(!constant_time_eq(b"token", b"other"));
        assert!(!constant_time_eq(b"token", b"token-longer"));
    }

    #[tokio::test]
    async fn healthz_ok_json() {
        let gateway = Gateway::starting();
        gateway.set_host(Arc::new(FakeHost::default()));
        let service = Service::new(router(gateway));
        let mut resp = TestClient::get("http://127.0.0.1:0/healthz")
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::OK);
        let body = resp.take_string().await.unwrap();
        assert!(body.contains("ok"), "{body}");
    }

    #[tokio::test]
    async fn post_describe_answers_200_json_with_the_request_id() {
        let host = Arc::new(FakeHost::default());
        let service = service(&host);
        let mut resp = TestClient::post("http://127.0.0.1:0/rpc")
            .json(&json!({"jsonrpc": "2.0", "id": "d1", "method": "host.describe", "params": {}}))
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("application/json")
        );
        let v: Value = serde_json::from_str(&resp.take_string().await.unwrap()).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], "d1");
        assert_eq!(v["result"]["protocol"], xylitol_protocol::PROTOCOL_VERSION);
        assert_eq!(v["result"]["formats"], json!(["jsonrpc"]));
    }

    #[tokio::test]
    async fn post_notification_is_200_with_empty_body_and_never_runs() {
        let host = Arc::new(FakeHost::default());
        let service = service(&host);
        let mut resp = TestClient::post("http://127.0.0.1:0/rpc")
            .json(&json!({"jsonrpc": "2.0", "method": "host.describe"}))
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::OK);
        assert_eq!(resp.take_string().await.unwrap(), "");
        assert_eq!(host.dispatched.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn post_batch_answers_in_order_and_skips_notifications() {
        let host = Arc::new(FakeHost::default());
        let service = service(&host);
        let mut resp = TestClient::post("http://127.0.0.1:0/rpc")
            .json(&json!([
                {"jsonrpc": "2.0", "id": 1, "method": "host.describe"},
                {"jsonrpc": "2.0", "method": "host.describe"},
                {"jsonrpc": "2.0", "id": "x", "method": "no_such_method"}
            ]))
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::OK);
        let v: Value = serde_json::from_str(&resp.take_string().await.unwrap()).unwrap();
        assert_eq!(v.as_array().map(Vec::len), Some(2), "{v}");
        assert_eq!(v[0]["id"], 1);
        assert!(v[0]["result"].is_object(), "{v}");
        assert_eq!(v[1]["id"], "x");
        assert_eq!(v[1]["error"]["code"], -32601);
        assert_eq!(v[1]["error"]["data"]["code"], "unregistered_method");
        assert_eq!(host.dispatched.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn post_empty_batch_is_400_invalid_request() {
        let host = Arc::new(FakeHost::default());
        let service = service(&host);
        let mut resp = TestClient::post("http://127.0.0.1:0/rpc")
            .raw_json("[]")
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::BAD_REQUEST);
        let v: Value = serde_json::from_str(&resp.take_string().await.unwrap()).unwrap();
        assert_eq!(v["error"]["code"], -32600);
        assert_eq!(host.dispatched.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn post_unparseable_body_is_400_with_null_id() {
        let host = Arc::new(FakeHost::default());
        let service = service(&host);
        let mut resp = TestClient::post("http://127.0.0.1:0/rpc")
            .bytes(vec![0x01, 0xff, 0x22])
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::BAD_REQUEST);
        let v: Value = serde_json::from_str(&resp.take_string().await.unwrap()).unwrap();
        assert_eq!(v["id"], Value::Null);
        assert_eq!(v["error"]["code"], -32700);
    }

    #[tokio::test]
    async fn post_response_only_body_is_400_with_id_echoed() {
        let host = Arc::new(FakeHost::default());
        let service = service(&host);
        let mut resp = TestClient::post("http://127.0.0.1:0/rpc")
            .raw_json(r#"{"jsonrpc":"2.0","id":"r9","result":{}}"#)
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::BAD_REQUEST);
        let v: Value = serde_json::from_str(&resp.take_string().await.unwrap()).unwrap();
        assert_eq!(v["id"], "r9");
        assert_eq!(host.dispatched.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn post_oversized_body_is_413_with_payload_too_large_envelope() {
        let host = Arc::new(FakeHost::default());
        let service = service(&host);
        let mut resp = TestClient::post("http://127.0.0.1:0/rpc")
            .bytes(vec![b' '; MAX_BODY_BYTES + 1])
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::PAYLOAD_TOO_LARGE);
        let v: Value = serde_json::from_str(&resp.take_string().await.unwrap()).unwrap();
        assert_eq!(v["id"], Value::Null);
        assert_eq!(v["error"]["code"], -32600);
        assert_eq!(v["error"]["message"], "payload too large");
        assert_eq!(v["error"]["data"]["code"], "payload_too_large");
        assert_eq!(host.dispatched.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn post_body_at_exact_limit_is_400_parse_error_not_413() {
        let host = Arc::new(FakeHost::default());
        let service = service(&host);
        let mut resp = TestClient::post("http://127.0.0.1:0/rpc")
            .bytes(vec![b' '; MAX_BODY_BYTES])
            .send(&service)
            .await;
        assert_eq!(
            resp.status_code.unwrap(),
            StatusCode::BAD_REQUEST,
            "exactly MAX_BODY_BYTES must not be rejected as 413"
        );
        let v: Value = serde_json::from_str(&resp.take_string().await.unwrap()).unwrap();
        assert_eq!(v["id"], Value::Null);
        assert_eq!(v["error"]["code"], -32700);
        assert_eq!(host.dispatched.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn unregistered_method_is_a_200_business_error() {
        let host = Arc::new(FakeHost::default());
        let service = service(&host);
        let mut resp = TestClient::post("http://127.0.0.1:0/rpc")
            .json(&json!({"jsonrpc": "2.0", "id": "u1", "method": "no_such_method"}))
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::OK);
        let v: Value = serde_json::from_str(&resp.take_string().await.unwrap()).unwrap();
        assert_eq!(v["id"], "u1");
        assert_eq!(v["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn openapi_and_docs_are_absent() {
        let gateway = Gateway::starting();
        gateway.set_host(Arc::new(FakeHost::default()));
        let service = Service::new(router(gateway));
        for path in ["/openapi.json", "/docs"] {
            let resp = TestClient::get(format!("http://127.0.0.1:0{path}"))
                .send(&service)
                .await;
            assert_ne!(
                resp.status_code.unwrap(),
                StatusCode::OK,
                "{path} must not serve debug docs"
            );
        }
    }

    #[tokio::test]
    async fn starting_window_gates_all_but_healthz_then_flips() {
        let gateway = Gateway::starting();
        let service = Service::new(router(gateway.clone()));

        let mut resp = TestClient::get("http://127.0.0.1:0/healthz")
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::SERVICE_UNAVAILABLE);
        let body = resp.take_string().await.unwrap();
        assert!(
            body.contains("starting") && body.contains("retry_after"),
            "{body}"
        );

        let body = json!({
            "jsonrpc": "2.0",
            "id": "r-window",
            "method": "host.describe",
            "params": {}
        });
        let resp = TestClient::post("http://127.0.0.1:0/rpc")
            .json(&body)
            .send(&service)
            .await;
        assert_eq!(
            resp.status_code.unwrap(),
            StatusCode::SERVICE_UNAVAILABLE,
            "unary must be gated in the window"
        );

        gateway.set_host(Arc::new(FakeHost::default()));
        let resp = TestClient::get("http://127.0.0.1:0/healthz")
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::OK, "flip to ready");
    }

    #[test]
    fn mux_origin_native_and_loopback_only() {
        assert!(mux_origin_allowed(None));
        assert!(mux_origin_allowed(Some("http://127.0.0.1:18790")));
        assert!(mux_origin_allowed(Some("http://localhost:5173")));
        assert!(mux_origin_allowed(Some("http://[::1]/")));
        assert!(!mux_origin_allowed(Some("https://evil.example")));
        assert!(!mux_origin_allowed(Some("null")));
    }

    #[test]
    fn frame_kinds_split_text_binary_and_control() {
        assert_eq!(frame_kind(&Message::text("{}")), FrameKind::Text);
        assert_eq!(
            frame_kind(&Message::binary(vec![1u8])),
            FrameKind::Unsupported
        );
        assert_eq!(
            frame_kind(&Message::ping(Vec::<u8>::new())),
            FrameKind::Ping
        );
        assert_eq!(
            frame_kind(&Message::pong(Vec::<u8>::new())),
            FrameKind::Control
        );
    }

    #[tokio::test]
    async fn ws_ping_is_answered_with_pong() {
        let mut h = ws_harness();
        h.to_server
            .unbounded_send(Ok(Message::ping(vec![7u8, 9])))
            .unwrap();
        let frame = next_frame(&mut h.from_server).await;
        assert!(frame.is_pong(), "{frame:?}");
        assert_eq!(frame.as_bytes(), &[7u8, 9]);
        drop(h.to_server);
        h.task.await.unwrap();
    }

    #[tokio::test]
    async fn ws_text_request_is_answered_with_a_text_frame() {
        let mut h = ws_harness();
        h.to_server
            .unbounded_send(Ok(Message::text(describe_request("w1"))))
            .unwrap();
        let frame = next_frame(&mut h.from_server).await;
        assert!(frame.is_text());
        let v = json_of(&frame);
        assert_eq!(v["id"], "w1");
        assert_eq!(v["result"]["protocol"], xylitol_protocol::PROTOCOL_VERSION);
        drop(h.to_server);
        h.task.await.unwrap();
    }

    #[tokio::test]
    async fn ws_parse_error_is_a_text_frame_and_the_connection_stays_open() {
        let mut h = ws_harness();
        h.to_server.unbounded_send(Ok(Message::text("{"))).unwrap();
        let err = json_of(&next_frame(&mut h.from_server).await);
        assert_eq!(err["id"], Value::Null);
        assert_eq!(err["error"]["code"], -32700);

        h.to_server
            .unbounded_send(Ok(Message::text(describe_request("after"))))
            .unwrap();
        let ok = json_of(&next_frame(&mut h.from_server).await);
        assert_eq!(ok["id"], "after");
        drop(h.to_server);
        h.task.await.unwrap();
    }

    #[tokio::test]
    async fn ws_notification_text_gets_no_frame() {
        let mut h = ws_harness();
        h.to_server
            .unbounded_send(Ok(Message::text(
                json!({"jsonrpc": "2.0", "method": "host.describe"}).to_string(),
            )))
            .unwrap();
        h.to_server
            .unbounded_send(Ok(Message::text(describe_request("next"))))
            .unwrap();
        let first = json_of(&next_frame(&mut h.from_server).await);
        assert_eq!(first["id"], "next", "notification must not produce a frame");
        drop(h.to_server);
        h.task.await.unwrap();
        assert_eq!(h.host.dispatched.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn ws_binary_frame_closes_with_1003_and_executes_nothing() {
        let mut h = ws_harness();
        h.to_server
            .unbounded_send(Ok(Message::binary(describe_request("bin").into_bytes())))
            .unwrap();
        let close = next_frame(&mut h.from_server).await;
        assert_eq!(close.close_frame(), Some((1003, "unsupported data")));
        assert!(
            h.from_server.next().await.is_none(),
            "socket ends after close"
        );
        h.task.await.unwrap();
        assert_eq!(h.host.dispatched.load(Ordering::SeqCst), 0);
        assert_eq!(*h.host.last_prune.lock().unwrap(), Some(false));
    }

    #[tokio::test]
    async fn ws_downlink_notification_is_a_text_frame_without_id() {
        let mut h = ws_harness();
        h.to_server
            .unbounded_send(Ok(Message::text(describe_request("sync"))))
            .unwrap();
        let _ = next_frame(&mut h.from_server).await;
        let tx = h.host.muxes.lock().unwrap()[0].clone();
        tx.send(Notification {
            method: "session/resync_required".into(),
            params: json!({"session_id": "s0"}),
        })
        .await
        .unwrap();
        let frame = next_frame(&mut h.from_server).await;
        assert!(frame.is_text());
        assert_eq!(
            json_of(&frame),
            json!({
                "jsonrpc": "2.0",
                "method": "session/resync_required",
                "params": {"session_id": "s0"}
            })
        );
    }

    #[tokio::test]
    async fn ws_close_frame_prunes_as_a_clean_close() {
        let h = ws_harness();
        h.to_server.unbounded_send(Ok(Message::close())).unwrap();
        h.task.await.unwrap();
        assert_eq!(*h.host.last_prune.lock().unwrap(), Some(true));
    }

    #[tokio::test]
    async fn last_client_idle_fires_after_grace() {
        let gateway = Gateway::starting();
        gateway.set_lifecycle(ServeLifecycle::LastClient, Duration::from_millis(40));
        gateway.mux_opened();
        let mut idle = gateway.subscribe_idle();
        gateway.mux_closed();
        let _ = tokio::time::timeout(Duration::from_millis(400), idle.changed())
            .await
            .expect("idle signal");
        assert!(*idle.borrow());
    }

    #[tokio::test]
    async fn last_client_reconnect_cancels_idle() {
        let gateway = Gateway::starting();
        gateway.set_lifecycle(ServeLifecycle::LastClient, Duration::from_millis(80));
        gateway.mux_opened();
        gateway.mux_closed();
        tokio::time::sleep(Duration::from_millis(20)).await;
        gateway.mux_opened();
        tokio::time::sleep(Duration::from_millis(120)).await;
        assert!(!*gateway.subscribe_idle().borrow());
        gateway.mux_closed();
    }

    #[tokio::test]
    async fn persistent_zero_clients_does_not_idle() {
        let gateway = Gateway::starting();
        gateway.set_lifecycle(ServeLifecycle::Persistent, Duration::from_millis(20));
        gateway.mux_opened();
        gateway.mux_closed();
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert!(!*gateway.subscribe_idle().borrow());
    }

    #[tokio::test]
    async fn last_client_never_connected_does_not_idle() {
        let gateway = Gateway::starting();
        gateway.set_lifecycle(ServeLifecycle::LastClient, Duration::from_millis(40));
        tokio::time::sleep(Duration::from_millis(120)).await;
        assert!(!*gateway.subscribe_idle().borrow());
    }

    #[tokio::test]
    async fn wait_until_idle_ignores_mux_opened() {
        let gateway = Gateway::starting();
        gateway.set_lifecycle(ServeLifecycle::LastClient, Duration::from_millis(80));
        let waiting = gateway.clone();
        let wait = tokio::spawn(async move { waiting.wait_until_idle().await });
        tokio::time::sleep(Duration::from_millis(10)).await;
        gateway.mux_opened();
        let raced = tokio::time::timeout(Duration::from_millis(50), wait).await;
        assert!(
            raced.is_err(),
            "mux_opened must not look like last-client idle"
        );
        gateway.mux_closed();
    }
}
