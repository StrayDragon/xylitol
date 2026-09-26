//! Salvo HTTP + mux routes.
//!
//! - `GET /healthz`
//! - `GET /openapi.json` / `GET /docs`
//! - `POST /rpc` (product JSON-RPC 2.0 unary, jsonrpsee method table)
//! - `GET /rpc` (mux WebSocket; downlink JSON-RPC notifications)

use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

use futures::StreamExt;
use salvo::http::StatusCode;
use salvo::prelude::*;
use salvo::websocket::{Message, WebSocket, WebSocketUpgrade};
use tokio::sync::mpsc;

use crate::app::server::host::{HostState, MUX_CHAN_CAP};
use crate::app::server::rpc_module::{self, ProductRpc};
use crate::protocol::RpcMessage;
use crate::protocol::wire::codec;

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
    rpc: OnceLock<ProductRpc>,
}

impl Gateway {
    const STARTING: u8 = 0;
    const READY: u8 = 1;
    const FAILED: u8 = 2;

    pub fn starting() -> Arc<Self> {
        Arc::new(Self {
            phase: AtomicU8::new(Self::STARTING),
            host: OnceLock::new(),
            rpc: OnceLock::new(),
        })
    }

    /// Fill the host and flip to ready (idempotent fill: first writer wins).
    pub fn set_host(&self, host: Arc<HostState>) {
        let _ = self.rpc.set(rpc_module::build_rpc_module(host.clone()));
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

    pub(crate) fn rpc_module(&self) -> Option<&ProductRpc> {
        self.rpc.get()
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
        .push(Router::with_path("openapi.json").get(openapi_json))
        .push(
            salvo_oapi::scalar::Scalar::new("/openapi.json")
                .title("xylitol unary debug API")
                .into_router("docs"),
        )
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

/// sr-oapi1: static OpenAPI 3.1 debug document (built from the method table).
#[handler]
async fn openapi_json(res: &mut Response) {
    res.status_code(StatusCode::OK);
    res.render(Text::Plain(super::oapi::openapi_doc()));
}

#[handler]
async fn rpc(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    let Some(gateway) = gateway_from(depot) else {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        return;
    };
    let Some(module) = gateway.rpc_module().cloned() else {
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
    let v: serde_json::Value = match serde_json::from_slice(bytes) {
        Ok(v) => v,
        Err(_) => {
            illegal_envelope(res);
            return;
        }
    };
    if v.get("jsonrpc").and_then(serde_json::Value::as_str) != Some("2.0") {
        illegal_envelope(res);
        return;
    }
    let rpc_id = match v.get("id") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        _ => String::new(),
    };
    let Ok(text) = std::str::from_utf8(bytes) else {
        illegal_envelope(res);
        return;
    };
    match rpc_module::dispatch_raw(&module, text, rpc_id, writer).await {
        Ok((raw, token)) => polish_rpc_http(res, &raw, token),
        Err(_) => illegal_envelope(res),
    }
}

/// Fill `-32601` product `data.code`. Defensively strip a leaked `result.writerToken`.
fn polish_rpc_json(raw: &str) -> (serde_json::Value, Option<String>) {
    let mut v: serde_json::Value =
        serde_json::from_str(raw).unwrap_or_else(|_| serde_json::json!({}));
    let method_not_found = v
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(serde_json::Value::as_i64)
        == Some(-32601);
    if method_not_found {
        let missing_product_code = v
            .pointer("/error/data/code")
            .and_then(serde_json::Value::as_str)
            .is_none();
        if missing_product_code
            && let Some(err) = v
                .get_mut("error")
                .and_then(serde_json::Value::as_object_mut)
        {
            err.insert(
                "data".into(),
                serde_json::json!({ "code": "unregistered_method" }),
            );
        }
    }
    let token = v
        .get("result")
        .and_then(|r| r.get("writerToken"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    if token.is_some()
        && let Some(obj) = v
            .get_mut("result")
            .and_then(serde_json::Value::as_object_mut)
    {
        obj.remove("writerToken");
    }
    (v, token)
}

/// Stamp `X-Writer-Token` from the lease side-channel and fill `-32601` product
/// `data.code`. jsonrpsee owns method dispatch; HTTP leftovers stay here.
fn polish_rpc_http(res: &mut Response, raw: &str, token: Option<String>) {
    let (v, leaked) = polish_rpc_json(raw);
    if let Some(tok) = token.or(leaked) {
        let _ = res.add_header("X-Writer-Token", tok, true);
    }
    res.status_code(StatusCode::OK);
    res.render(Json(v));
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
    let module = gateway.rpc_module().cloned();
    let writer = req
        .headers()
        .get("X-Writer-Token")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    WebSocketUpgrade::new()
        .check_origin(mux_origin_allowed)
        .upgrade(req, res, move |ws| handle_mux(ws, host, module, writer))
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

async fn handle_mux(
    ws: WebSocket,
    host: Arc<HostState>,
    module: Option<ProductRpc>,
    mut writer: Option<String>,
) {
    use futures::SinkExt;
    let (mut sink, mut stream) = ws.split();
    // Handshake is host.describe result, not a mux ServerHello frame (c2825).
    let (tx, mut rx) = mpsc::channel::<RpcMessage>(MUX_CHAN_CAP);
    let (reply_tx, mut reply_rx) = mpsc::channel::<String>(MUX_CHAN_CAP);
    host.register_unbound_mux(tx).await;

    let send_loop = async {
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    let Some(msg) = msg else { break };
                    let text = match &msg {
                        RpcMessage::ServerRequest {
                            method, payload, ..
                        } => codec::jsonrpc_notification(method, payload.clone()).to_string(),
                        _ => continue,
                    };
                    if sink.send(Message::text(text)).await.is_err() {
                        break;
                    }
                }
                text = reply_rx.recv() => {
                    let Some(text) = text else { break };
                    if sink.send(Message::text(text)).await.is_err() {
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
            if !msg.is_text() {
                if msg.is_binary() {
                    break;
                }
                continue;
            }
            let Ok(text) = msg.as_str() else {
                break;
            };
            let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else {
                break;
            };
            if v.get("jsonrpc").and_then(serde_json::Value::as_str) != Some("2.0")
                || v.get("method")
                    .and_then(serde_json::Value::as_str)
                    .is_none()
            {
                break;
            }
            let Some(module) = module.as_ref() else {
                break;
            };
            let rpc_id = match v.get("id") {
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(serde_json::Value::Number(n)) => n.to_string(),
                _ => String::new(),
            };
            let has_id = v.get("id").is_some() && !v.get("id").is_some_and(|id| id.is_null());
            match rpc_module::dispatch_raw(module, text, rpc_id, writer.clone()).await {
                Ok((raw, token)) if has_id => {
                    let (mut body, leaked) = polish_rpc_json(&raw);
                    if let Some(tok) = token.or(leaked) {
                        writer = Some(tok.clone());
                        if let Some(obj) = body.as_object_mut() {
                            obj.insert("writerToken".into(), serde_json::json!(tok));
                        }
                    }
                    if reply_tx.send(body.to_string()).await.is_err() {
                        break;
                    }
                }
                Ok(_) => {}
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

    #[tokio::test]
    async fn unary_host_describe_ok() {
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
        let mut resp = TestClient::post("http://127.0.0.1:0/rpc")
            .json(&body)
            .send(&service)
            .await;
        assert_eq!(resp.status_code.unwrap(), StatusCode::OK, "unary path");
        let text = resp.take_string().await.unwrap();
        assert!(
            text.contains("jsonrpc") && text.contains("result"),
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
