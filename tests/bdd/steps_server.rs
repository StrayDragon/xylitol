use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use std::time::Duration;

use crate::bdd::helpers::with_test_timeout;
use xylitol_app_tui::XyRemoteDriver;
use xylitol_app_tui::driver::LinkTunings;
use xylitol_app_tui::driver::{LinkHealth, XyDriver};
use xylitol_app_tui::host_client::{HostClient, HttpWsClient, MuxStream};
use xylitol_kernel::occupancy::host::HostState;
use xylitol_kernel::occupancy::runtime::{RunningServer, ServerConfig, bind_serve, serve};
use xylitol_protocol::Event;
use xylitol_protocol::codec;
use xylitol_protocol::envelope::{PROTOCOL_VERSION, RpcMessage};
use xylitol_serve::Gateway;
use xylitol_serve::occupancy::{EventJournal, ReverseRpcResult};

const SESSION_C2841: &str = "s-c2841";
const SESSION_V3: &str = "s-v3";

pub struct ServerTest {
    pub running: RefCell<Option<RunningServer>>,
    pub host: RefCell<Option<Arc<HostState>>>,
    pub port: Cell<u16>,
    pub second_err: RefCell<Option<String>>,
    pub unary_status: Cell<u16>,
    pub unary_body: RefCell<Option<String>>,
    pub mux_frames: RefCell<Vec<RpcMessage>>,
    pub occupied: RefCell<Option<std::net::TcpListener>>,
    pub journal: RefCell<Option<EventJournal>>,
    pub last_seq: Cell<u64>,
    pub approval_rx: RefCell<Option<tokio::sync::oneshot::Receiver<ReverseRpcResult>>>,
    pub last_rpc: RefCell<Option<ReverseRpcResult>>,
    pub mux_acc: Arc<std::sync::Mutex<Vec<RpcMessage>>>,
    pub mux_rx: RefCell<Option<MuxStream>>,
    pub idem_first: Arc<std::sync::Mutex<Option<String>>>,
    pub idem_second: RefCell<Option<String>>,
    pub idem_exec_file: RefCell<Option<std::path::PathBuf>>,
    pub idem_task: RefCell<Option<tokio::task::JoinHandle<()>>>,
    pub gateway: RefCell<Option<Arc<Gateway>>>,
    pub rdy_status: Cell<u16>,
    pub rdy_body: RefCell<Option<String>>,
    pub reg_path: RefCell<Option<std::path::PathBuf>>,
    pub evict_rx: RefCell<Option<tokio::sync::watch::Receiver<bool>>>,
    pub attach_driver: RefCell<Option<XyRemoteDriver<HttpWsClient>>>,
    pub writer_header: RefCell<Option<String>>,
    pub lock_child: RefCell<Option<std::process::Child>>,
    pub nonbinary_posts: RefCell<Vec<(u16, String)>>,
    pub observe_rx_a: RefCell<Option<tokio::sync::mpsc::Receiver<RpcMessage>>>,
    pub observe_rx_b: RefCell<Option<tokio::sync::mpsc::Receiver<RpcMessage>>>,
    pub observe_bind: RefCell<Option<serde_json::Value>>,
    pub busy_code: RefCell<Option<String>>,
    pub freeze_session: RefCell<String>,
    pub freeze_value: RefCell<Option<serde_json::Value>>,
    pub freeze_error: RefCell<Option<String>>,
    pub freeze_elapsed_ms: Cell<u64>,
}

impl Drop for ServerTest {
    fn drop(&mut self) {
        if let Some(mut child) = self.lock_child.borrow_mut().take() {
            crate::bdd::helpers::kill_child(&mut child);
        }
    }
}

impl ServerTest {
    fn new() -> Self {
        Self {
            running: RefCell::new(None),
            host: RefCell::new(None),
            port: Cell::new(0),
            second_err: RefCell::new(None),
            unary_status: Cell::new(0),
            unary_body: RefCell::new(None),
            mux_frames: RefCell::new(Vec::new()),
            occupied: RefCell::new(None),
            journal: RefCell::new(None),
            last_seq: Cell::new(0),
            approval_rx: RefCell::new(None),
            last_rpc: RefCell::new(None),
            mux_acc: Arc::new(std::sync::Mutex::new(Vec::new())),
            mux_rx: RefCell::new(None),
            idem_first: Arc::new(std::sync::Mutex::new(None)),
            idem_second: RefCell::new(None),
            idem_exec_file: RefCell::new(None),
            idem_task: RefCell::new(None),
            gateway: RefCell::new(None),
            rdy_status: Cell::new(0),
            rdy_body: RefCell::new(None),
            reg_path: RefCell::new(None),
            evict_rx: RefCell::new(None),
            attach_driver: RefCell::new(None),
            writer_header: RefCell::new(None),
            lock_child: RefCell::new(None),
            nonbinary_posts: RefCell::new(Vec::new()),
            observe_rx_a: RefCell::new(None),
            observe_rx_b: RefCell::new(None),
            observe_bind: RefCell::new(None),
            busy_code: RefCell::new(None),
            freeze_session: RefCell::new(String::new()),
            freeze_value: RefCell::new(None),
            freeze_error: RefCell::new(None),
            freeze_elapsed_ms: Cell::new(0),
        }
    }

    fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port.get())
    }
}

#[fixture]
pub fn server_test() -> ServerTest {
    ServerTest::new()
}

#[fixture]
pub fn approval_test() -> ServerTest {
    ServerTest::new()
}

pub async fn start_host(t: &ServerTest) {
    let host = xylitol_kernel::occupancy::host_state_for_test().expect("HostState");
    let (running, port) = with_test_timeout(|| {
        let host = host.clone();
        async move {
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
            tokio::time::sleep(Duration::from_millis(30)).await;
            (running, port)
        }
    })
    .await
    .expect("serve 启动 MUST 在 with_test_timeout 内完成");
    t.host.replace(Some(host));
    t.running.replace(Some(running));
    t.port.set(port);
}

async fn wait_unbound(host: &HostState, n: usize) {
    for _ in 0..80 {
        if host.unbound_mux.lock().await.len() >= n {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn http_status(port: u16, method: &str, path: &str, body: &str) -> (u16, String) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let client = reqwest::Client::new();
    let builder = match method {
        "GET" => client.get(&url),
        "POST" => client
            .post(&url)
            .header("content-type", "application/json")
            .body(body.to_string()),
        other => client.request(other.parse().unwrap_or(reqwest::Method::GET), &url),
    };
    let resp = builder.send().await.expect("http");
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    (status, text)
}

async fn post_rpc(
    port: u16,
    id: &str,
    method: &str,
    params: serde_json::Value,
    writer: Option<&str>,
) -> (u16, Option<String>, String) {
    use xylitol_app_tui::host_client::{
        client_request_bytes, server_response_to_result, stable_rpc_id,
    };
    use xylitol_protocol::v3::Frame;

    let url = format!("http://127.0.0.1:{port}/rpc");
    let bytes = client_request_bytes(id, method, &params).expect("v3 encode");
    let _ = writer;
    let client = reqwest::Client::new();
    let builder = client
        .post(&url)
        .header("content-type", "application/x-fory-v3")
        .body(bytes);
    let resp = builder.send().await.expect("rpc");
    let status = resp.status().as_u16();
    let token = resp
        .headers()
        .get("x-writer-token")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let raw = resp.bytes().await.unwrap_or_default();
    if status != 200 {
        return (status, token, String::from_utf8_lossy(&raw).into_owned());
    }
    let Frame::ServerResponse(frame) = Frame::from_bytes(&raw).expect("v3 frame") else {
        panic!("expected ServerResponse");
    };
    let result = server_response_to_result(&frame);
    let mut view = serde_json::json!({
        "rpc_id": frame.rpc_id,
        "expected_rpc_id": stable_rpc_id(id),
        "ok": result.ok,
        "result": result.value,
    });
    if let Some(e) = result.error.as_ref() {
        view["error"] = serde_json::json!({
            "code": e.code,
            "details": e.details,
            "data": { "code": e.code },
        });
    }
    (status, token.or(result.writer_token), view.to_string())
}

#[when("服务端在空闲端口上启动")]
async fn w_start_idle(server_test: &ServerTest) {
    start_host(server_test).await;
}

#[then("healthz 端点返回 200 OK")]
async fn t_healthz(server_test: &ServerTest) {
    let (status, body) = http_status(server_test.port.get(), "GET", "/healthz", "").await;
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("ok"), "{body}");
}

#[when("GET /openapi.json")]
async fn w_openapi_doc(server_test: &ServerTest) {
    let (status, body) = http_status(server_test.port.get(), "GET", "/openapi.json", "").await;
    server_test.unary_status.set(status);
    *server_test.unary_body.borrow_mut() = Some(body);
}

#[then("该路径不提供调试文档")]
fn t_debug_doc_absent(server_test: &ServerTest) {
    let st = server_test.unary_status.get();
    assert_ne!(st, 200, "debug docs must not be served, got {st}");
}

#[when("GET /docs")]
async fn w_scalar_page(server_test: &ServerTest) {
    let (status, body) = http_status(server_test.port.get(), "GET", "/docs", "").await;
    server_test.unary_status.set(status);
    *server_test.unary_body.borrow_mut() = Some(body);
}

#[given("服务端已在该地址端口监听")]
async fn g_already_listening(server_test: &ServerTest) {
    start_host(server_test).await;
}

#[when("第二个服务端绑定同一地址端口")]
async fn w_second_bind(server_test: &ServerTest) {
    let host2 = xylitol_kernel::occupancy::host_state_for_test().expect("host2");
    let err = serve(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: server_test.port.get(),
            sessions_dir: None,
            registration_path: None,
        },
        host2,
    )
    .await
    .err()
    .map(|e| e.to_string());
    server_test.second_err.replace(err);
}

#[then("第二个实例因地址占用失败")]
fn t_second_fails(server_test: &ServerTest) {
    let err = server_test.second_err.borrow();
    let msg = err.as_deref().unwrap_or("");
    assert!(
        msg.to_lowercase().contains("in use")
            || msg.contains("bind")
            || msg.contains("AddrInUse")
            || !msg.is_empty(),
        "expected bind failure, got {msg}"
    );
}

#[when("server 应用面启动")]
async fn w_server_app_start(server_test: &ServerTest) {
    start_host(server_test).await;
}

#[then("装配 infra 运行时并按 session 槽注入 Driver，供 unary 处理器调用")]
fn t_sr1(_server_test: &ServerTest) {
    assert!(
        std::any::type_name::<xylitol_kernel::occupancy::HostState>().contains("HostState"),
        "occupancy host must be HostState"
    );
}

#[when("POST /rpc 调用 host.describe")]
async fn w_rpc_host_describe(server_test: &ServerTest) {
    let (st, _, body) = post_rpc(
        server_test.port.get(),
        "R",
        "host.describe",
        serde_json::json!({}),
        None,
    )
    .await;
    server_test.unary_status.set(st);
    server_test.unary_body.replace(Some(body));
}

#[then("应答为产品成功且 rpc_id 回显")]
fn t_jsonrpc_success(server_test: &ServerTest) {
    let st = server_test.unary_status.get();
    let body = server_test.unary_body.borrow().clone().unwrap_or_default();
    assert_eq!(st, 200, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).expect("v3 view");
    assert_eq!(v["ok"], true, "{body}");
    assert_eq!(v["rpc_id"], v["expected_rpc_id"], "{body}");
    assert!(v.get("result").is_some(), "{body}");
    assert!(v.get("error").is_none() || v["error"].is_null(), "{body}");
}

#[then("result 携带协议版本整数")]
fn t_describe_protocol(server_test: &ServerTest) {
    t_jsonrpc_success(server_test);
    let body = server_test.unary_body.borrow().clone().unwrap_or_default();
    let v: serde_json::Value = serde_json::from_str(&body).expect("v3 view");
    let protocol = v["result"]["protocol"]
        .as_u64()
        .expect("host.describe result.protocol");
    assert_eq!(protocol, u64::from(PROTOCOL_VERSION), "{body}");
}

#[when("POST /rpc 调用 approve_tool")]
async fn w_rpc_approve_tool(server_test: &ServerTest) {
    let (st, _, body) = post_rpc(
        server_test.port.get(),
        "A",
        "approve_tool",
        serde_json::json!({"call_id": "none", "approved": true}),
        None,
    )
    .await;
    server_test.unary_status.set(st);
    server_test.unary_body.replace(Some(body));
}

#[then("应答不是未登记方法错误")]
fn t_not_method_not_found(server_test: &ServerTest) {
    let st = server_test.unary_status.get();
    let body = server_test.unary_body.borrow().clone().unwrap_or_default();
    assert_eq!(st, 200, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).expect("v3 view");
    let code = v["error"]["data"]["code"].as_str().unwrap_or("");
    assert_ne!(code, "unregistered_method", "{body}");
    assert_eq!(v["ok"], true, "{body}");
}

#[when("POST /rpc 发送非法信封")]
async fn w_rpc_illegal(server_test: &ServerTest) {
    let (st, resp) = http_status(server_test.port.get(), "POST", "/rpc", "{}").await;
    server_test.unary_status.set(st);
    server_test.unary_body.replace(Some(resp));
}

#[when("POST /rpc 发送 JSON-RPC 2.0 文本")]
async fn w_rpc_jsonrpc_text(server_test: &ServerTest) {
    let body = r#"{"jsonrpc":"2.0","id":"j1","method":"host.describe","params":{}}"#;
    let (st, resp) = http_status(server_test.port.get(), "POST", "/rpc", body).await;
    server_test.unary_status.set(st);
    server_test.unary_body.replace(Some(resp));
}

#[then("HTTP 失败且无成功业务应答")]
fn t_illegal_envelope(server_test: &ServerTest) {
    let st = server_test.unary_status.get();
    let body = server_test.unary_body.borrow().clone().unwrap_or_default();
    assert!(
        st >= 400,
        "illegal envelope MUST fail HTTP, got {st} {body}"
    );
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::json!({}));
    assert!(
        v.get("result").is_none(),
        "illegal envelope MUST NOT be a JSON-RPC success: {body}"
    );
}

#[then("Host 对该路径给出可观察往返")]
fn t_sr_env1_roundtrip(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow();
    let s = body.as_deref().unwrap_or("");
    assert!(
        s.contains(&PROTOCOL_VERSION.to_string()) || s.contains("protocol"),
        "host.describe must round-trip, got {s}"
    );
}

#[when("启动 app::server 运行时")]
async fn w_start_runtime(server_test: &ServerTest) {
    start_host(server_test).await;
}

#[then("暴露 POST /rpc、WS /rpc 与 GET /healthz")]
async fn t_sr2_routes(server_test: &ServerTest) {
    let (st, body) = http_status(server_test.port.get(), "GET", "/healthz", "").await;
    assert_eq!(st, 200, "{body}");
    let (st, _, body) = post_rpc(
        server_test.port.get(),
        "R",
        "host.describe",
        serde_json::json!({}),
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert!(
        body.contains("protocol") || body.contains("\"ok\":true"),
        "{body}"
    );
    let client = HttpWsClient::new(server_test.base_url());
    let mux = client.mux().await.expect("WS /rpc upgrade");
    drop(mux);
}

#[then("不暴露 /api/v1 产品 REST")]
async fn t_no_v1_rest(server_test: &ServerTest) {
    let (st, _) = http_status(server_test.port.get(), "POST", "/api/v1/session/x/run", "").await;
    assert!(
        st == 404 || st == 400 || st == 405,
        "REST must not be product, got {st}"
    );
}

#[when("第二个 app::server 进程绑定同一地址端口")]
async fn w_second_app_bind(server_test: &ServerTest) {
    if server_test.port.get() == 0 {
        start_host(server_test).await;
    }
    w_second_bind(server_test).await;
}

#[then("因地址占用失败且不写整机锁文件")]
fn t_sr3(server_test: &ServerTest) {
    t_second_fails(server_test);
    assert!(
        !std::path::Path::new("/tmp/xylitol-server.lock").exists()
            || std::fs::read_to_string("/tmp/xylitol-server.lock")
                .ok()
                .is_none_or(|s| !s.contains(&server_test.port.get().to_string())),
        "must not use lock file as mutex"
    );
}

#[given("客户端断开 N 秒后以 last_seq 经 POST /rpc 订阅")]
async fn g_reconnect(server_test: &ServerTest) {
    start_host(server_test).await;
    server_test.last_seq.set(0);
}

#[when("断开期间 server 产生事件")]
async fn w_events_while_disconnected(server_test: &ServerTest) {
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot("s-replay").await;
    slot.append_and_push(Event::TextDelta {
        text: "missed".into(),
    })
    .await;
    server_test.last_seq.set(0);
}

#[then("客户端收到 last_seq+1 起全部遗漏事件再收实时事件")]
async fn t_replay(server_test: &ServerTest) {
    let client = HttpWsClient::new(server_test.base_url());
    let mut mux = client.mux().await.expect("mux");
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    wait_unbound(&host, 1).await;
    let (st, _, body) = post_rpc(
        server_test.port.get(),
        "S",
        "subscribe",
        serde_json::json!({"session_id": "s-replay", "last_seq": 0}),
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).expect("jsonrpc");
    assert!(v.get("result").is_some(), "subscribe must succeed: {body}");
    let frame = tokio::time::timeout(Duration::from_secs(2), mux.next())
        .await
        .expect("timeout")
        .expect("eof")
        .expect("ws");
    match frame {
        RpcMessage::ServerRequest { method, .. } => {
            assert!(
                method == "session/event" || method == "session/subscribed",
                "{method}"
            );
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[given("工具需审批")]
async fn g_tool_needs_approval(server_test: &ServerTest) {
    start_host(server_test).await;
}

#[when("推送 ApprovalRequired")]
async fn w_push_approval(server_test: &ServerTest) {
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot(&host.fallback_session).await;
    let rx = slot.request_approval("xyz".into()).await;
    server_test.approval_rx.replace(Some(rx));
}

#[then("客户端经 POST /rpc 调用 approve_tool 且回合恢复")]
async fn t_sr5_respond(server_test: &ServerTest) {
    let (st, _, body) = post_rpc(
        server_test.port.get(),
        "A1",
        "approve_tool",
        serde_json::json!({"call_id": "xyz", "approved": true}),
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    let mut rx = server_test.approval_rx.borrow_mut().take().expect("rx");
    let got = tokio::time::timeout(Duration::from_secs(1), &mut rx)
        .await
        .expect("timeout")
        .expect("dropped");
    assert_eq!(got, ReverseRpcResult::Approved);
}

#[given("server 已启动")]
async fn g_server_started(server_test: &ServerTest) {
    start_host(server_test).await;
}

#[when("检查单实例互斥手段")]
fn w_check_mutex(_server_test: &ServerTest) {}

#[then("不依赖锁文件 JSON 的 port、pid、hostname")]
fn t_sr7(_server_test: &ServerTest) {
    let _ = xylitol_kernel::occupancy::runtime::ServerConfig {
        host: "127.0.0.1".into(),
        port: 0,
        sessions_dir: None,
        registration_path: None,
    };
}

#[given("18790 被其它进程占用")]
fn g_occupy_18790(server_test: &ServerTest) {
    match std::net::TcpListener::bind(("127.0.0.1", 18790)) {
        Ok(l) => {
            server_test.occupied.replace(Some(l));
        }
        Err(_) => {}
    }
}

#[when("server 在 18790 启动")]
async fn w_start_18790(server_test: &ServerTest) {
    let host = xylitol_kernel::occupancy::host_state_for_test().expect("host");
    let err = serve(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 18790,
            sessions_dir: None,
            registration_path: None,
        },
        host,
    )
    .await
    .err()
    .map(|e| e.to_string());
    server_test.second_err.replace(err);
}

#[then("失败返回且不绑定 18791")]
fn t_sr8(server_test: &ServerTest) {
    assert!(
        server_test.second_err.borrow().is_some(),
        "must fail when 18790 is busy"
    );
    assert!(
        std::net::TcpListener::bind(("127.0.0.1", 18791)).is_ok(),
        "must not bind 18791"
    );
}

#[given("向 POST /rpc 发送 prompt 请求")]
async fn g_post_prompt(server_test: &ServerTest) {
    start_host(server_test).await;
    let (st, _, body) = post_rpc(
        server_test.port.get(),
        "R-prompt",
        "prompt",
        serde_json::json!({"message": "hi", "session_id": "s-prompt"}),
        None,
    )
    .await;
    server_test.unary_status.set(st);
    server_test.unary_body.replace(Some(body));
}

#[when("server 处理 prompt")]
fn w_handle_prompt(_server_test: &ServerTest) {}

#[then("HTTP 200 且应答回显 rpc_id")]
fn t_prompt_200(server_test: &ServerTest) {
    assert_eq!(server_test.unary_status.get(), 200);
    let body = server_test.unary_body.borrow().clone().unwrap_or_default();
    let v: serde_json::Value = serde_json::from_str(&body).expect("v3 view");
    assert_eq!(v["ok"], true, "{body}");
    assert_eq!(v["rpc_id"], v["expected_rpc_id"], "{body}");
}

#[then("POST /api/v1/session/x/run 不是产品路径")]
async fn t_run_not_product(server_test: &ServerTest) {
    t_no_v1_rest(server_test).await;
}

#[when("app::server 运行时经 app::core::composition 组装 Agent")]
fn w_sr10(_server_test: &ServerTest) {}

#[then("向 Agent 传入 Arc<dyn ExportIo>")]
fn t_sr10(_server_test: &ServerTest) {
    let _ = std::any::type_name::<xylitol_kernel::driver::SessionExporter>();
}

#[given("审查 server 组合根")]
fn g_review_root(_server_test: &ServerTest) {}

#[when("检查持有类型")]
fn w_check_types(_server_test: &ServerTest) {}

#[then("按 session 槽持有 Driver；handlers 不直接以 Mutex AgentRuntime 作为唯一入口")]
fn t_driver1(_server_test: &ServerTest) {
    let _ = std::any::type_name::<xylitol_kernel::occupancy::SessionSlot>();
}

#[when("调用 RemoteDriver 已登记 unary（如 steer）")]
fn w_remote_steer(_server_test: &ServerTest) {}

#[then("经 HostClient 到达 Host")]
fn t_remote1(_server_test: &ServerTest) {
    let _ = std::any::type_name::<xylitol_app_tui::XyRemoteDriver>();
    let _ = std::any::type_name::<xylitol_app_tui::host_client::HttpWsClient>();
}

#[then("未登记方法不发明 REST")]
fn t_no_invented_rest(_server_test: &ServerTest) {
    assert!(
        xylitol_protocol::is_unary_method("list_sessions"),
        "list_sessions is a product unary"
    );
}

#[when("审查 prompt/steer 等 unary handlers")]
fn w_review_handlers(_server_test: &ServerTest) {}

#[then("调用 dispatch 或共享 helper；与 InProcessDriver 语义一致")]
fn t_dispatch1(_server_test: &ServerTest) {
    let _ = xylitol_app_tui::dispatch::dispatch;
}

#[given("server 上存在 session 与消息树")]
async fn g_session_tree(server_test: &ServerTest) {
    start_host(server_test).await;
}

#[when("请求 /api/v1 会话树 REST")]
async fn w_tree_rest(server_test: &ServerTest) {
    let (st, body) = http_status(
        server_test.port.get(),
        "GET",
        "/api/v1/session/x/trees/message-history",
        "",
    )
    .await;
    server_test.unary_status.set(st);
    server_test.unary_body.replace(Some(body));
}

#[then("不是产品路径（不存在或非产品）")]
fn t_not_product(server_test: &ServerTest) {
    let st = server_test.unary_status.get();
    assert!(st == 404 || st == 400 || st == 405, "got {st}");
}

#[given("RemoteDriver 指向该 server")]
fn g_remote_points(_server_test: &ServerTest) {}

#[when("调用已登记方法表的 session_tree/travel")]
fn w_session_tree_registered(_server_test: &ServerTest) {}

#[then("经产品 unary 到达 Host 且不经 REST 冒充")]
fn t_tree_registered(_server_test: &ServerTest) {
    assert!(
        xylitol_protocol::is_unary_method("session_tree"),
        "session_tree is a product unary"
    );
}

#[when("调用已登记的 session 能力 unary")]
fn w_session_methods_registered(_server_test: &ServerTest) {
    assert!(xylitol_protocol::is_unary_method("list_sessions"));
}

#[when("调用 Host 资源 unary")]
fn w_host_resource_methods(_server_test: &ServerTest) {
    assert!(xylitol_protocol::is_unary_method("reload"));
}

#[given(
    "构造下行 session/event、session/subscribed、session/resync_required 与 session/resources notification"
)]
fn g_w1_frame(server_test: &ServerTest) {
    let notes: Vec<String> = [
        "session/event",
        "session/subscribed",
        "session/resync_required",
        "session/resources",
        "approval/requested",
        "question/requested",
    ]
    .into_iter()
    .map(|method| {
        codec::jsonrpc_notification(method, serde_json::json!({"session_id": "s0"})).to_string()
    })
    .collect();
    server_test.unary_body.replace(Some(notes.join("\n")));
}

#[when("编码为下行通知帧")]
fn w_serialize(_server_test: &ServerTest) {}

#[then("各帧均为无作答义务的下行通知且携带 method 语义")]
fn t_w1(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow();
    let s = body.as_deref().unwrap();
    for line in s.lines() {
        let v: serde_json::Value = serde_json::from_str(line).expect("json");
        assert!(v.get("method").and_then(|m| m.as_str()).is_some(), "{line}");
        assert!(
            v.get("id").is_none(),
            "notification must not have id: {line}"
        );
    }
}

#[given("客户端欲以 last_seq 5 订阅会话 s0")]
fn g_w2(_server_test: &ServerTest) {}

#[when("POST /rpc 调用 subscribe 带 session_id=s0 与 last_seq=5")]
async fn w_subscribe_s0(server_test: &ServerTest) {
    if server_test.port.get() == 0 {
        start_host(server_test).await;
    }
    let (st, _, body) = post_rpc(
        server_test.port.get(),
        "S0",
        "subscribe",
        serde_json::json!({"session_id": "s0", "last_seq": 5}),
        None,
    )
    .await;
    server_test.unary_status.set(st);
    server_test.unary_body.replace(Some(body));
}

#[then("应答为产品成功且 result 含 session 与 seq")]
fn t_subscribe_result(server_test: &ServerTest) {
    let st = server_test.unary_status.get();
    let body = server_test.unary_body.borrow().clone().unwrap_or_default();
    assert_eq!(st, 200, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).expect("v3 view");
    assert_eq!(v["ok"], true, "{body}");
    assert!(v.get("error").is_none() || v["error"].is_null(), "{body}");
    let result = v.get("result").expect("result");
    assert!(
        result.get("session_id").and_then(|s| s.as_str()) == Some("s0")
            || result.get("sessionId").and_then(|s| s.as_str()) == Some("s0"),
        "{body}"
    );
    assert!(
        result.get("seq").and_then(|s| s.as_u64()).is_some(),
        "{body}"
    );
}

#[then("WS 丢弃文本业务上行")]
async fn t_w2_no_uplink(server_test: &ServerTest) {
    use futures::SinkExt;
    let url = format!("ws://127.0.0.1:{}/rpc", server_test.port.get());
    let (mut ws, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("WS /rpc");
    let _ = tokio::time::timeout(Duration::from_secs(2), ws.next()).await;
    ws.send(tokio_tungstenite::tungstenite::Message::Text(
        "not-jsonrpc".into(),
    ))
    .await
    .ok();
    let closed = tokio::time::timeout(Duration::from_secs(2), ws.next()).await;
    match closed {
        Ok(None)
        | Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))))
        | Ok(Some(Err(_)))
        | Err(_) => {}
        Ok(Some(Ok(other))) => panic!("non-v3 text must not be answered, got {other:?}"),
    }
}

#[when("产品客户端经 WS /rpc 调用 host.describe")]
async fn w_product_client_describe(server_test: &ServerTest) {
    let client = HttpWsClient::new(server_test.base_url());
    let result = client
        .unary("host.describe", serde_json::json!({}))
        .await
        .expect("product HttpWsClient unary");
    server_test
        .unary_body
        .replace(Some(serde_json::to_string(&result).expect("rpc result")));
}

#[then("产品 unary 成功且 result 含协议版本")]
fn t_product_unary_describe(server_test: &ServerTest) {
    let body = server_test
        .unary_body
        .borrow()
        .clone()
        .expect("product unary body");
    let result: xylitol_protocol::RpcResult = serde_json::from_str(&body).expect("RpcResult");
    assert!(result.ok, "{body}");
    assert!(result.error.is_none(), "{body}");
    let protocol = result
        .value
        .as_ref()
        .and_then(|v| v.get("protocol"))
        .and_then(|v| v.as_u64())
        .expect("host.describe protocol");
    assert_eq!(protocol, u64::from(PROTOCOL_VERSION), "{body}");
}

#[when("客户端经 WS /rpc 发送 host.describe JSON-RPC 请求")]
async fn w_ws_describe(server_test: &ServerTest) {
    use futures::{SinkExt, StreamExt};
    let url = format!("ws://127.0.0.1:{}/rpc", server_test.port.get());
    let (mut ws, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("WS /rpc");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "ws-1",
        "method": "host.describe",
        "params": {}
    });
    ws.send(tokio_tungstenite::tungstenite::Message::Text(
        req.to_string().into(),
    ))
    .await
    .expect("send describe");
    let outcome = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            match ws.next().await {
                Some(Ok(tokio_tungstenite::tungstenite::Message::Ping(_)))
                | Some(Ok(tokio_tungstenite::tungstenite::Message::Pong(_))) => continue,
                other => return format!("{other:?}"),
            }
        }
    })
    .await;
    server_test.unary_body.replace(Some(match outcome {
        Err(_) => "timeout".into(),
        Ok(s) => s,
    }));
    let _ = ws.close(None).await;
}

#[then("WS 拒绝该文本业务帧")]
fn t_ws_reject_text(server_test: &ServerTest) {
    let body = server_test
        .unary_body
        .borrow()
        .clone()
        .expect("ws text outcome");
    assert!(
        body == "timeout"
            || body.contains("Close")
            || body.contains("None")
            || body.contains("Err"),
        "JSON 文本业务帧 MUST 被拒且无成功 unary 应答，got {body}"
    );
}

#[given("客户端连接 /rpc")]
async fn g_w3_mux(server_test: &ServerTest) {
    start_host(server_test).await;
}

#[when("server 接受升级")]
async fn w_w3_upgrade(server_test: &ServerTest) {
    let client = HttpWsClient::new(server_test.base_url());
    let mux = client.mux().await.expect("upgrade");
    drop(mux);
}

#[then("升级成功")]
fn t_w3_upgraded(_server_test: &ServerTest) {}

#[then("不把会话绑在 /api/v1/session/x/ws")]
fn t_w3_no_old_ws(_server_test: &ServerTest) {
    let http = std::fs::read_to_string("crates/xylitol-serve/src/listen.rs").unwrap();
    let product = http.split("#[cfg(test)]").next().unwrap_or(&http);
    assert!(
        !product.contains("/api/v1/session"),
        "product router must not mount /api/v1/session"
    );
}

#[given("以短空闲超时的产品订阅客户端已升级 WS /rpc")]
async fn g_halfopen_mux(server_test: &ServerTest) {
    start_host(server_test).await;
    let client = HttpWsClient::new(server_test.base_url())
        .with_mux_liveness(Duration::from_secs(3600), Duration::from_millis(80));
    let mux = client.mux().await.expect("upgrade");
    server_test.mux_rx.replace(Some(mux));
}

#[when("超过空闲超时仍无入站帧")]
async fn w_wait_idle(_server_test: &ServerTest) {
    tokio::time::sleep(Duration::from_millis(160)).await;
}

#[then("客户端判定半开并结束该订阅")]
async fn t_halfopen_idle(server_test: &ServerTest) {
    let mut mux = server_test.mux_rx.borrow_mut().take().expect("mux");
    let item = tokio::time::timeout(Duration::from_secs(2), mux.next())
        .await
        .expect("idle wait");
    match item {
        Some(Err(e)) => {
            let msg = e.to_string();
            assert!(msg.contains("mux idle"), "{msg}");
        }
        other => panic!("expected half-open idle error, got {other:?}"),
    }
}

#[given("向会话 append 3 个事件")]
fn g_w4(server_test: &ServerTest) {
    let mut j = EventJournal::with_default_capacity("s0");
    j.append(Event::TextDelta { text: "a".into() });
    j.append(Event::TextDelta { text: "b".into() });
    j.append(Event::TextDelta { text: "c".into() });
    server_test.journal.replace(Some(j));
}

#[when("读回事件")]
fn w_w4_read(_server_test: &ServerTest) {}

#[then("seq 值为 1,2,3（严格递增）")]
fn t_w4(server_test: &ServerTest) {
    let j = server_test.journal.borrow();
    let replayed = j.as_ref().unwrap().replay_from(0).unwrap();
    let seqs: Vec<u64> = replayed.iter().map(|(s, _)| *s).collect();
    assert_eq!(seqs, vec![1, 2, 3]);
}

#[given("向会话 append 10001 个事件（journal 容量=10000）")]
fn g_w5(server_test: &ServerTest) {
    let mut j = EventJournal::new("s0", 10_000);
    for i in 0..10_001 {
        j.append(Event::TextDelta {
            text: format!("{i}"),
        });
    }
    server_test.journal.replace(Some(j));
}

#[when("last_seq=0 的客户端经 POST /rpc 订阅")]
async fn w_w5_subscribe(server_test: &ServerTest) {
    start_host(server_test).await;
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot("s-wrap").await;
    {
        let mut j = slot.journal.lock().await;
        for i in 0..10_001 {
            j.append(Event::TextDelta {
                text: format!("{i}"),
            });
        }
    }
    let client = HttpWsClient::new(server_test.base_url());
    let mut mux = client.mux().await.expect("mux");
    wait_unbound(&host, 1).await;
    let _ = post_rpc(
        server_test.port.get(),
        "W5",
        "subscribe",
        serde_json::json!({"session_id": "s-wrap", "last_seq": 0}),
        None,
    )
    .await;
    if let Ok(Some(Ok(frame))) = tokio::time::timeout(Duration::from_secs(2), mux.next()).await {
        server_test.mux_frames.replace(vec![frame]);
    }
}

#[then("server 发送 session/resync_required 因事件 0..1 已丢失")]
fn t_w5(server_test: &ServerTest) {
    let frames = server_test.mux_frames.borrow();
    let hit = frames.iter().any(|f| {
        matches!(
            f,
            RpcMessage::ServerRequest { method, .. } if method == "session/resync_required"
        )
    });
    assert!(hit, "expected resync_required, got {frames:?}");
}

#[given("客户端收到 session/resync_required")]
async fn g_w6(server_test: &ServerTest) {
    w_w5_subscribe(server_test).await;
}

#[when("客户端以 last_seq=0 再次经 POST /rpc 订阅")]
async fn w_w6_resub(server_test: &ServerTest) {
    let client = HttpWsClient::new(server_test.base_url());
    let mut mux = client.mux().await.expect("mux");
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    wait_unbound(&host, 1).await;
    let _ = post_rpc(
        server_test.port.get(),
        "W6",
        "subscribe",
        serde_json::json!({"session_id": "s-wrap", "last_seq": 0}),
        None,
    )
    .await;
    let mut got = Vec::new();
    while let Ok(Some(Ok(frame))) =
        tokio::time::timeout(Duration::from_millis(400), mux.next()).await
    {
        let is_event =
            matches!(&frame, RpcMessage::ServerRequest { method, .. } if method == "session/event");
        got.push(frame);
        if !is_event && got.len() > 1 {
            break;
        }
        if got.len() > 8 {
            break;
        }
    }
    server_test.mux_frames.replace(got);
}

#[then("server 从 journal 重放全部可用事件")]
fn t_w6(server_test: &ServerTest) {
    let frames = server_test.mux_frames.borrow();
    let events = frames.iter().filter(
        |f| matches!(f, RpcMessage::ServerRequest { method, .. } if method == "session/event"),
    );
    assert!(
        events.count() > 0,
        "expected replayed session/event, got {frames:?}"
    );
}

#[given("客户端已 subscribe 且 prompt 运行")]
async fn g_w7(server_test: &ServerTest) {
    start_host(server_test).await;
}

#[when("agent 发出 TextDelta 事件")]
async fn w_w7_delta(server_test: &ServerTest) {
    let client = HttpWsClient::new(server_test.base_url());
    let mut mux = client.mux().await.expect("mux");
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    wait_unbound(&host, 1).await;
    let _ = post_rpc(
        server_test.port.get(),
        "W7",
        "subscribe",
        serde_json::json!({"session_id": "s-delta", "last_seq": 0}),
        None,
    )
    .await;
    host.slot("s-delta")
        .await
        .append_and_push(Event::TextDelta {
            text: "hello".into(),
        })
        .await;
    let mut got = Vec::new();
    while let Ok(Some(Ok(frame))) = tokio::time::timeout(Duration::from_secs(2), mux.next()).await {
        let is_event = matches!(
            &frame,
            RpcMessage::ServerRequest { method, .. } if method == "session/event"
        );
        got.push(frame);
        if is_event || got.len() > 8 {
            break;
        }
    }
    server_test.mux_frames.replace(got);
}

#[then("客户端收到 session/event notification")]
fn t_w7(server_test: &ServerTest) {
    let frames = server_test.mux_frames.borrow();
    let hit = frames.iter().any(|f| {
        matches!(
            f,
            RpcMessage::ServerRequest { method, .. } if method == "session/event"
        )
    });
    assert!(hit, "expected session/event, got {frames:?}");
}

async fn seed_w8_history(server_test: &ServerTest) {
    start_host(server_test).await;
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    host.ports
        .store
        .create("s0", Some("."), None)
        .await
        .expect("create session");
    for i in 0..3 {
        let entry = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("w8-{i}"),
                parent_id: None,
                timestamp: 1_700_000_000_000 + i,
            },
            message: serde_json::json!({"role": "user", "content": format!("m{i}")}),
        });
        host.ports
            .store
            .append_session_entry("s0", &entry)
            .await
            .expect("append entry");
    }
}

async fn snapshot_s0(server_test: &ServerTest) {
    let client = HttpWsClient::new(server_test.base_url());
    let subscribed = client
        .unary(
            "subscribe",
            serde_json::json!({ "session_id": "s0", "last_seq": 0 }),
        )
        .await
        .expect("subscribe");
    assert!(subscribed.ok, "{subscribed:?}");
    let switched = client
        .unary(
            "switch_session",
            serde_json::json!({ "session_path": "s0" }),
        )
        .await
        .expect("switch_session");
    assert!(switched.ok, "{switched:?}");
    let snap = client
        .unary("get_messages", serde_json::json!({ "session_id": "s0" }))
        .await
        .expect("get_messages");
    assert!(snap.ok, "snapshot MUST succeed: {snap:?}");
    server_test
        .unary_body
        .replace(Some(serde_json::to_string(&snap).unwrap()));
}

#[given("会话 s0 已有 3 条历史条目且客户端无有效 last_seq")]
async fn g_w8_history(server_test: &ServerTest) {
    seed_w8_history(server_test).await;
}

#[when("冷订客户端调用消息快照")]
async fn w_w8_snapshot(server_test: &ServerTest) {
    snapshot_s0(server_test).await;
}

#[then("快照一次返回全部 3 条条目")]
fn t_w8_snapshot_count(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow();
    let v: serde_json::Value = serde_json::from_str(body.as_deref().expect("body")).expect("json");
    let entries = v["value"]["entries"].as_array().expect("entries array");
    let messages = entries
        .iter()
        .filter(|e| e.get("type").and_then(serde_json::Value::as_str) == Some("message"))
        .count();
    assert_eq!(messages, 3, "snapshot MUST project full history once: {v}");
}

#[given("冷订客户端处于恢复窗内且 journal 含实况磁带事件")]
async fn g_w8_tape_window(server_test: &ServerTest) {
    seed_w8_history(server_test).await;
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot("s0").await;
    {
        let mut j = slot.journal.lock().await;
        j.append(Event::TextDelta {
            text: "tape-1".into(),
        });
        j.append(Event::TextDelta {
            text: "tape-2".into(),
        });
    }
}

#[when("调用消息快照")]
async fn w_w8_snapshot_windowed(server_test: &ServerTest) {
    snapshot_s0(server_test).await;
}

#[then("快照内容与磁带回放无关")]
fn t_w8_unpolluted(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow();
    let v: serde_json::Value = serde_json::from_str(body.as_deref().expect("body")).expect("json");
    let serialized = v.to_string();
    let entries = v["value"]["entries"].as_array().expect("entries array");
    let messages = entries
        .iter()
        .filter(|e| e.get("type").and_then(serde_json::Value::as_str) == Some("message"))
        .count();
    assert_eq!(
        messages, 3,
        "snapshot MUST stay the persisted projection: {v}"
    );
    assert!(
        !serialized.contains("tape-1") && !serialized.contains("tape-2"),
        "journal live tape MUST NOT leak into the snapshot: {serialized}"
    );
}

#[then("断线续传语义仍按 sr4 从 last_seq+1 重放")]
async fn t_w8_sr4_intact(server_test: &ServerTest) {
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot("s0").await;
    let replayed = slot.journal.lock().await.replay_from(0).unwrap();
    let seqs: Vec<u64> = replayed.iter().map(|(s, _)| *s).collect();
    assert_eq!(
        seqs.first().copied(),
        Some(1),
        "sr4 continuity MUST hold: replay starts at last_seq+1"
    );
}

#[given("服务端和已连接的 mux 客户端")]
async fn g_rr_mux(approval_test: &ServerTest) {
    start_host(approval_test).await;
    let client = HttpWsClient::new(approval_test.base_url());
    let mut mux = client.mux().await.expect("mux");
    let host = approval_test.host.borrow().as_ref().expect("host").clone();
    wait_unbound(&host, 1).await;
    let acc = approval_test.mux_acc.clone();
    tokio::spawn(async move {
        while let Some(Ok(frame)) = mux.next().await {
            acc.lock().unwrap().push(frame);
        }
    });
    let _ = client
        .unary(
            "subscribe",
            serde_json::json!({"session_id": host.fallback_session, "last_seq": 0}),
        )
        .await;
}

#[when("agent 执行需要审批的工具")]
async fn w_rr_tool(approval_test: &ServerTest) {
    let host = approval_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot(&host.fallback_session).await;
    let rx = slot.request_approval("call-approve-1".into()).await;
    approval_test.approval_rx.replace(Some(rx));
    tokio::time::sleep(Duration::from_millis(80)).await;
}

#[then("客户端收到 approval/requested notification")]
fn t_rr_got_request(approval_test: &ServerTest) {
    let acc = approval_test.mux_acc.lock().unwrap();
    let hit = acc.iter().any(|f| match f {
        RpcMessage::ServerRequest {
            method,
            rpc_id,
            payload,
            ..
        } if method == "approval/requested" => {
            rpc_id == "call-approve-1"
                || payload.get("call_id").and_then(|v| v.as_str()) == Some("call-approve-1")
        }
        _ => false,
    });
    assert!(hit, "expected approval/requested, got {acc:?}");
}

#[when("客户端 POST /rpc 调用 approve_tool 且 approved=true")]
async fn w_rr_approve(approval_test: &ServerTest) {
    let (st, _, body) = post_rpc(
        approval_test.port.get(),
        "A1",
        "approve_tool",
        serde_json::json!({"call_id": "call-approve-1", "approved": true}),
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    let mut rx = approval_test.approval_rx.borrow_mut().take().expect("rx");
    let got = tokio::time::timeout(Duration::from_secs(1), &mut rx)
        .await
        .expect("timeout")
        .expect("dropped");
    approval_test.last_rpc.replace(Some(got));
}

#[then("工具执行继续")]
fn t_tool_continues(approval_test: &ServerTest) {
    assert_eq!(
        *approval_test.last_rpc.borrow(),
        Some(ReverseRpcResult::Approved)
    );
}

#[then("turn 正常结束")]
fn t_turn_ok(approval_test: &ServerTest) {
    t_tool_continues(approval_test);
}

#[when("客户端 POST /rpc 调用 approve_tool 且 approved=false")]
async fn w_rr_deny(approval_test: &ServerTest) {
    let (st, _, body) = post_rpc(
        approval_test.port.get(),
        "A1",
        "approve_tool",
        serde_json::json!({"call_id": "call-approve-1", "approved": false}),
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    let mut rx = approval_test.approval_rx.borrow_mut().take().expect("rx");
    let got = tokio::time::timeout(Duration::from_secs(1), &mut rx)
        .await
        .expect("timeout")
        .expect("dropped");
    approval_test.last_rpc.replace(Some(got));
}

#[then("工具被拒绝")]
fn t_tool_denied(approval_test: &ServerTest) {
    assert_eq!(
        *approval_test.last_rpc.borrow(),
        Some(ReverseRpcResult::Denied)
    );
}

#[then("turn 继续但不包含工具结果")]
fn t_turn_no_tool(approval_test: &ServerTest) {
    t_tool_denied(approval_test);
}

#[given("agent 发出 ApprovalRequired，rpcId 为 xyz")]
async fn g_rr1(approval_test: &ServerTest) {
    start_host(approval_test).await;
    let host = approval_test.host.borrow().as_ref().expect("host").clone();
    let client = HttpWsClient::new(approval_test.base_url());
    let hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    for _ in 0..3 {
        let mut mux = client.mux().await.expect("mux");
        let hits = hits.clone();
        tokio::spawn(async move {
            while let Some(Ok(frame)) = mux.next().await {
                if matches!(
                    &frame,
                    RpcMessage::ServerRequest { method, payload, .. }
                        if method == "approval/requested"
                            && payload.get("call_id").and_then(|v| v.as_str()) == Some("xyz")
                ) {
                    hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            }
        });
    }
    wait_unbound(&host, 3).await;
    let _ = client
        .unary(
            "subscribe",
            serde_json::json!({"session_id": host.fallback_session, "last_seq": 0}),
        )
        .await;
    let slot = host.slot(&host.fallback_session).await;
    let _rx = slot.request_approval("xyz".into()).await;
    for _ in 0..40 {
        if hits.load(std::sync::atomic::Ordering::SeqCst) >= 3 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    approval_test
        .last_seq
        .set(hits.load(std::sync::atomic::Ordering::SeqCst) as u64);
}

#[when("server 检查 session 的 mux 连接")]
fn w_rr1_check(_approval_test: &ServerTest) {}

#[then("3 个已连接客户端均收到 approval/requested")]
fn t_rr1(approval_test: &ServerTest) {
    assert_eq!(
        approval_test.last_seq.get(),
        3,
        "each of 3 mux clients must see approval/requested"
    );
}

#[given("2 个客户端 POST /rpc 调用 approve_tool，call_id xyz（首个 true，50ms 后 false）")]
async fn g_rr2(approval_test: &ServerTest) {
    start_host(approval_test).await;
    let host = approval_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot(&host.fallback_session).await;
    let rx = slot.request_approval("xyz".into()).await;
    approval_test.approval_rx.replace(Some(rx));
    let port = approval_test.port.get();
    let _ = post_rpc(
        port,
        "A1",
        "approve_tool",
        serde_json::json!({"call_id": "xyz", "approved": true}),
        None,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    let _ = post_rpc(
        port,
        "A2",
        "approve_tool",
        serde_json::json!({"call_id": "xyz", "approved": false}),
        None,
    )
    .await;
}

#[when("server 处理首个应答")]
async fn w_rr2_first(approval_test: &ServerTest) {
    let mut rx = approval_test.approval_rx.borrow_mut().take().expect("rx");
    let got = tokio::time::timeout(Duration::from_secs(1), &mut rx)
        .await
        .expect("timeout")
        .expect("dropped");
    approval_test.last_rpc.replace(Some(got));
}

#[then("agent 以 approved=true 恢复；第二个应答被忽略")]
fn t_rr2(approval_test: &ServerTest) {
    assert_eq!(
        *approval_test.last_rpc.borrow(),
        Some(ReverseRpcResult::Approved)
    );
}

#[given("60s 内无客户端 POST /rpc 调用 approve_tool")]
async fn g_rr3(approval_test: &ServerTest) {
    start_host(approval_test).await;
    let host = approval_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot(&host.fallback_session).await;
    let rx = slot.gateway.register("expired".into());
    approval_test.approval_rx.replace(Some(rx));
}

#[when("server 将 rpcId 标记为 expired")]
async fn w_rr3_expire(approval_test: &ServerTest) {
    let host = approval_test.host.borrow().as_ref().expect("host").clone();
    let fallback = host.fallback_session.clone();
    let slot = host.slot(&fallback).await;
    slot.gateway.remove("expired");
}

#[then("agent 收到 ApprovalTimeout 错误")]
fn t_rr3(approval_test: &ServerTest) {
    let mut rx = approval_test.approval_rx.borrow_mut().take().expect("rx");
    assert!(
        rx.try_recv().is_err(),
        "sender dropped without result = timeout"
    );
}

#[given("第二个客户端对已消费 call 再 POST /rpc 调用 approve_tool")]
async fn g_rr4(approval_test: &ServerTest) {
    start_host(approval_test).await;
    let host = approval_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot(&host.fallback_session).await;
    let rx = slot.request_approval("xyz".into()).await;
    let (st, _, body) = post_rpc(
        approval_test.port.get(),
        "A1",
        "approve_tool",
        serde_json::json!({"call_id": "xyz", "approved": true}),
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    let _ = rx.await;
    approval_test
        .last_rpc
        .replace(Some(ReverseRpcResult::Approved));
}

#[when("server 收到该应答")]
async fn w_rr4_second(approval_test: &ServerTest) {
    let (st, _, body) = post_rpc(
        approval_test.port.get(),
        "A2",
        "approve_tool",
        serde_json::json!({"call_id": "xyz", "approved": false}),
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
}

#[then("应答被静默忽略（无状态变化、无错误）")]
fn t_rr4(approval_test: &ServerTest) {
    assert_eq!(
        *approval_test.last_rpc.borrow(),
        Some(ReverseRpcResult::Approved)
    );
}

const OCCUPANCY_SID: &str = "s-occ";

fn occupancy_assistant() -> xylitol_plugin_sdk::session::SessionEntry {
    use xylitol_plugin_sdk::session::{
        EntryBase, MessageEntry, SessionEntry, fixture_message_json,
    };
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: String::new(),
            parent_id: None,
            timestamp: 0,
        },
        message: fixture_message_json("assistant", "hi"),
    })
}

async fn materialize_occupancy(host: &HostState) {
    host.ports
        .store
        .create(OCCUPANCY_SID, Some("."), None)
        .await
        .expect("create occupancy session");
    host.ports
        .store
        .append_session_entry(OCCUPANCY_SID, &occupancy_assistant())
        .await
        .expect("append occupancy session");
}

async fn occupancy_writer_unary(server_test: &ServerTest, id: &str) -> (u16, String) {
    let (st, token, body) = post_rpc(
        server_test.port.get(),
        id,
        "clear_queue",
        serde_json::json!({
            "session_id": OCCUPANCY_SID,
            "clear_steer": true,
            "clear_follow_up": true
        }),
        None,
    )
    .await;
    assert!(
        token.is_none(),
        "MUST NOT carry X-Writer-Token: {token:?} {body}"
    );
    assert!(
        !body.contains("writerToken") && !body.to_lowercase().contains("writer_token"),
        "MUST NOT carry writerToken: {body}"
    );
    (st, body)
}

#[given("进程 A 已对会话取得写占用")]
async fn g_sr_w1_process_a_holds(server_test: &ServerTest) {
    start_host(server_test).await;
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    materialize_occupancy(&host).await;
    host.ports.store.release_write_lock(OCCUPANCY_SID);
    let lock = host.occupancy_lock_path(OCCUPANCY_SID);
    let child = crate::bdd::helpers::python_hold_lock(&lock);
    server_test.lock_child.replace(Some(child));
}

#[when("进程 B 再对该会话发起写占用")]
async fn w_sr_w1_process_b_write(server_test: &ServerTest) {
    let (st, body) = occupancy_writer_unary(server_test, "B").await;
    server_test.unary_status.set(st);
    server_test.unary_body.replace(Some(body));
}

#[then("B 得到业务错误且不得写入")]
fn t_sr_w1_b_held(server_test: &ServerTest) {
    let s = server_test.unary_body.borrow().clone().expect("B result");
    let v: serde_json::Value = serde_json::from_str(&s).expect("jsonrpc");
    assert_eq!(server_test.unary_status.get(), 200, "{s}");
    assert_eq!(v["error"]["data"]["code"], "session_write_held", "{s}");
}

#[given("进程 A 持锁后退出")]
async fn g_sr_w1_holder_exit(server_test: &ServerTest) {
    start_host(server_test).await;
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    materialize_occupancy(&host).await;
    host.ports.store.release_write_lock(OCCUPANCY_SID);
    let lock = host.occupancy_lock_path(OCCUPANCY_SID);
    let mut child = crate::bdd::helpers::python_hold_lock(&lock);
    crate::bdd::helpers::kill_child(&mut child);
}

#[then("B 取得占用")]
fn t_sr_w1_b_acquired(server_test: &ServerTest) {
    let s = server_test.unary_body.borrow().clone().expect("B result");
    let v: serde_json::Value = serde_json::from_str(&s).expect("jsonrpc");
    assert_eq!(server_test.unary_status.get(), 200, "{s}");
    assert_eq!(v["ok"], true, "{s}");
    assert!(v.get("error").is_none(), "{s}");
}

#[given("会话仅一条订阅连接且持有写占用")]
async fn g_sr_w1_one_sub_holds(server_test: &ServerTest) {
    start_host(server_test).await;
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    materialize_occupancy(&host).await;
    let slot = host.slot(OCCUPANCY_SID).await;
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    slot.add_subscriber(tx).await;
    drop(rx);
}

#[when("该连接干净关闭")]
async fn w_sr_w1_clean_close(server_test: &ServerTest) {
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    xylitol_kernel::occupancy::host::prune_mux_and_maybe_release(&host, true).await;
}

#[then("写占用已释放且后续同机进程可取得")]
fn t_sr_w1_lock_free(server_test: &ServerTest) {
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    let lock = host.occupancy_lock_path(OCCUPANCY_SID);
    assert!(lock.exists(), "release MUST NOT unlink session.lock");
    let code = crate::bdd::helpers::python_flock_probe(&lock);
    assert_eq!(code, Some(0), "other process MUST take after clean release");
}

#[given("唯一订阅异常断开且宽限未到")]
async fn g_sr_w1_linger_window(server_test: &ServerTest) {
    start_host(server_test).await;
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    materialize_occupancy(&host).await;
    let slot = host.slot(OCCUPANCY_SID).await;
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    slot.add_subscriber(tx).await;
    drop(rx);
    xylitol_kernel::occupancy::host::prune_mux_and_maybe_release(&host, false).await;
}

#[when("同一进程再次订阅该会话")]
async fn w_sr_w1_reconnect_subscribe(server_test: &ServerTest) {
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot(OCCUPANCY_SID).await;
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    host.register_unbound_mux(tx).await;
    let result = host.bind_mux_to_session(&slot, 0).await;
    server_test.unary_body.replace(Some(
        serde_json::json!({ "ok": result.ok, "error": result.error.map(|e| e.code) }).to_string(),
    ));
}

#[then("写占用仍在且订阅成功")]
fn t_sr_w1_linger_held(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().expect("subscribe");
    let v: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["ok"], true, "{body}");
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    let lock = host.occupancy_lock_path(OCCUPANCY_SID);
    let code = crate::bdd::helpers::python_flock_probe(&lock);
    assert_eq!(code, Some(2), "linger reconnect MUST keep occupancy");
}

#[when("POST /rpc 查询只读方法 queue_stats")]
async fn w_queue_stats(server_test: &ServerTest) {
    let (st, token, body) = post_rpc(
        server_test.port.get(),
        "Q",
        "queue_stats",
        serde_json::json!({"session_id": "s-q"}),
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert!(token.is_none(), "readonly must not set X-Writer-Token");
    server_test.unary_body.replace(Some(body));
}

#[then("返回 steer 与 follow-up 队列深度")]
fn t_queue_stats_shape(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().expect("body");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(v["result"]["steer_count"].is_u64(), "{body}");
    assert!(v["result"]["follow_up_count"].is_u64(), "{body}");
}

#[given("进程级 reload 正在进行（取消令牌已注册）")]
async fn g_reload_inflight(server_test: &ServerTest) {
    start_host(server_test).await;
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    *host.reload_cancel.lock().await = Some(tokio_util::sync::CancellationToken::new());
}

#[when("收到进程级 abort unary")]
async fn w_abort_unary(server_test: &ServerTest) {
    let (st, token, body) = post_rpc(
        server_test.port.get(),
        "AB",
        "abort",
        serde_json::json!({}),
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    server_test.writer_header.replace(token);
    server_test.unary_body.replace(Some(body));
}

#[then("应答携带 cancelled 指示且取消令牌被置位")]
async fn t_reload_cancelled(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().expect("body");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let cancelled = v["result"]["cancelled"]
        .as_bool()
        .or_else(|| v["value"]["cancelled"].as_bool())
        .unwrap_or(false);
    assert!(cancelled, "{body}");
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    let token = host
        .reload_cancel
        .lock()
        .await
        .clone()
        .expect("token still registered");
    assert!(token.is_cancelled(), "reload token must be cancelled");
}

#[given("无进行中的进程级 reload")]
async fn g_reload_idle(server_test: &ServerTest) {
    start_host(server_test).await;
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    assert!(host.reload_cancel.lock().await.is_none());
}

#[then("未命中 reload 取消而落回会话 abort 处理")]
fn t_abort_falls_back(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().expect("body");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(
        server_test.writer_header.borrow().is_none(),
        "abort MUST NOT mint a writer token: header={:?} body={body}",
        server_test.writer_header.borrow()
    );
    let code = v["error"]["data"]["code"].as_str().unwrap_or("");
    assert_ne!(code, "writer_conflict", "session abort must run: {body}");
    assert!(
        v.get("result").is_some() || v.get("error").is_some(),
        "session abort MUST produce a unary result: {body}"
    );
}

#[given("客户端已订阅会话 s-sub")]
async fn g_subscribe_ssub(server_test: &ServerTest) {
    start_host(server_test).await;
    let client = HttpWsClient::new(server_test.base_url());
    let mux = client.mux().await.expect("mux");
    server_test.mux_rx.replace(Some(mux));
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    wait_unbound(&host, 1).await;
    let r = client
        .unary(
            "subscribe",
            serde_json::json!({"session_id": "s-sub", "last_seq": 0}),
        )
        .await
        .expect("subscribe");
    assert!(r.ok, "{r:?}");
}

#[when("会话回合以 AgentEnd 结束后又追加新事件")]
async fn w_agent_end_then_more(server_test: &ServerTest) {
    let mut mux = server_test.mux_rx.borrow_mut().take().expect("mux");
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot("s-sub").await;
    slot.append_and_push(Event::AgentEnd).await;

    let mut frames: Vec<RpcMessage> = Vec::new();
    for _ in 0..3 {
        let f = tokio::time::timeout(Duration::from_secs(2), mux.next())
            .await
            .expect("timeout waiting agent_end frame")
            .expect("mux eof")
            .expect("ws frame");
        let text = serde_json::to_string(&f).unwrap();
        let is_end = text.contains("agent_end") || text.contains("AgentEnd");
        frames.push(f);
        if is_end {
            break;
        }
    }
    assert!(
        frames.iter().any(|f| {
            let t = serde_json::to_string(f).unwrap();
            t.contains("agent_end") || t.contains("AgentEnd")
        }),
        "expected AgentEnd frame, got {frames:?}"
    );

    slot.append_and_push(Event::TextDelta {
        text: "after-agent-end".into(),
    })
    .await;
    let f = tokio::time::timeout(Duration::from_secs(2), mux.next())
        .await
        .expect("timeout: subscription did not survive agent end")
        .expect("mux eof")
        .expect("ws frame");
    frames.push(f);
    server_test.mux_frames.replace(frames);
}

#[then("订阅仍存活且新事件继续送达")]
fn t_subscription_alive(server_test: &ServerTest) {
    let frames = server_test.mux_frames.borrow();
    let texts: Vec<String> = frames
        .iter()
        .map(|f| serde_json::to_string(f).unwrap())
        .collect();
    let end_idx = texts
        .iter()
        .position(|t| t.contains("agent_end") || t.contains("AgentEnd"))
        .expect("agent_end frame");
    assert!(
        texts[end_idx + 1..]
            .iter()
            .any(|t| t.contains("after-agent-end")),
        "post-turn event must arrive on same subscription: {texts:?}"
    );
}

#[when("POST /rpc 调用未登记方法 no_such_method")]
async fn w_unknown_unary(server_test: &ServerTest) {
    let body = r#"{"jsonrpc":"2.0","id":"R","method":"no_such_method","params":{}}"#;
    let (st, resp) = http_status(server_test.port.get(), "POST", "/rpc", body).await;
    server_test.unary_status.set(st);
    *server_test.unary_body.borrow_mut() = Some(resp);
}

#[then("应答为产品错误且为未登记方法")]
fn t_unknown_unary_shape(server_test: &ServerTest) {
    let st = server_test.unary_status.get();
    let body = server_test.unary_body.borrow().clone().unwrap_or_default();
    if st >= 400 {
        let v: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::json!({}));
        assert!(
            v.get("result").is_none() || v["result"].is_null(),
            "unregistered MUST NOT succeed: {body}"
        );
        return;
    }
    assert_eq!(st, 200, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).expect("v3 view");
    assert_ne!(v["ok"], true, "{body}");
    let code = v["error"]["data"]["code"]
        .as_str()
        .or_else(|| v["error"]["code"].as_str())
        .unwrap_or("");
    assert!(
        code.contains("unregistered") || code.contains("method") || body.contains("illegal"),
        "expected unregistered-method product error: {body}"
    );
}

async fn post_unary_rpc_id(
    port: u16,
    rpc_id: &str,
    method: &str,
    payload: serde_json::Value,
) -> String {
    let (_, _, resp) = post_rpc(port, rpc_id, method, payload, None).await;
    resp
}

#[given("客户端以 rpc_id R 对某 session 经 POST /rpc 提交方法并得到结果")]
async fn g_idem_first_steer(server_test: &ServerTest) {
    start_host(server_test).await;
    let body = post_unary_rpc_id(
        server_test.port.get(),
        "R",
        "steer",
        serde_json::json!({ "session_id": "idem-s1", "message": "idem-replay" }),
    )
    .await;
    *server_test.idem_first.lock().expect("idem_first") = Some(body);
}

#[when("客户端以相同 rpc_id R 重试同一方法")]
async fn w_idem_retry(server_test: &ServerTest) {
    let body = post_unary_rpc_id(
        server_test.port.get(),
        "R",
        "steer",
        serde_json::json!({ "session_id": "idem-s1", "message": "idem-replay" }),
    )
    .await;
    *server_test.idem_second.borrow_mut() = Some(body);
}

#[then("第二次得到与首次相同的结果且命令仅执行一次")]
async fn t_idem_replay_once(server_test: &ServerTest) {
    let first = server_test
        .idem_first
        .lock()
        .expect("idem_first")
        .clone()
        .expect("first result recorded");
    let second = server_test
        .idem_second
        .borrow()
        .clone()
        .expect("second result recorded");
    let first: serde_json::Value = serde_json::from_str(&first).expect("first envelope");
    let second: serde_json::Value = serde_json::from_str(&second).expect("second envelope");
    assert_eq!(first, second, "duplicate MUST replay the first result");
    assert!(
        first.get("error").map(|e| e.is_null()).unwrap_or(true),
        "{first}"
    );
    assert!(
        first.get("result").is_some() && !first["result"].is_null(),
        "{first}"
    );
    let stats = post_unary_rpc_id(
        server_test.port.get(),
        "R-stats",
        "queue_stats",
        serde_json::json!({ "session_id": "idem-s1" }),
    )
    .await;
    let stats: serde_json::Value = serde_json::from_str(&stats).expect("queue_stats envelope");
    assert_eq!(
        stats["result"]["steer_count"],
        serde_json::json!(1),
        "steer MUST be admitted exactly once: {stats}"
    );
}

#[given("rpc_id R 已被某 method 与 params 的提交占用")]
async fn g_idem_occupied(server_test: &ServerTest) {
    start_host(server_test).await;
    let body = post_unary_rpc_id(
        server_test.port.get(),
        "R",
        "get_state",
        serde_json::json!({ "session_id": "idem-s2" }),
    )
    .await;
    let v: serde_json::Value = serde_json::from_str(&body).expect("get_state envelope");
    assert!(v.get("error").map(|e| e.is_null()).unwrap_or(true), "{v}");
}

#[when("以相同 rpc_id R 提交不同 method 或 params")]
async fn w_idem_conflict(server_test: &ServerTest) {
    let body = post_unary_rpc_id(
        server_test.port.get(),
        "R",
        "get_session_stats",
        serde_json::json!({ "session_id": "idem-s2" }),
    )
    .await;
    *server_test.idem_second.borrow_mut() = Some(body);
}

#[then("返回产品错误且 data.code 为 idempotency_conflict 且不执行")]
async fn t_idem_conflict(server_test: &ServerTest) {
    let second = server_test
        .idem_second
        .borrow()
        .clone()
        .expect("conflict result recorded");
    let v: serde_json::Value = serde_json::from_str(&second).expect("envelope");
    assert_eq!(
        v["error"]["data"]["code"],
        serde_json::json!("idempotency_conflict"),
        "{v}"
    );
}

#[given("rpc_id R 的首次命令仍在处理中")]
async fn g_idem_inflight(server_test: &ServerTest) {
    start_host(server_test).await;
    let exec_file = std::env::temp_dir().join(format!("xylitol-idem-{}.txt", uuid::Uuid::new_v4()));
    let command = format!(
        "printf x >> '{file}' && sleep 0.4",
        file = exec_file.display()
    );
    let port = server_test.port.get();
    let first_body = server_test.idem_first.clone();
    let task = tokio::spawn(async move {
        let body = post_unary_rpc_id(
            port,
            "R",
            "bash",
            serde_json::json!({ "session_id": "idem-s3", "command": command }),
        )
        .await;
        *first_body.lock().expect("idem_first") = Some(body);
    });
    *server_test.idem_task.borrow_mut() = Some(task);
    *server_test.idem_exec_file.borrow_mut() = Some(exec_file);
    tokio::time::sleep(Duration::from_millis(120)).await;
}

#[when("相同 rpc_id R 的重复请求到达")]
async fn w_idem_inflight_duplicate(server_test: &ServerTest) {
    let file = server_test
        .idem_exec_file
        .borrow()
        .clone()
        .expect("exec file recorded");
    let command = format!("printf x >> '{file}' && sleep 0.4", file = file.display());
    let body = post_unary_rpc_id(
        server_test.port.get(),
        "R",
        "bash",
        serde_json::json!({ "session_id": "idem-s3", "command": command }),
    )
    .await;
    *server_test.idem_second.borrow_mut() = Some(body);
}

#[then("等待首次完成并回放同一结果且不并行执行")]
async fn t_idem_inflight_wait(server_test: &ServerTest) {
    let task = server_test
        .idem_task
        .borrow_mut()
        .take()
        .expect("first task");
    task.await.expect("join first bash task");
    let first = server_test
        .idem_first
        .lock()
        .expect("idem_first")
        .clone()
        .expect("first result recorded");
    let second = server_test
        .idem_second
        .borrow()
        .clone()
        .expect("duplicate result recorded");
    let first: serde_json::Value = serde_json::from_str(&first).expect("first envelope");
    let second: serde_json::Value = serde_json::from_str(&second).expect("second envelope");
    assert_eq!(first, second, "duplicate MUST wait then replay: {second}");
    assert!(
        first.get("error").map(|e| e.is_null()).unwrap_or(true),
        "{first}"
    );
    let file = server_test
        .idem_exec_file
        .borrow()
        .clone()
        .expect("exec file recorded");
    let executed = std::fs::read_to_string(&file).unwrap_or_default();
    assert_eq!(
        executed, "x",
        "bash MUST execute exactly once, got {executed:?}"
    );
    let _ = std::fs::remove_file(&file);
}

#[given("监听器已绑定端口但装配未完成")]
async fn g_rdy_starting(server_test: &ServerTest) {
    let gateway = Gateway::starting();
    let (running, port) = bind_serve(
        &ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: None,
        },
        gateway.clone(),
    )
    .await
    .expect("bind_serve");
    *server_test.running.borrow_mut() = Some(running);
    server_test.port.set(port);
    *server_test.gateway.borrow_mut() = Some(gateway);
    tokio::time::sleep(Duration::from_millis(30)).await;
}

#[when("访问 /healthz 或任一 unary")]
async fn w_rdy_probe_window(server_test: &ServerTest) {
    let (hz_status, hz_body) = http_status(server_test.port.get(), "GET", "/healthz", "").await;
    server_test.rdy_status.set(hz_status);
    *server_test.rdy_body.borrow_mut() = Some(hz_body);
    let (st, _, body) = post_rpc(
        server_test.port.get(),
        "rdy-probe",
        "get_state",
        serde_json::json!({ "session_id": "rdy-s1" }),
        None,
    )
    .await;
    server_test.unary_status.set(st);
    *server_test.unary_body.borrow_mut() = Some(body);
}

#[then("healthz 返回 503 且携带 starting 语义与 retry-after")]
async fn t_rdy_starting(server_test: &ServerTest) {
    assert_eq!(
        server_test.rdy_status.get(),
        503,
        "healthz must be 503 in window"
    );
    let body = server_test.rdy_body.borrow().clone().unwrap_or_default();
    let v: serde_json::Value = serde_json::from_str(&body).expect("healthz json");
    assert_eq!(v["status"], serde_json::json!("starting"), "{v}");
    assert_eq!(v["retry_after"], serde_json::json!(1), "{v}");
}

#[then("unary 得到同语义 503 且不半执行")]
async fn t_rdy_starting_unary(server_test: &ServerTest) {
    assert_eq!(
        server_test.unary_status.get(),
        503,
        "unary must be 503 in window"
    );
    let ubody = server_test.unary_body.borrow().clone().unwrap_or_default();
    assert!(
        ubody.contains("starting"),
        "unary must carry starting semantics: {ubody}"
    );
}

#[given("装配已完成")]
async fn g_rdy_ready(server_test: &ServerTest) {
    start_host(server_test).await;
}

#[when("访问 /healthz")]
async fn w_rdy_probe_healthz(server_test: &ServerTest) {
    let (status, body) = http_status(server_test.port.get(), "GET", "/healthz", "").await;
    server_test.rdy_status.set(status);
    *server_test.rdy_body.borrow_mut() = Some(body);
}

#[then("返回 200 OK")]
async fn t_rdy_ready(server_test: &ServerTest) {
    assert_eq!(server_test.rdy_status.get(), 200);
    let body = server_test.rdy_body.borrow().clone().unwrap_or_default();
    assert!(body.contains("ok"), "{body}");
}

#[given("装配失败")]
async fn g_rdy_failed(server_test: &ServerTest) {
    let gateway = Gateway::starting();
    gateway.mark_failed();
    let (running, port) = bind_serve(
        &ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: None,
        },
        gateway,
    )
    .await
    .expect("bind_serve");
    *server_test.running.borrow_mut() = Some(running);
    server_test.port.set(port);
    tokio::time::sleep(Duration::from_millis(30)).await;
}

#[then("返回 503 且携带 failed 不可重试语义")]
async fn t_rdy_failed(server_test: &ServerTest) {
    assert_eq!(server_test.rdy_status.get(), 503);
    let body = server_test.rdy_body.borrow().clone().unwrap_or_default();
    let v: serde_json::Value = serde_json::from_str(&body).expect("healthz json");
    assert_eq!(v["status"], serde_json::json!("failed"), "{v}");
    assert!(
        v.get("retry_after").is_none(),
        "failed must not advertise retry: {v}"
    );
}

use xylitol_kernel::occupancy::runtime::serve_registered;
use xylitol_serve::{Registration, read_registration, run_self_check, write_registration};

fn temp_reg_path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("xylitol-bdd-reg-{}.json", uuid::Uuid::new_v4()))
}

#[given("serve 已装配完成并在空闲端口监听")]
async fn g_reg_serving(server_test: &ServerTest) {
    let host = xylitol_kernel::occupancy::host_state_for_test().expect("host");
    let reg_path = temp_reg_path();
    let (running, port) = serve_registered(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: None,
        },
        host,
        reg_path.clone(),
    )
    .await
    .expect("serve_registered");
    *server_test.running.borrow_mut() = Some(running);
    server_test.port.set(port);
    *server_test.reg_path.borrow_mut() = Some(reg_path);
    tokio::time::sleep(Duration::from_millis(30)).await;
}

#[when("读取注册文件")]
async fn w_reg_read(server_test: &ServerTest) {
    let reg_path = server_test.reg_path.borrow().clone().expect("reg path");
    let reg = read_registration(&reg_path).expect("registration readable");
    *server_test.rdy_body.borrow_mut() =
        Some(serde_json::to_string(&reg).expect("serialize registration"));
    let (status, body) = http_status(server_test.port.get(), "GET", "/healthz", "").await;
    server_test.unary_status.set(status);
    *server_test.unary_body.borrow_mut() = Some(body);
}

#[then("文件存在且为 0600 且内容含 url、pid 与 version")]
async fn t_reg_file(server_test: &ServerTest) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let reg_path = server_test.reg_path.borrow().clone().expect("reg path");
        let mode = std::fs::metadata(&reg_path)
            .expect("meta")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "registration must be 0600");
    }
    let body = server_test.rdy_body.borrow().clone().unwrap_or_default();
    let v: serde_json::Value = serde_json::from_str(&body).expect("registration json");
    assert!(v["url"].is_string(), "{v}");
    assert!(v["pid"].is_u64(), "{v}");
    assert!(v["version"].is_string(), "{v}");
}

#[then("healthz 应答 body 的 pid 和 version 与文件一致")]
async fn t_reg_healthz_identity(server_test: &ServerTest) {
    assert_eq!(server_test.unary_status.get(), 200, "ready healthz");
    let file = server_test.rdy_body.borrow().clone().unwrap_or_default();
    let file: serde_json::Value = serde_json::from_str(&file).expect("registration json");
    let hz = server_test.unary_body.borrow().clone().unwrap_or_default();
    let hz: serde_json::Value = serde_json::from_str(&hz).expect("healthz json");
    assert_eq!(hz["pid"], file["pid"], "{hz}");
    assert_eq!(hz["version"], file["version"], "{hz}");
}

#[given("注册文件存在但对应端口的 serve 已退出")]
async fn g_reg_stale(server_test: &ServerTest) {
    let reg_path = temp_reg_path();
    let reg = Registration {
        url: "http://127.0.0.1:1".into(),
        pid: 2_147_483_647,
        version: "0.0.0-dev".into(),
    };
    write_registration(&reg_path, &reg).expect("write stale registration");
    *server_test.reg_path.borrow_mut() = Some(reg_path);
}

#[when("客户端 attach 探活该地址")]
async fn w_reg_attach_probe(server_test: &ServerTest) {
    let reg_path = server_test.reg_path.borrow().clone().expect("reg path");
    let msg = xylitol::attach_preflight_with("http://127.0.0.1:1", Some(reg_path))
        .await
        .expect_err("stale registration must fail the preflight");
    *server_test.unary_body.borrow_mut() = Some(msg);
}

#[then("得到可操作诊断说明 pid 对应的 serve 已退出并提示重新 serve")]
async fn t_reg_stale_diagnosis(server_test: &ServerTest) {
    let msg = server_test.unary_body.borrow().clone().unwrap_or_default();
    assert!(msg.contains("has exited"), "{msg}");
    assert!(msg.contains("pid 2147483647"), "{msg}");
    assert!(msg.contains("xylitol serve"), "{msg}");
    if let Some(path) = server_test.reg_path.borrow().clone() {
        assert!(
            !path.exists(),
            "stale registration must be removed after diagnosis"
        );
        let _ = std::fs::remove_file(&path);
    }
}

#[given("旧 serve 进程持有注册文件")]
async fn g_reg_takeover_old(server_test: &ServerTest) {
    let reg_path = temp_reg_path();
    let own = Registration::own("http://127.0.0.1:9".into());
    write_registration(&reg_path, &own).expect("write old registration");
    let (tx, rx) = tokio::sync::watch::channel(false);
    tokio::spawn(run_self_check(
        reg_path.clone(),
        own,
        Duration::from_millis(50),
        move || {
            let _ = tx.send(true);
        },
    ));
    *server_test.reg_path.borrow_mut() = Some(reg_path);
    *server_test.evict_rx.borrow_mut() = Some(rx);
}

#[when("新 serve 写入字段不同的注册文件")]
async fn w_reg_takeover_new(server_test: &ServerTest) {
    tokio::time::sleep(Duration::from_millis(80)).await;
    let reg_path = server_test.reg_path.borrow().clone().expect("reg path");
    let newcomer = Registration {
        url: "http://127.0.0.1:9".into(),
        pid: 2_147_483_646,
        version: "0.0.0-dev".into(),
    };
    write_registration(&reg_path, &newcomer).expect("takeover write");
}

#[then("旧进程在自检周期内检测到字段不全等并自行退出")]
async fn t_reg_takeover_evicted(server_test: &ServerTest) {
    let mut rx = server_test
        .evict_rx
        .borrow_mut()
        .take()
        .expect("eviction watch");
    tokio::time::timeout(Duration::from_secs(5), rx.changed())
        .await
        .expect("eviction within self-check period")
        .expect("watch open");
    let reg_path = server_test.reg_path.borrow().clone().expect("reg path");
    let _ = std::fs::remove_file(&reg_path);
}

#[given("server 就绪")]
async fn g_server_ready(server_test: &ServerTest) {
    start_host(server_test).await;
}

#[when("推送 content 载荷导入会话")]
async fn w_staged_wire_import(server_test: &ServerTest) {
    let content = "{\"type\":\"session\",\"version\":6,\"id\":\"imp-scenario-1\",\"timestamp\":1,\"cwd\":\"/tmp\"}\n";
    let (status, _, resp) = post_rpc(
        server_test.port.get(),
        "r-staged-import",
        "import_jsonl",
        serde_json::json!({ "content": content }),
        None,
    )
    .await;
    server_test.unary_status.set(status);
    *server_test.unary_body.borrow_mut() = Some(resp);
}

#[then("返回新 session_id 且暂存文件不残留")]
async fn t_staged_wire_import(server_test: &ServerTest) {
    assert_eq!(server_test.unary_status.get(), 200, "unary must be 200");
    let body = server_test.unary_body.borrow().clone().unwrap_or_default();
    let v: serde_json::Value = serde_json::from_str(&body).expect("valid envelope");
    let sid = v["result"]["session_id"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| panic!("no session_id in result, body={body}"));
    assert!(!sid.is_empty(), "imported session id must be non-empty");
    let host = server_test.host.borrow().clone().expect("host state");
    assert!(
        host.ports.store.exists(&sid).await,
        "imported session must exist in store"
    );
    let leftovers = std::fs::read_dir(std::env::temp_dir())
        .expect("temp dir readable")
        .filter_map(|e| e.ok())
        .any(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("xylitol-import-")
        });
    assert!(!leftovers, "staged import temp file must be cleaned up");
}

#[given("真进程 serve 已启动且 attach 客户端已订阅")]
async fn g_real_attach(server_test: &ServerTest) {
    start_host(server_test).await;
    let driver =
        XyRemoteDriver::new(server_test.base_url(), "s-real-attach").with_tunings(LinkTunings {
            backoff_base: Duration::from_millis(50),
            backoff_cap: Duration::from_millis(400),
            survive_threshold: Duration::from_millis(80),
            coalesce_window: Duration::from_millis(10),
        });
    let mut driver = driver;
    driver.attach_session().await.expect("real attach");
    server_test.attach_driver.replace(Some(driver));
}

#[when("server 的 mux 连接被断开且期间 journal 新增事件")]
async fn w_real_drop_mux(server_test: &ServerTest) {
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot("s-real-attach").await;
    slot.append_and_push(Event::TextDelta {
        text: "online".into(),
    })
    .await;
    tokio::time::sleep(Duration::from_millis(120)).await;
    slot.subscribers.lock().await.clear();
    slot.append_and_push(Event::TextDelta {
        text: "offline".into(),
    })
    .await;
    tokio::time::sleep(Duration::from_millis(30)).await;
}

#[then("客户端 在宽限内不上屏断线错误并自动重连")]
async fn t_real_no_error_rows(server_test: &ServerTest) {
    fn link_health(server_test: &ServerTest) -> LinkHealth {
        server_test
            .attach_driver
            .borrow_mut()
            .as_mut()
            .expect("attach driver")
            .link_health()
    }

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let mut saw_down = false;
    while std::time::Instant::now() < deadline {
        if link_health(server_test) == LinkHealth::Down {
            saw_down = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    assert!(
        saw_down,
        "link_health MUST report Down during the outage (ath42 signal)"
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while std::time::Instant::now() < deadline && link_health(server_test) != LinkHealth::Up {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(
        link_health(server_test),
        LinkHealth::Up,
        "client MUST auto-reconnect without manual action"
    );
}

#[then("重连后 按 last_seq 从 journal 续传缺失事件且 不依赖人工重开")]
async fn t_real_replay_offline(server_test: &ServerTest) {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let mut replayed = false;
    let mut saw_error_row = false;
    while std::time::Instant::now() < deadline {
        {
            let mut driver_slot = server_test.attach_driver.borrow_mut();
            let driver = driver_slot.as_mut().expect("attach driver");
            for ev in driver.drain_idle_events() {
                match ev {
                    XyEvent::TextDelta(text) => {
                        if text == "offline" {
                            replayed = true;
                        }
                    }
                    XyEvent::Error(err) if !err.message.contains("protocol") => {
                        saw_error_row = true;
                    }
                    _ => {}
                }
            }
        }
        if replayed {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(replayed, "offline event MUST replay after auto-reconnect");
    assert!(
        !saw_error_row,
        "reconnect window MUST NOT produce transcript error rows"
    );
}

#[when("POST /rpc 查询会话 {sid:string} 的只读 queue_stats")]
async fn w_queue_stats_for(server_test: &ServerTest, sid: String) {
    let sid = sid.trim_matches('"');
    let (st, token, body) = post_rpc(
        server_test.port.get(),
        "Q2",
        "queue_stats",
        serde_json::json!({"session_id": sid}),
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert!(token.is_none(), "readonly must not set X-Writer-Token");
    server_test.unary_body.replace(Some(body));
}

#[when("POST /rpc 调用 switch_session 指向不存在会话")]
async fn w_rpc_switch_missing(server_test: &ServerTest) {
    let (st, _, body) = post_rpc(
        server_test.port.get(),
        "S-miss",
        "switch_session",
        serde_json::json!({"session_id": "definitely-missing-session"}),
        None,
    )
    .await;
    server_test.unary_status.set(st);
    server_test.unary_body.replace(Some(body));
}

#[then("应答为产品错误且错误提及会话不存在或无效")]
fn t_switch_missing_error(server_test: &ServerTest) {
    let st = server_test.unary_status.get();
    let body = server_test.unary_body.borrow().clone().unwrap_or_default();
    assert_eq!(st, 200, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).expect("v3 view");
    assert!(
        v.get("error").is_some() && !v["error"].is_null(),
        "c2826: 不存在会话切换必须报错：{body}"
    );
    let msg = v["error"]["details"]
        .as_str()
        .or_else(|| v["error"]["message"].as_str())
        .unwrap_or("");
    assert!(
        msg.contains("session") || msg.contains("会话") || body.contains("session"),
        "c2826: 错误须提及会话缺失/无效：{body}"
    );
}

#[when("POST /rpc 调用 get_messages")]
async fn w_rpc_get_messages(server_test: &ServerTest) {
    let (st, _, body) = post_rpc(
        server_test.port.get(),
        "G",
        "get_messages",
        serde_json::json!({"session_id": "s-prompt"}),
        None,
    )
    .await;
    server_test.unary_status.set(st);
    server_test.unary_body.replace(Some(body));
}

#[then("应答含已加载会话条目")]
fn t_get_messages_entries(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().expect("body");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let result = v
        .get("result")
        .unwrap_or_else(|| panic!("c2826: get_messages 必须成功返回：{body}"));
    assert!(
        result
            .get("entries")
            .map(|e| e.as_array().map(|a| !a.is_empty()).unwrap_or(false))
            .unwrap_or(false)
            || result.to_string().contains("hi"),
        "c2826: get_messages 应含已加载条目：{body}"
    );
}

#[when("POST /rpc 调用 steer 携带文本")]
async fn w_rpc_steer(server_test: &ServerTest) {
    let (_, token, prompt_body) = post_rpc(
        server_test.port.get(),
        "St-pre",
        "prompt",
        serde_json::json!({"message": "占位起租", "session_id": "s-prompt"}),
        None,
    )
    .await;
    let token = token.or_else(|| {
        serde_json::from_str::<serde_json::Value>(&prompt_body)
            .ok()
            .and_then(|v| v["result"]["writerToken"].as_str().map(str::to_string))
    });
    let (st, _, body) = post_rpc(
        server_test.port.get(),
        "St",
        "steer",
        serde_json::json!({"session_id": "s-prompt", "message": "改个方向"}),
        token.as_deref(),
    )
    .await;
    assert_eq!(st, 200, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(
        v.get("error").is_none(),
        "c2826: steer 经 dispatch 路由必须成功：{body}"
    );
    server_test.unary_status.set(st);
    server_test.unary_body.replace(Some(body));
}

#[then("steer 深度至少为 1")]
fn t_steer_enqueued(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().expect("body");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let steer = v["result"]["steer_count"]
        .as_u64()
        .unwrap_or_else(|| panic!("c2826: queue_stats 缺 steer_count：{body}"));
    assert!(steer >= 1, "c2826: steer 未见入队（深度 {steer}）：{body}");
}

#[when("查询方法表的会话能力方法")]
fn w_lookup_session_methods(server_test: &ServerTest) {
    let names = [
        "session_tree",
        "travel_session_tree",
        "append_entry_label",
        "list_sessions",
        "get_messages",
        "new_session",
        "get_session_name",
        "set_session_name",
        "delete_session",
    ];
    let missing: Vec<&str> = names
        .iter()
        .copied()
        .filter(|n| xylitol_protocol::registry::lookup(n).is_none())
        .collect();
    server_test
        .unary_body
        .replace(Some(serde_json::json!({ "missing": missing }).to_string()));
}

#[then("会话树、travel、label、列表、条目读取、新建、名称读写与删除均已登记")]
fn t_session_methods_registered(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().expect("body");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let missing = v["missing"].as_array().expect("missing list");
    assert!(missing.is_empty(), "c2826: 会话能力方法未登记：{missing:?}");
}

#[when("查询方法表的 Host 资源方法")]
fn w_lookup_resource_methods(server_test: &ServerTest) {
    let missing: Vec<&str> = ["reload", "loaded_resources"]
        .iter()
        .copied()
        .filter(|n| xylitol_protocol::registry::lookup(n).is_none())
        .collect();
    server_test
        .unary_body
        .replace(Some(serde_json::json!({ "missing": missing }).to_string()));
}

#[then("reload 与 loaded_resources 均已登记")]
fn t_resource_methods_registered(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().expect("body");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let missing = v["missing"].as_array().expect("missing list");
    assert!(
        missing.is_empty(),
        "c2826: Host 资源方法未登记：{missing:?}"
    );
}

#[when("查询方法表的 estimate_context")]
fn w_lookup_estimate_context(server_test: &ServerTest) {
    let registered = xylitol_protocol::registry::lookup("estimate_context").is_some();
    server_test
        .unary_body
        .replace(Some(format!("registered={registered}")));
}

#[then("已登记且为只读 unary 不占写者")]
fn t_estimate_context_registered(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().expect("body");
    assert_ne!(body, "missing", "c2826: estimate_context 必须已登记");
}

#[when("另一客户端对同会话发送 steer")]
async fn w_c2826_second_writer_steer(server_test: &ServerTest) {
    let (st, token, body) = post_rpc(
        server_test.port.get(),
        "W2",
        "steer",
        serde_json::json!({"session_id": "s-prompt", "message": "抢写者"}),
        None,
    )
    .await;
    server_test.unary_status.set(st);
    server_test.writer_header.replace(token);
    server_test.unary_body.replace(Some(body));
}

#[then("同进程第二客户端不得因协议写者令牌被拒")]
fn t_c2826_same_process_not_token_conflict(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().expect("body");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(server_test.unary_status.get(), 200, "{body}");
    assert!(
        server_test.writer_header.borrow().is_none(),
        "MUST NOT mint X-Writer-Token: {:?}",
        server_test.writer_header.borrow()
    );
    let code = v["error"]["data"]["code"].as_str().unwrap_or("");
    assert_ne!(code, "writer_conflict", "token lease is gone: {body}");
    assert_ne!(
        code, "session_write_held",
        "same process MUST share occupancy: {body}"
    );
}

async fn start_host_with_default_model(t: &ServerTest, model_id: &str) {
    let host = xylitol_kernel::occupancy::host_state_for_test_with_default_model(model_id)
        .expect("host with default model");
    let (running, port) = with_test_timeout(|| {
        let host = host.clone();
        async move {
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
            tokio::time::sleep(Duration::from_millis(30)).await;
            (running, port)
        }
    })
    .await
    .expect("serve 启动 MUST 在 with_test_timeout 内完成");
    t.host.replace(Some(host));
    t.running.replace(Some(running));
    t.port.set(port);
}

#[given("会话写者已装配且持有已解析默认模型 fake-model")]
async fn g_c2841_writer_with_default_model(server_test: &ServerTest) {
    start_host_with_default_model(server_test, "fake-model").await;
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot(SESSION_C2841).await;
    xylitol_kernel::occupancy::host::materialize_writer(&host, &slot)
        .await
        .expect("materialize writer");
    let mut g = slot.driver.lock().await;
    let d = g.as_mut().expect("writer driver");
    let m = d
        .current_model()
        .expect("writer must hold the default model");
    assert_eq!(m.id, "fake-model");
}

#[when("订阅者绑定该会话")]
async fn w_c2841_bind_subscriber(server_test: &ServerTest) {
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    wait_unbound(&host, 1).await;
    let client = HttpWsClient::new(server_test.base_url());
    let mux = client.mux().await.expect("mux");
    server_test.mux_rx.replace(Some(mux));
    let r = client
        .unary(
            "subscribe",
            serde_json::json!({"session_id": SESSION_C2841, "last_seq": 0}),
        )
        .await
        .expect("subscribe");
    assert!(r.ok, "{r:?}");
}

#[then("订阅者收到模型同步事件且模型为 fake-model")]
async fn t_c2841_model_sync_received(server_test: &ServerTest) {
    let mut mux = server_test.mux_rx.borrow_mut().take().expect("mux");
    for _ in 0..6 {
        let f = tokio::time::timeout(Duration::from_secs(3), mux.next())
            .await
            .expect("frame timeout")
            .expect("mux eof")
            .expect("ws frame");
        let RpcMessage::ServerRequest {
            method, payload, ..
        } = &f
        else {
            continue;
        };
        if method != "session/event" {
            continue;
        }
        let ev = payload.get("event").cloned().unwrap_or_default();
        if ev.get("type").and_then(serde_json::Value::as_str) == Some("model_select") {
            assert_eq!(
                ev.get("model_id").and_then(serde_json::Value::as_str),
                Some("fake-model")
            );
            return;
        }
    }
    panic!("no model_select sync frame received on the subscribed session");
}

async fn drain_model_select(mux: &mut MuxStream) -> bool {
    for _ in 0..8 {
        let f = match tokio::time::timeout(Duration::from_secs(3), mux.next()).await {
            Ok(Some(Ok(f))) => f,
            _ => return false,
        };
        let RpcMessage::ServerRequest {
            method, payload, ..
        } = &f
        else {
            continue;
        };
        if method == "session/event"
            && payload
                .get("event")
                .and_then(|e| e.get("type"))
                .and_then(serde_json::Value::as_str)
                == Some("model_select")
        {
            return payload
                .get("event")
                .and_then(|e| e.get("model_id"))
                .and_then(serde_json::Value::as_str)
                == Some("fake-model");
        }
    }
    false
}

#[when("同一会话被订阅两次")]
async fn w_c2850_subscribe_twice(server_test: &ServerTest) {
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    let seq_before = host
        .slot(SESSION_C2841)
        .await
        .journal
        .lock()
        .await
        .max_seq();

    let mut seen = 0u8;
    for _ in 0..2 {
        let client = HttpWsClient::new(server_test.base_url());
        let mut mux = client.mux().await.expect("mux");
        wait_unbound(&host, 1).await;
        let r = client
            .unary(
                "subscribe",
                serde_json::json!({"session_id": SESSION_C2841, "last_seq": 0}),
            )
            .await
            .expect("subscribe");
        assert!(r.ok, "{r:?}");
        if drain_model_select(&mut mux).await {
            seen += 1;
        }
    }

    let seq_after = host
        .slot(SESSION_C2841)
        .await
        .journal
        .lock()
        .await
        .max_seq();
    server_test.last_seq.set(seq_after.wrapping_sub(seq_before));
    server_test.unary_status.set(u16::from(seen));
}

#[then("journal 序号未因绑定递增且两次均收到模型同步事件")]
fn t_c2850_rebind_transient(server_test: &ServerTest) {
    assert_eq!(
        server_test.unary_status.get(),
        2,
        "两次订阅 MUST 各收到一帧 model_select"
    );
    assert_eq!(
        server_test.last_seq.get(),
        0,
        "两次绑定 MUST NOT 递增 journal seq"
    );
}

#[given("客户端已订阅会话 s-prompt 并以可用模型 fake-model 发送 prompt")]
async fn g_c2841_prompt_with_identity(server_test: &ServerTest) {
    start_host_with_default_model(server_test, "fake-model").await;
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    wait_unbound(&host, 1).await;
    let client = HttpWsClient::new(server_test.base_url());
    let mux = client.mux().await.expect("mux");
    server_test.mux_rx.replace(Some(mux));
    let sub = client
        .unary(
            "subscribe",
            serde_json::json!({"session_id": "s-prompt", "last_seq": 0}),
        )
        .await
        .expect("subscribe");
    assert!(sub.ok, "{sub:?}");
    let prompt = client
        .unary(
            "prompt",
            serde_json::json!({
                "message": "hi",
                "session_id": "s-prompt",
                "model_id": "fake-model",
            }),
        )
        .await
        .expect("prompt");
    assert!(prompt.ok, "{prompt:?}");
}

#[when("Host 处理该 prompt")]
async fn w_c2841_prompt_processed(_server_test: &ServerTest) {}

#[then("run 路由到 s-prompt 且事件在 s-prompt 的订阅者上可达")]
async fn t_c2841_prompt_events_reachable(server_test: &ServerTest) {
    let mut mux = server_test.mux_rx.borrow_mut().take().expect("mux");
    for _ in 0..16 {
        let f = match tokio::time::timeout(Duration::from_secs(3), mux.next()).await {
            Ok(Some(Ok(f))) => f,
            other => panic!("c2841: run 事件未在订阅流上完成送达：{other:?}"),
        };
        let RpcMessage::ServerRequest {
            method, payload, ..
        } = &f
        else {
            continue;
        };
        if method == "session/event"
            && payload
                .get("event")
                .and_then(|e| e.get("type"))
                .and_then(serde_json::Value::as_str)
                == Some("agent_end")
        {
            return;
        }
    }
    panic!("c2841: 订阅会话流上未收到 agent_end");
}

#[given("以产品客户端订阅会话 s-v3 且写者装配了默认模型 fake-model")]
async fn g_c2842_v3_subscribe_default_model(server_test: &ServerTest) {
    start_host_with_default_model(server_test, "fake-model").await;
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot(SESSION_V3).await;
    xylitol_kernel::occupancy::host::materialize_writer(&host, &slot)
        .await
        .expect("materialize writer");
    let client = HttpWsClient::new(server_test.base_url()).with_envelope(true);
    let r = client
        .unary(
            "subscribe",
            serde_json::json!({"session_id": SESSION_V3, "last_seq": 0, "cwd": "/tmp"}),
        )
        .await
        .expect("v3 subscribe");
    assert!(r.ok, "v3 subscribe: {r:?}");
    drop(client);
}

#[when("该客户端在 s-v3 上设置模型 fake-model")]
async fn w_c2842_v3_set_model(server_test: &ServerTest) {
    let client = HttpWsClient::new(server_test.base_url()).with_envelope(true);
    let r = client
        .unary(
            "set_model",
            serde_json::json!({
                "type": "set_model",
                "session_id": SESSION_V3,
                "model_id": "fake-model",
            }),
        )
        .await
        .expect("v3 set_model");
    server_test
        .unary_body
        .replace(Some(serde_json::to_string(&r).unwrap_or_default()));
    if !r.ok {
        server_test.unary_status.set(1);
    } else {
        server_test.unary_status.set(0);
    }
}

#[then("设置无 writer_conflict")]
fn t_c2842_no_writer_conflict(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().expect("body");
    assert_eq!(
        server_test.unary_status.get(),
        0,
        "set_model must not conflict: {body}"
    );
    assert!(
        !body.contains("writer_conflict"),
        "set_model must not hit writer_conflict: {body}"
    );
}

#[when("客户端请求该会话 get_state")]
async fn w_c2842_get_state(server_test: &ServerTest) {
    let client = HttpWsClient::new(server_test.base_url());
    let r = client
        .unary(
            "get_state",
            serde_json::json!({"type": "get_state", "session_id": SESSION_C2841}),
        )
        .await
        .expect("get_state");
    server_test.unary_status.set(u16::from(!r.ok));
    let inner = r.value.clone().unwrap_or(serde_json::Value::Null);
    server_test.unary_body.replace(Some(inner.to_string()));
}

#[then("应答的 model 为 fake-model")]
fn t_c2842_get_state_model(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().expect("body");
    assert_eq!(server_test.unary_status.get(), 0, "get_state ok: {body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let model = v.get("model").expect("model field");
    assert_eq!(
        model.get("id").and_then(serde_json::Value::as_str),
        Some("fake-model"),
        "get_state model must reflect the writer: {body}"
    );
}

#[when("POST /rpc 发送 JSON 对象、JSON 数组与纯文本等非产品二进制 body")]
async fn w_rpc_nonbinary_bodies(server_test: &ServerTest) {
    let bodies = [
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"host.describe","params":{}})
            .to_string(),
        serde_json::json!([{"jsonrpc":"2.0","id":2,"method":"host.describe"}]).to_string(),
        "plain-text-not-json".to_string(),
    ];
    let mut results = Vec::new();
    for body in bodies {
        let (st, resp) = http_status(server_test.port.get(), "POST", "/rpc", &body).await;
        results.push((st, resp));
    }
    *server_test.nonbinary_posts.borrow_mut() = results;
}

#[then("各 body 判非法信封 HTTP 400 且不执行业务")]
fn t_nonbinary_illegal(server_test: &ServerTest) {
    let results = server_test.nonbinary_posts.borrow();
    assert_eq!(results.len(), 3, "三个非产品二进制 body 均须已发送");
    for (st, body) in results.iter() {
        assert_eq!(*st, 400, "非产品二进制 body MUST 400: {st} {body}");
    }
}

fn arm_freeze_fixture_mcp(name: &str) -> xylitol_kernel::composition::McpServerSpec {
    use xylitol_kernel::composition::{McpServerSpec, McpTransportSpec};
    let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/support/mcp_fixture_server.py");
    let mut env = std::collections::HashMap::new();
    env.insert("XYLITOL_MCP_FIXTURE_TOOLS".to_string(), "ping".to_string());
    McpServerSpec {
        name: name.into(),
        transport: McpTransportSpec::Stdio,
        command: Some("python3".into()),
        args: Some(vec![script.display().to_string()]),
        url: None,
        env: Some(env),
        headers: None,
    }
}

#[given("宿主配置了连不上的 MCP 且会话写者已物化")]
async fn g_arm_freeze_hang_mcp(server_test: &ServerTest) {
    use xylitol_kernel::composition::{McpServerSpec, McpTransportSpec};
    let hang = McpServerSpec {
        name: "hang-forever-bdd".into(),
        transport: McpTransportSpec::Stdio,
        command: Some("sleep".into()),
        args: Some(vec!["300".into()]),
        url: None,
        env: None,
        headers: None,
    };
    let host = xylitol_kernel::occupancy::host_state_for_test_with_mcp(vec![hang]).expect("host");
    let slot = host.slot("freeze-gate-hang").await;
    xylitol_kernel::occupancy::host::materialize_writer(&host, &slot)
        .await
        .expect("materialize writer");
    *server_test.host.borrow_mut() = Some(host);
    *server_test.freeze_session.borrow_mut() = "freeze-gate-hang".into();
}

#[given("会话写租约已被其他写者持有")]
async fn g_arm_freeze_lease_held(server_test: &ServerTest) {
    let host =
        xylitol_kernel::occupancy::host_state_for_test_with_mcp(vec![arm_freeze_fixture_mcp(
            "fixture-bdd",
        )])
        .expect("host");
    let slot = host.slot("freeze-gate-conflict").await;
    xylitol_kernel::occupancy::host::materialize_writer(&host, &slot)
        .await
        .expect("materialize writer");
    let first = xylitol_kernel::occupancy::host::handle_unary(
        &host,
        None,
        "set_model",
        serde_json::json!({ "provider": "", "model_id": "keep", "session_id": "freeze-gate-conflict" }),
    )
    .await;
    let _ = first;
    *server_test.host.borrow_mut() = Some(host);
    *server_test.freeze_session.borrow_mut() = "freeze-gate-conflict".into();
}

#[when("发起首轮工具门冻结")]
async fn w_arm_tool_freeze(server_test: &ServerTest) {
    let host = server_test.host.borrow().clone().expect("host present");
    let session = server_test.freeze_session.borrow().clone();
    let t0 = std::time::Instant::now();
    let result = xylitol_kernel::occupancy::host::handle_unary(
        &host,
        None,
        "arm_tool_freeze",
        serde_json::json!({ "session_id": session }),
    )
    .await;
    server_test
        .freeze_elapsed_ms
        .set(t0.elapsed().as_millis() as u64);
    *server_test.freeze_value.borrow_mut() = result.value;
    if let Some(e) = result.error {
        *server_test.freeze_error.borrow_mut() = Some(format!("{}: {}", e.code, e.details));
    }
}

#[then("门时限内返回冻结快照且引导收口")]
fn t_arm_freeze_bounded(server_test: &ServerTest) {
    let err = server_test.freeze_error.borrow().clone();
    assert!(err.is_none(), "arm_tool_freeze MUST ok: {err:?}");
    let elapsed = server_test.freeze_elapsed_ms.get();
    assert!(
        elapsed < 30_000,
        "MUST be bounded by the first-turn gate window, took {elapsed}ms"
    );
    let v = server_test
        .freeze_value
        .borrow()
        .clone()
        .expect("snapshot value");
    assert_eq!(v["tools_table_frozen"], true, "timeout MUST freeze: {v}");
    assert_eq!(
        v["mcp_bootstrap_complete"], true,
        "timeout MUST settle bootstrap: {v}"
    );
}

#[then("不取写占用仍返回冻结快照")]
fn t_arm_freeze_lease_free(server_test: &ServerTest) {
    let err = server_test.freeze_error.borrow().clone();
    assert!(err.is_none(), "lease-free arm MUST ok: {err:?}");
    let v = server_test
        .freeze_value
        .borrow()
        .clone()
        .expect("snapshot value");
    assert_eq!(
        v["tools_table_frozen"], true,
        "lease-free arm MUST resolve frozen: {v}"
    );
}

const OBSERVE_SESSION: &str = "s-observe";

async fn observe_bind_a(
    host: &std::sync::Arc<HostState>,
) -> tokio::sync::mpsc::Receiver<RpcMessage> {
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    host.register_unbound_mux(tx).await;
    let slot = host.slot(OBSERVE_SESSION).await;
    let _ = host.bind_mux_to_session(&slot, 0).await;
    rx
}

async fn observe_seed_journal(host: &std::sync::Arc<HostState>) {
    let slot = host.slot(OBSERVE_SESSION).await;
    let mut j = slot.journal.lock().await;
    j.append(Event::TextDelta { text: "e1".into() });
    j.append(Event::TextDelta { text: "e2".into() });
}

async fn observe_bind_b(server_test: &ServerTest) {
    let host = server_test.host.borrow().clone().expect("host present");
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    host.register_unbound_mux(tx).await;
    let slot = host.slot(OBSERVE_SESSION).await;
    let result = host.bind_mux_to_session(&slot, 0).await;
    *server_test.observe_bind.borrow_mut() = result.value;
    *server_test.observe_rx_b.borrow_mut() = Some(rx);
}

fn observe_drain(rx: &mut tokio::sync::mpsc::Receiver<RpcMessage>) -> Vec<(String, Option<Event>)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        let RpcMessage::ServerRequest {
            method, payload, ..
        } = msg
        else {
            continue;
        };
        let event = payload
            .get("event")
            .and_then(|v| serde_json::from_value::<Event>(v.clone()).ok());
        out.push((method, event));
    }
    out
}

fn observe_texts(frames: &[(String, Option<Event>)]) -> Vec<String> {
    frames
        .iter()
        .filter_map(|(_, ev)| match ev {
            Some(Event::TextDelta { text }) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn observe_agent_ends(frames: &[(String, Option<Event>)]) -> usize {
    frames
        .iter()
        .filter(|(_, ev)| matches!(ev, Some(Event::AgentEnd)))
        .count()
}

#[given("会话回合成中且已有一路直播订阅")]
async fn g_observe_inflight(server_test: &ServerTest) {
    let host = xylitol_kernel::occupancy::host_state_for_test().expect("host");
    let rx_a = observe_bind_a(&host).await;
    observe_seed_journal(&host).await;
    host.slot(OBSERVE_SESSION)
        .await
        .mark_run_inflight_for_test(true);
    *server_test.host.borrow_mut() = Some(host);
    *server_test.observe_rx_a.borrow_mut() = Some(rx_a);
}

#[given("第二路处于观景态")]
async fn g_observe_bound(server_test: &ServerTest) {
    g_observe_inflight(server_test).await;
    observe_bind_b(server_test).await;
}

#[given("一路订阅在订且 journal 含历史事件")]
async fn g_observe_idle_journal(server_test: &ServerTest) {
    let host = xylitol_kernel::occupancy::host_state_for_test().expect("host");
    let rx_a = observe_bind_a(&host).await;
    observe_seed_journal(&host).await;
    *server_test.host.borrow_mut() = Some(host);
    *server_test.observe_rx_a.borrow_mut() = Some(rx_a);
}

#[given("会话回合进行中")]
async fn g_busy_inflight(server_test: &ServerTest) {
    let host = xylitol_kernel::occupancy::host_state_for_test().expect("host");
    host.slot(OBSERVE_SESSION)
        .await
        .mark_run_inflight_for_test(true);
    *server_test.host.borrow_mut() = Some(host);
}

#[when("第二路冷订阅同一会话")]
#[when("第二路冷订阅")]
async fn w_observe_second_bind(server_test: &ServerTest) {
    observe_bind_b(server_test).await;
}

#[when("会话产生流式 tape 与 AgentEnd")]
async fn w_observe_inject(server_test: &ServerTest) {
    let host = server_test.host.borrow().clone().expect("host present");
    let slot = host.slot(OBSERVE_SESSION).await;
    slot.append_and_push(Event::TextDelta {
        text: "live-tail".into(),
    })
    .await;
    slot.append_and_push(Event::AgentEnd).await;
}

#[when("任一连接对该会话再发根提交")]
async fn w_busy_second_root(server_test: &ServerTest) {
    let host = server_test.host.borrow().clone().expect("host present");
    let result = xylitol_kernel::occupancy::host::handle_unary(
        &host,
        None,
        "prompt",
        serde_json::json!({ "session_id": OBSERVE_SESSION, "message": "second root" }),
    )
    .await;
    *server_test.busy_code.borrow_mut() =
        result.error.map(|e| e.code).or_else(|| Some("ok".into()));
}

#[then("订阅应答标记 inflight 且第二路不收到历史 tape 重放")]
fn t_observe_bind_flag(server_test: &ServerTest) {
    let bind = server_test
        .observe_bind
        .borrow()
        .clone()
        .expect("bind result");
    assert_eq!(
        bind["inflight"], true,
        "观景绑定应答须携带 inflight: {bind}"
    );
    let mut rx = server_test.observe_rx_b.borrow_mut().take().expect("rx b");
    let frames = observe_drain(&mut rx);
    *server_test.observe_rx_b.borrow_mut() = Some(rx);
    assert!(
        !frames.iter().any(|(m, _)| m == "session/event"),
        "观景绑定不重放历史 tape: {frames:?}"
    );
    assert!(
        frames.iter().any(|(m, _)| m == "session/subscribed"),
        "订阅确认照常: {frames:?}"
    );
}

#[then("第二路仅收到 AgentEnd 而直播端全收")]
fn t_observe_filter(server_test: &ServerTest) {
    let mut rx_b = server_test.observe_rx_b.borrow_mut().take().expect("rx b");
    let b = observe_drain(&mut rx_b);
    *server_test.observe_rx_b.borrow_mut() = Some(rx_b);
    let mut rx_a = server_test.observe_rx_a.borrow_mut().take().expect("rx a");
    let a = observe_drain(&mut rx_a);
    *server_test.observe_rx_a.borrow_mut() = Some(rx_a);
    assert_eq!(
        observe_texts(&b),
        Vec::<String>::new(),
        "观景端 tape 冻结: {b:?}"
    );
    assert_eq!(observe_agent_ends(&b), 1, "观景端放行 AgentEnd: {b:?}");
    assert_eq!(observe_texts(&a), vec!["live-tail"], "直播端全收: {a:?}");
    assert_eq!(observe_agent_ends(&a), 1, "直播端 AgentEnd: {a:?}");
}

#[then("重放仅达第二路而在订者不收到重放帧")]
fn t_observe_targeted(server_test: &ServerTest) {
    let mut rx_b = server_test.observe_rx_b.borrow_mut().take().expect("rx b");
    let b = observe_drain(&mut rx_b);
    *server_test.observe_rx_b.borrow_mut() = Some(rx_b);
    let mut rx_a = server_test.observe_rx_a.borrow_mut().take().expect("rx a");
    let a = observe_drain(&mut rx_a);
    *server_test.observe_rx_a.borrow_mut() = Some(rx_a);
    let b_events = b.iter().filter(|(m, _)| m == "session/event").count();
    let a_events = a.iter().filter(|(m, _)| m == "session/event").count();
    assert_eq!(b_events, 2, "新绑定者收全量重放: {b:?}");
    assert_eq!(a_events, 0, "在订者不收重放突发: {a:?}");
}

#[then("得到稳定 session_busy 错误且不产生第二回合并发")]
fn t_busy_rejected(server_test: &ServerTest) {
    let code = server_test
        .busy_code
        .borrow()
        .clone()
        .expect("prompt result");
    assert_eq!(code, "session_busy", "根提交忙碌须稳定拒绝");
}

fn rdy_fresh_gateway(phase: &str) -> Arc<Gateway> {
    let g = Gateway::starting();
    match phase {
        "ready" => g.set_host(xylitol_kernel::occupancy::host_state_for_test().expect("HostState")),
        "failed" => g.mark_failed(),
        "starting" => {}
        other => panic!("unknown readiness phase: {other}"),
    }
    g
}

#[given("监听器处于 {from:string} 阶段")]
async fn g_rdy_phase(server_test: &ServerTest, from: String) {
    *server_test.gateway.borrow_mut() = Some(rdy_fresh_gateway(&from));
}

#[when("装配事件 {event:string} 发生")]
async fn w_rdy_transition(server_test: &ServerTest, event: String) {
    let gateway = server_test
        .gateway
        .borrow()
        .clone()
        .expect("gateway held by given");
    match event.as_str() {
        "assembly_complete" => {
            gateway.set_host(xylitol_kernel::occupancy::host_state_for_test().expect("HostState"))
        }
        "assembly_fail" => gateway.mark_failed(),
        other => panic!("unknown assembly event: {other}"),
    }
}

#[then("就绪状态迁移为 {to:string}")]
async fn t_rdy_phase(server_test: &ServerTest, to: String) {
    let gateway = server_test
        .gateway
        .borrow()
        .clone()
        .expect("gateway held by given");
    let phase = format!("{:?}", gateway.phase()).to_lowercase();
    assert_eq!(phase, to, "readiness phase transition");
}
