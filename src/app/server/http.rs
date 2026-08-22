//! Salvo HTTP + mux routes (four-quadrant product carrier).
//!
//! - `GET /healthz`
//! - `GET /openapi.json` (unary debug document, built from the method table)
//! - `GET /docs` (Scalar debug UI — the only debug UI; points at `/openapi.json`)
//! - `POST /api/respond`
//! - `POST /api/{method}` (registered unary only)
//! - `GET /api/events.mux` (WebSocket downlink only)

use std::sync::Arc;

use futures::StreamExt;
use salvo::http::StatusCode;
use salvo::prelude::*;
use salvo::websocket::{Message, WebSocket, WebSocketUpgrade};
use tokio::sync::mpsc;

use crate::app::server::host::{HostState, MUX_CHAN_CAP, handle_unary};
use crate::protocol::RpcMessage;
use crate::protocol::wire::method::is_unary_method;

struct InjectHost(Arc<HostState>);

#[async_trait]
impl Handler for InjectHost {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        depot.insert_typed(self.0.clone());
        ctrl.call_next(req, depot, res).await;
    }
}

fn host_from(depot: &Depot) -> Option<Arc<HostState>> {
    depot.get_typed::<Arc<HostState>>().ok().cloned()
}

/// Product router (no `/api/v1` REST).
pub fn router(state: Arc<HostState>) -> Router {
    Router::new()
        .hoop(InjectHost(state))
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

#[handler]
async fn healthz(depot: &mut Depot, res: &mut Response) {
    let shutting = host_from(depot)
        .map(|h| h.shutting_down.load(std::sync::atomic::Ordering::Relaxed))
        .unwrap_or(false);
    if shutting {
        res.status_code(StatusCode::SERVICE_UNAVAILABLE);
        res.render(Json(serde_json::json!({"status": "shutting_down"})));
        return;
    }
    res.status_code(StatusCode::OK);
    res.render(Json(serde_json::json!({"status": "ok"})));
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

    let result = handle_unary(&host, &method, payload, writer_token).await;
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
        let service = Service::new(router(state));
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
        let service = Service::new(router(state));
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
        let service = Service::new(router(state));
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
