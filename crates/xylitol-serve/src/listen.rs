use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use salvo::conn::tcp::TcpAcceptor;
use salvo::http::StatusCode;
use salvo::prelude::*;
use salvo::server::ServerHandle;
use salvo::websocket::{Message, WebSocket, WebSocketUpgrade};
use tokio::sync::mpsc;
use xylitol_protocol::RpcMessage;

use crate::registration::{self, Registration};

pub const MUX_CHAN_CAP: usize = 16_384;

pub const CONTENT_TYPE: &str = "application/x-fory-v3";

pub type MuxSink = mpsc::Sender<RpcMessage>;

#[async_trait]
pub trait OccupiedHost: Send + Sync + 'static {
    fn is_shutting_down(&self) -> bool;
    fn mark_shutting_down(&self);
    async fn register_unbound_mux(&self, tx: MuxSink);
    async fn handle_uplink(self: Arc<Self>, bytes: &[u8]) -> Result<Vec<u8>, ()>;
    fn encode_downlink(&self, method: &str, payload: serde_json::Value) -> Result<Vec<u8>, String>;
    async fn prune_mux(self: Arc<Self>, clean_close: bool);
}

pub fn looks_like_fory_envelope(bytes: &[u8]) -> bool {
    bytes.first() == Some(&0x01)
}

#[derive(Clone, Debug)]
pub struct ListenConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ListenConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 18790,
        }
    }
}

pub struct RunningListener {
    handle: ServerHandle,
    gateway: Arc<Gateway>,
    evicted: tokio::sync::watch::Receiver<bool>,
    registration_path: Option<PathBuf>,
}

impl RunningListener {
    pub fn shutdown(&self) {
        if let Some(host) = self.gateway.host() {
            host.mark_shutting_down();
        }
        self.handle.stop_graceful(Duration::from_secs(30));
    }

    pub async fn evicted(&mut self) {
        let _ = self.evicted.changed().await;
    }

    pub fn arm_registration(
        &mut self,
        url: String,
        registration_path: PathBuf,
        interval: Duration,
    ) -> std::io::Result<()> {
        let own = Registration::own(url);
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
        if let Some(host) = self.gateway.host() {
            host.mark_shutting_down();
        }
        if let Some(path) = &self.registration_path {
            registration::remove_registration(path);
        }
        self.handle.stop_graceful(Duration::from_secs(30));
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
    let router = router(gateway.clone());
    let server = Server::new(acceptor);
    let handle = server.handle();
    let shutdown_handle = handle.clone();
    tokio::spawn(async move {
        shutdown_signal(shutdown_handle).await;
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

async fn shutdown_signal(handle: ServerHandle) {
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
    handle.stop_graceful(Duration::from_secs(30));
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Starting,
    Ready,
    Failed,
}

pub struct Gateway {
    phase: AtomicU8,
    host: OnceLock<Arc<dyn OccupiedHost>>,
}

impl Gateway {
    const STARTING: u8 = 0;
    const READY: u8 = 1;
    const FAILED: u8 = 2;

    pub fn starting() -> Arc<Self> {
        Arc::new(Self {
            phase: AtomicU8::new(Self::STARTING),
            host: OnceLock::new(),
        })
    }

    pub fn set_host(&self, host: Arc<dyn OccupiedHost>) {
        let _ = self.host.set(host);
        self.phase.store(Self::READY, Ordering::SeqCst);
    }

    pub fn mark_failed(&self) {
        self.phase.store(Self::FAILED, Ordering::SeqCst);
    }

    pub fn phase(&self) -> Phase {
        match self.phase.load(Ordering::SeqCst) {
            Self::READY => Phase::Ready,
            Self::FAILED => Phase::Failed,
            _ => Phase::Starting,
        }
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
            "status": "starting",
            "retry_after": 1,
        })));
    } else {
        res.render(Json(serde_json::json!({ "status": "failed" })));
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
            Phase::Ready => {
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
        .hoop(GatewayHoop(gateway))
        .push(Router::with_path("healthz").get(healthz))
        .push(Router::with_path("rpc").post(rpc).get(mux_upgrade))
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
    match gateway.phase() {
        Phase::Starting => {
            res.status_code(StatusCode::SERVICE_UNAVAILABLE);
            let _ = res.add_header("retry-after", "1", true);
            res.render(Json(healthz_body(
                "starting",
                serde_json::json!({ "retry_after": 1 }),
            )));
        }
        Phase::Failed => {
            res.status_code(StatusCode::SERVICE_UNAVAILABLE);
            res.render(Json(healthz_body("failed", serde_json::json!({}))));
        }
        Phase::Ready => {
            let stopping = gateway.host().is_some_and(|h| h.is_shutting_down());
            if stopping {
                res.status_code(StatusCode::SERVICE_UNAVAILABLE);
                res.render(Json(healthz_body("stopping", serde_json::json!({}))));
            } else {
                res.status_code(StatusCode::OK);
                res.render(Json(healthz_body("ok", serde_json::json!({}))));
            }
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
    let bytes = match req.payload().await {
        Ok(bytes) => bytes,
        Err(_) => {
            illegal_envelope(res);
            return;
        }
    };
    if !looks_like_fory_envelope(bytes) {
        illegal_envelope(res);
        return;
    }
    match host.handle_uplink(bytes).await {
        Ok(frame) => {
            res.status_code(StatusCode::OK);
            let _ = res.add_header("Content-Type", CONTENT_TYPE, true);
            res.body(frame);
        }
        Err(_) => illegal_envelope(res),
    }
}

fn illegal_envelope(res: &mut Response) {
    res.status_code(StatusCode::BAD_REQUEST);
    res.render(Json(serde_json::json!({
        "error": "illegal envelope"
    })));
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
    WebSocketUpgrade::new()
        .check_origin(mux_origin_allowed)
        .upgrade(req, res, move |ws| handle_mux(ws, host))
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

async fn handle_mux(ws: WebSocket, host: Arc<dyn OccupiedHost>) {
    use std::sync::atomic::{AtomicBool, Ordering};

    use futures::SinkExt;
    let (mut sink, mut stream) = ws.split();
    let (tx, mut rx) = mpsc::channel::<RpcMessage>(MUX_CHAN_CAP);
    let (bin_tx, mut bin_rx) = mpsc::channel::<Vec<u8>>(MUX_CHAN_CAP);
    host.register_unbound_mux(tx).await;
    let clean_close = AtomicBool::new(false);

    let send_loop = async {
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    let Some(msg) = msg else { break };
                    let RpcMessage::ServerRequest { method, payload, .. } = &msg else {
                        continue;
                    };
                    match host.encode_downlink(method, payload.clone()) {
                        Ok(bytes) => {
                            if sink.send(Message::binary(bytes)).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            log::warn!(target: "xylitol::server",
                                "v3 downlink unmapped method={method} skipped: {e}");
                        }
                    }
                }
                bin = bin_rx.recv() => {
                    let Some(frame) = bin else { break };
                    if sink.send(Message::binary(frame)).await.is_err() {
                        break;
                    }
                }
            }
        }
    };
    let recv_loop = async {
        while let Some(item) = stream.next().await {
            let Ok(msg) = item else {
                break;
            };
            if msg.is_close() {
                clean_close.store(true, Ordering::SeqCst);
                break;
            }
            if msg.is_text() {
                continue;
            }
            if !msg.is_binary() {
                continue;
            }
            match host.clone().handle_uplink(msg.as_bytes()).await {
                Ok(frame) => {
                    if bin_tx.send(frame).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };
    tokio::select! {
        _ = send_loop => {}
        _ = recv_loop => {}
    }
    host.prune_mux(clean_close.load(Ordering::SeqCst)).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    use salvo::test::{ResponseExt, TestClient};
    use xylitol_protocol::PROTOCOL_VERSION;
    use xylitol_protocol::WIRE_FORMAT_FORY_V3;
    use xylitol_protocol::v3::{
        ClientRequest, Describe, DescribeResult, Frame, Request, ResponsePayload, ServerResponse,
    };

    struct FakeHost {
        shutting_down: AtomicBool,
    }

    impl FakeHost {
        fn ready() -> Arc<dyn OccupiedHost> {
            Arc::new(Self {
                shutting_down: AtomicBool::new(false),
            })
        }
    }

    #[async_trait]
    impl OccupiedHost for FakeHost {
        fn is_shutting_down(&self) -> bool {
            self.shutting_down.load(Ordering::Relaxed)
        }

        fn mark_shutting_down(&self) {
            self.shutting_down.store(true, Ordering::Relaxed);
        }

        async fn register_unbound_mux(&self, _tx: MuxSink) {}

        async fn handle_uplink(self: Arc<Self>, bytes: &[u8]) -> Result<Vec<u8>, ()> {
            let frame = Frame::from_bytes(bytes).map_err(|_| ())?;
            let Frame::ClientRequest(req) = frame else {
                return Err(());
            };
            let payload = match req.request {
                Request::Describe(_) => Some(ResponsePayload::DescribeResult(DescribeResult {
                    protocol: PROTOCOL_VERSION,
                    formats: vec![WIRE_FORMAT_FORY_V3.into()],
                })),
                _ => None,
            };
            Frame::ServerResponse(ServerResponse {
                rpc_id: req.rpc_id,
                ok: true,
                error: None,
                payload,
                writer_token: None,
            })
            .to_bytes()
            .map_err(|_| ())
        }

        fn encode_downlink(
            &self,
            _method: &str,
            _payload: serde_json::Value,
        ) -> Result<Vec<u8>, String> {
            Err("unmapped".into())
        }

        async fn prune_mux(self: Arc<Self>, _clean_close: bool) {}
    }

    #[tokio::test]
    async fn healthz_ok_json() {
        let gateway = Gateway::starting();
        gateway.set_host(FakeHost::ready());
        let service = Service::new(router(gateway));
        let mut resp = TestClient::get("http://127.0.0.1:0/healthz")
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::OK);
        let body = resp.take_string().await.unwrap();
        assert!(body.contains("ok"), "{body}");
    }

    #[tokio::test]
    async fn post_binary_describe_v3() {
        let gateway = Gateway::starting();
        gateway.set_host(FakeHost::ready());
        let service = Service::new(router(gateway));

        let uplink = Frame::ClientRequest(ClientRequest {
            rpc_id: 21,
            request: Request::Describe(Describe {}),
            writer_token: None,
        })
        .to_bytes()
        .unwrap();

        let mut resp = TestClient::post("http://127.0.0.1:0/rpc")
            .add_header("Content-Type", CONTENT_TYPE, true)
            .body(uplink)
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::OK);

        let body = resp.take_bytes(None).await.unwrap();
        let frame = Frame::from_bytes(&body).expect("v3 response frame");
        let Frame::ServerResponse(resp) = frame else {
            panic!("expected ServerResponse, got {frame:?}")
        };
        assert_eq!(resp.rpc_id, 21);
        assert!(resp.ok);
        match resp.payload {
            Some(ResponsePayload::DescribeResult(d)) => {
                assert_eq!(d.protocol, PROTOCOL_VERSION);
                assert_eq!(d.formats, vec!["fory-v3".to_string()], "{d:?}");
            }
            other => panic!("expected DescribeResult, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn json_rpc_text_is_illegal_envelope() {
        let gateway = Gateway::starting();
        gateway.set_host(FakeHost::ready());
        let service = Service::new(router(gateway));
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "r1",
            "method": "host.describe",
            "params": {}
        });
        let resp = TestClient::post("http://127.0.0.1:0/rpc")
            .json(&body)
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn openapi_and_docs_are_absent() {
        let gateway = Gateway::starting();
        gateway.set_host(FakeHost::ready());
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

        let body = serde_json::json!({
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

        gateway.set_host(FakeHost::ready());
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

    #[tokio::test]
    async fn v1_rest_is_not_mounted() {
        let gateway = Gateway::starting();
        gateway.set_host(FakeHost::ready());
        let service = Service::new(router(gateway));
        let resp = TestClient::post("http://127.0.0.1:0/api/v1/session/x/run")
            .send(&service)
            .await;
        let code = resp.status_code.unwrap();
        assert!(
            code == StatusCode::NOT_FOUND || code == StatusCode::BAD_REQUEST,
            "REST must not be a product path, got {code}"
        );
    }
}
