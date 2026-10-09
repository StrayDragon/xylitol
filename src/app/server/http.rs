//! Salvo HTTP + mux routes.
//!
//! - `GET /healthz`
//! - `POST /rpc` (v3 fory unary)
//! - `GET /rpc` (mux WebSocket; v3 binary downlink)

use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

use futures::StreamExt;
use salvo::http::StatusCode;
use salvo::prelude::*;
use salvo::websocket::{Message, WebSocket, WebSocketUpgrade};
use tokio::sync::mpsc;

use crate::app::server::host::{HostState, MUX_CHAN_CAP};
use crate::protocol::RpcMessage;

/// Readiness phase of the listener (c2465 sr-rdy1).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Bound, assembly not finished yet.
    Starting,
    /// Assembled and serving.
    Ready,
    /// Assembly failed (non-retryable).
    Failed,
}

/// Host cell + readiness phase. Created at bind time with an empty cell; the
/// host is filled after assembly and the phase flips to [`Phase::Ready`].
pub struct Gateway {
    phase: AtomicU8,
    host: OnceLock<Arc<HostState>>,
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

    /// Fill the host and flip to ready (idempotent fill: first writer wins).
    pub fn set_host(&self, host: Arc<HostState>) {
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

    pub fn host(&self) -> Option<Arc<HostState>> {
        self.host.get().cloned()
    }
}

fn gateway_from(depot: &Depot) -> Option<Arc<Gateway>> {
    depot.get_typed::<Arc<Gateway>>().ok().cloned()
}

/// 503 with the phase's semantic body (c2465 D3).
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

/// Injects the [`Gateway`] and, once ready, the [`HostState`]. Every route
/// except `/healthz` is short-circuited with a semantic 503 until ready
/// (c2465: requests in the window MUST NOT half-execute).
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
                if let Some(host) = self.0.host() {
                    depot.insert_typed(host);
                }
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

/// Product router (no `/api/v1` REST).
pub fn router(gateway: Arc<Gateway>) -> Router {
    Router::new()
        .hoop(GatewayHoop(gateway))
        .push(Router::with_path("healthz").get(healthz))
        .push(Router::with_path("rpc").post(rpc).get(mux_upgrade))
}

/// Healthz body with the daemon identity fields (c2475 sr-reg1).
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
            let stopping = gateway
                .host()
                .is_some_and(|h| h.shutting_down.load(Ordering::Relaxed));
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
    let writer = req
        .headers()
        .get("X-Writer-Token")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let bytes = match req.payload().await {
        Ok(bytes) => bytes,
        Err(_) => {
            illegal_envelope(res);
            return;
        }
    };
    if !super::wire_v3::looks_like_fory_v3(bytes) {
        illegal_envelope(res);
        return;
    }
    match super::wire_v3::handle_uplink(&host, bytes, writer).await {
        Ok((frame, token)) => {
            res.status_code(StatusCode::OK);
            let _ = res.add_header("Content-Type", super::wire_v3::CONTENT_TYPE, true);
            if let Some(tok) = token {
                let _ = res.add_header("X-Writer-Token", tok, true);
            }
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
    let writer = req
        .headers()
        .get("X-Writer-Token")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    WebSocketUpgrade::new()
        .check_origin(mux_origin_allowed)
        .upgrade(req, res, move |ws| handle_mux(ws, host, writer))
        .await
}

/// Native clients omit Origin. Loopback pages may send one; anything else waits c2303 `--trusted-host`.
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

async fn handle_mux(ws: WebSocket, host: Arc<HostState>, mut writer: Option<String>) {
    use futures::SinkExt;
    let (mut sink, mut stream) = ws.split();
    // Handshake is host.describe result, not a mux ServerHello frame (c2825).
    let (tx, mut rx) = mpsc::channel::<RpcMessage>(MUX_CHAN_CAP);
    let (bin_tx, mut bin_rx) = mpsc::channel::<Vec<u8>>(MUX_CHAN_CAP);
    host.register_unbound_mux(tx).await;

    let send_loop = async {
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    let Some(msg) = msg else { break };
                    let RpcMessage::ServerRequest { method, payload, .. } = &msg else {
                        continue;
                    };
                    match super::wire_v3::downlink_frame(method, payload.clone(), 0) {
                        Ok(bytes) => {
                            if sink.send(Message::binary(bytes)).await.is_err() {
                                break;
                            }
                        }
                        // 未映射 method 降级跳过不断链(r1719)。
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
                break;
            }
            if msg.is_text() {
                // JSON 文本业务帧已退役；丢弃不断链。
                continue;
            }
            if !msg.is_binary() {
                continue;
            }
            match super::wire_v3::handle_uplink(&host, msg.as_bytes(), writer.clone()).await {
                Ok((frame, token)) => {
                    if let Some(tok) = token {
                        writer = Some(tok);
                    }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::server::host::HostState;
    use salvo::test::{ResponseExt, TestClient};

    #[tokio::test]
    async fn healthz_ok_json() {
        let state = HostState::for_test().expect("host");
        let gateway = Gateway::starting();
        gateway.set_host(state);
        let service = Service::new(router(gateway));
        let mut resp = TestClient::get("http://127.0.0.1:0/healthz")
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::OK);
        let body = resp.take_string().await.unwrap();
        assert!(body.contains("ok"), "{body}");
    }

    /// c2834 spec r1902/r1911:v3 binary 上行经 POST /rpc 可服务,应答为
    /// fory 帧;describe 只宣告 fory-v3。
    #[tokio::test]
    async fn post_binary_describe_v3() {
        use crate::protocol::wire::v3::{ClientRequest, Describe, Frame, Request, ResponsePayload};

        let state = HostState::for_test().expect("host");
        let gateway = Gateway::starting();
        gateway.set_host(state);
        let service = Service::new(router(gateway));

        let uplink = Frame::ClientRequest(ClientRequest {
            rpc_id: 21,
            request: Request::Describe(Describe {}),
            writer_token: None,
        })
        .to_bytes()
        .unwrap();

        let mut resp = TestClient::post("http://127.0.0.1:0/rpc")
            .add_header("Content-Type", "application/x-fory-v3", true)
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
                assert_eq!(d.protocol, crate::protocol::wire::PROTOCOL_VERSION);
                assert_eq!(d.formats, vec!["fory-v3".to_string()], "{d:?}");
            }
            other => panic!("expected DescribeResult, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn json_rpc_text_is_illegal_envelope() {
        let state = HostState::for_test().expect("host");
        let gateway = Gateway::starting();
        gateway.set_host(state);
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
        let state = HostState::for_test().expect("host");
        let gateway = Gateway::starting();
        gateway.set_host(state);
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

        gateway.set_host(HostState::for_test().expect("host"));
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
        let state = HostState::for_test().expect("host");
        let gateway = Gateway::starting();
        gateway.set_host(state);
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
