//! Salvo HTTP + mux routes (four-quadrant product carrier).
//!
//! - `GET /healthz`
//! - `GET /openapi.json` (unary debug document, built from the method table)
//! - `GET /docs` (Scalar debug UI — the only debug UI; points at `/openapi.json`)
//! - `POST /api/respond`
//! - `POST /api/{method}` (registered unary only)
//! - `GET /api/events.mux` (WebSocket downlink only)

use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

use futures::StreamExt;
use salvo::http::StatusCode;
use salvo::prelude::*;
use salvo::websocket::{Message, WebSocket, WebSocketUpgrade};
use tokio::sync::mpsc;

use crate::app::server::host::{HostState, MUX_CHAN_CAP, handle_unary};
use crate::protocol::RpcMessage;
use crate::protocol::wire::method::is_unary_method;

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

fn host_from(depot: &Depot) -> Option<Arc<HostState>> {
    depot.get_typed::<Arc<HostState>>().ok().cloned()
}

/// Product router (no `/api/v1` REST).
pub fn router(gateway: Arc<Gateway>) -> Router {
    Router::new()
        .hoop(GatewayHoop(gateway))
        .push(Router::with_path("healthz").get(healthz))
        .push(Router::with_path("openapi.json").get(openapi_json))
        .push(
            salvo_oapi::scalar::Scalar::new("/openapi.json")
                .title("xylitol unary debug API")
                .into_router("docs"),
        )
        .push(Router::with_path("api/respond").post(respond))
        .push(Router::with_path("api/events.mux").get(mux_upgrade))
        .push(Router::with_path("api/{method}").post(unary))
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

/// sr-oapi1: static OpenAPI 3.1 debug document (built from the method table).
#[handler]
async fn openapi_json(res: &mut Response) {
    res.status_code(StatusCode::OK);
    res.render(Text::Plain(super::oapi::openapi_doc()));
}

#[handler]
async fn unary(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    let Some(host) = host_from(depot) else {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        return;
    };
    let path_method = req.param::<String>("method").unwrap_or_default();
    if path_method == "respond" {
        res.status_code(StatusCode::NOT_FOUND);
        return;
    }
    if !is_unary_method(&path_method) {
        res.status_code(StatusCode::NOT_FOUND);
        res.render(Json(serde_json::json!({
            "error": "unregistered method"
        })));
        return;
    }

    let body: RpcMessage = match req.parse_json().await {
        Ok(b) => b,
        Err(_) => {
            illegal_envelope(res);
            return;
        }
    };
    let RpcMessage::ClientRequest {
        rpc_id,
        method,
        payload,
        writer_token,
    } = body
    else {
        illegal_envelope(res);
        return;
    };
    if method != path_method {
        illegal_envelope(res);
        return;
    }

    let result = handle_unary(&host, Some(&rpc_id), &method, payload, writer_token).await;
    res.status_code(StatusCode::OK);
    res.render(Json(RpcMessage::ServerResponse { rpc_id, result }));
}

#[handler]
async fn respond(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    let Some(host) = host_from(depot) else {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        return;
    };
    let body: RpcMessage = match req.parse_json().await {
        Ok(b) => b,
        Err(_) => {
            illegal_envelope(res);
            return;
        }
    };
    let RpcMessage::ClientResponse { rpc_id, payload } = body else {
        illegal_envelope(res);
        return;
    };
    let _ = host.respond(&rpc_id, payload).await;
    res.status_code(StatusCode::OK);
    res.render(Json(serde_json::json!({ "ok": true })));
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
    let Some(host) = host_from(depot) else {
        return Err(StatusError::internal_server_error());
    };
    WebSocketUpgrade::new()
        .check_origin(mux_origin_allowed)
        .upgrade(req, res, move |ws| handle_mux(ws, host))
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

async fn handle_mux(ws: WebSocket, host: Arc<HostState>) {
    use futures::SinkExt;
    let (mut sink, mut stream) = ws.split();
    let (tx, mut rx) = mpsc::channel::<RpcMessage>(MUX_CHAN_CAP);
    host.register_unbound_mux(tx).await;

    let send_loop = async {
        while let Some(msg) = rx.recv().await {
            let Ok(text) = serde_json::to_string(&msg) else {
                continue;
            };
            if sink.send(Message::text(text)).await.is_err() {
                break;
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
            if msg.is_text() || msg.is_binary() {
                // Business uplink is forbidden on mux: drop the connection.
                break;
            }
            // ping/pong handled by the crate.
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

    #[tokio::test]
    async fn unary_host_describe_ok() {
        let state = HostState::for_test().expect("host");
        let gateway = Gateway::starting();
        gateway.set_host(state);
        let service = Service::new(router(gateway));
        let body = serde_json::json!({
            "type": "client-request",
            "rpcId": "r1",
            "method": "host.describe",
            "payload": {}
        });
        let mut resp = TestClient::post("http://127.0.0.1:0/api/host.describe")
            .json(&body)
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::OK, "unary path");
        let text = resp.take_string().await.unwrap();
        assert!(
            text.contains("server-response") || text.contains("ok"),
            "{text}"
        );
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
            "type": "client-request",
            "rpcId": "r-window",
            "method": "host.describe",
            "payload": {}
        });
        let resp = TestClient::post("http://127.0.0.1:0/api/host.describe")
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
