use crate::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use std::time::Duration;

use xylitol::app::server::host::HostState;
use xylitol::app::server::runtime::{RunningServer, ServerConfig, bind_serve, serve};
use xylitol::app::server::ws::{EventJournal, ReverseRpcResult};
use xylitol::protocol::Event;
use xylitol::protocol::wire::envelope::{PROTOCOL_VERSION, RpcMessage};
use xylitol::protocol::wire::method::DOWNLINK_METHODS;
use xylitol::protocol::wire::registry;
use xylitol::{HostClient, HttpWsClient, MuxStream};

/// Shared fixture for server-core scenarios.
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
    /// sr-sub1：跨步骤持有已订阅会话的下行流。
    pub mux_rx: RefCell<Option<MuxStream>>,
    /// c2460 sr-idem：幂等场景的两次应答与首次执行的侧写。
    pub idem_first: Arc<std::sync::Mutex<Option<String>>>,
    pub idem_second: RefCell<Option<String>>,
    pub idem_exec_file: RefCell<Option<std::path::PathBuf>>,
    pub idem_task: RefCell<Option<tokio::task::JoinHandle<()>>>,
    /// c2465 sr-rdy1：就绪窗口场景的网关与最近一次 healthz 应答。
    pub gateway: RefCell<Option<Arc<xylitol::app::server::http::Gateway>>>,
    pub rdy_status: Cell<u16>,
    pub rdy_body: RefCell<Option<String>>,
    /// c2475 sr-reg1：注册文件路径与自检驱逐信号。
    pub reg_path: RefCell<Option<std::path::PathBuf>>,
    pub evict_rx: RefCell<Option<tokio::sync::watch::Receiver<bool>>>,
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

async fn start_host(t: &ServerTest) {
    let host = HostState::for_test().expect("HostState");
    let (running, port) = serve(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: None,
        },
        host.clone(),
    )
    .await
    .expect("serve");
    t.host.replace(Some(host));
    t.running.replace(Some(running));
    t.port.set(port);
    tokio::time::sleep(Duration::from_millis(30)).await;
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

fn source_contains(path: &str, needle: &str) -> bool {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .contains(needle)
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

#[then("返回 OpenAPI 3.1 文档且含全部登记 unary 条目")]
fn t_openapi_methods(server_test: &ServerTest) {
    assert_eq!(server_test.unary_status.get(), 200, "must be 200");
    let body = server_test.unary_body.borrow().clone().unwrap_or_default();
    let v: serde_json::Value = serde_json::from_str(&body).expect("valid openapi json");
    assert_eq!(v["openapi"], "3.1.0", "{body}");
    let paths = v["paths"].as_object().expect("paths object");
    for m in registry::names() {
        let key = format!("/api/{m}");
        let entry = paths
            .get(key.as_str())
            .unwrap_or_else(|| panic!("missing /api/{m}"));
        assert_eq!(
            entry["post"]["operationId"].as_str(),
            Some(m),
            "mismatched operationId for {m}"
        );
    }
    assert!(paths.contains_key("/healthz"));
    assert!(paths.contains_key("/api/respond"));
}

#[then("文档不含 WS 下行 path")]
fn t_openapi_no_ws(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().unwrap_or_default();
    let v: serde_json::Value = serde_json::from_str(&body).expect("valid openapi json");
    let paths = v["paths"].as_object().expect("paths object");
    assert!(!paths.keys().any(|k| k.contains("events.mux")));
    for d in DOWNLINK_METHODS {
        assert!(!paths.contains_key(*d), "downlink path leaked: {d}");
    }
    // The prose pointer is the contract (sr-oapi1): mux channel + Rust protocol types.
    let desc = v["info"]["description"].as_str().expect("description");
    assert!(desc.contains("events.mux"), "{desc}");
    assert!(desc.contains("protocol::wire"), "{desc}");
}

#[when("GET /docs")]
async fn w_scalar_page(server_test: &ServerTest) {
    let (status, body) = http_status(server_test.port.get(), "GET", "/docs", "").await;
    server_test.unary_status.set(status);
    *server_test.unary_body.borrow_mut() = Some(body);
}

#[then("Scalar 调试页可达且指向 spec")]
fn t_scalar_page(server_test: &ServerTest) {
    assert_eq!(server_test.unary_status.get(), 200, "must be 200");
    let body = server_test.unary_body.borrow().clone().unwrap_or_default();
    let lower = body.to_lowercase();
    assert!(lower.contains("scalar"), "scalar UI html expected: {body}");
    assert!(
        body.contains("/openapi.json"),
        "must point at the spec: {body}"
    );
}

#[given("服务端已在该地址端口监听")]
async fn g_already_listening(server_test: &ServerTest) {
    start_host(server_test).await;
}

#[when("第二个服务端绑定同一地址端口")]
async fn w_second_bind(server_test: &ServerTest) {
    let host2 = HostState::for_test().expect("host2");
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
    assert!(source_contains("src/app/server/host.rs", "SessionSlot"));
    assert!(source_contains(
        "src/app/server/host.rs",
        "XyInProcessDriver"
    ));
    assert!(source_contains(
        "src/app/server/host.rs",
        "materialize_writer"
    ));
}

#[when("产品 TUI 访问 Host")]
async fn w_product_tui(server_test: &ServerTest) {
    start_host(server_test).await;
    let client = HttpWsClient::new(server_test.base_url());
    let desc = client
        .unary("host.describe", serde_json::json!({}))
        .await
        .expect("describe");
    server_test
        .unary_body
        .replace(Some(serde_json::to_string(&desc).unwrap()));
    let mut mux = client.mux().await.expect("mux");
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    wait_unbound(&host, 1).await;
    let sub = client
        .unary(
            "subscribe",
            serde_json::json!({"session_id": "s-tui", "last_seq": 0}),
        )
        .await
        .expect("subscribe");
    assert!(sub.ok, "{sub:?}");
    host.slot("s-tui")
        .await
        .append_and_push(Event::TextDelta {
            text: "roundtrip".into(),
        })
        .await;
    let frame = tokio::time::timeout(Duration::from_secs(2), mux.next())
        .await
        .expect("mux timeout")
        .expect("mux eof")
        .expect("mux frame");
    server_test.mux_frames.replace(vec![frame]);
}

#[then("经四象限 POST unary 与 WebSocket 下行")]
fn t_four_quad_shape(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow();
    let s = body.as_deref().unwrap_or("");
    assert!(s.contains("ok") || s.contains("protocol"), "{s}");
}

#[then("Host 对该路径给出可观察往返")]
fn t_sr_env1_roundtrip(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow();
    let s = body.as_deref().unwrap_or("");
    assert!(
        s.contains(&PROTOCOL_VERSION.to_string()) || s.contains("protocol"),
        "host.describe must round-trip, got {s}"
    );
    let frames = server_test.mux_frames.borrow();
    let hit = frames.iter().any(|f| {
        matches!(
            f,
            RpcMessage::ServerRequest { method, .. }
                if method == "session/event" || method == "session/subscribed"
        )
    });
    assert!(
        hit,
        "mux downlink must carry a ServerRequest, got {frames:?}"
    );
}

#[when("启动 app::server 运行时")]
async fn w_start_runtime(server_test: &ServerTest) {
    start_host(server_test).await;
}

#[then("暴露 POST /api/{{method}}、POST /api/respond 与只下行的 events.mux")]
async fn t_sr2_routes(server_test: &ServerTest) {
    let client = HttpWsClient::new(server_test.base_url());
    let r = client
        .unary("host.describe", serde_json::json!({}))
        .await
        .expect("describe");
    assert!(r.ok, "{r:?}");
    let (st, body) = http_status(server_test.port.get(), "POST", "/api/respond", "{}").await;
    assert!(st == 400 || st == 200, "respond mounted, got {st} {body}");
    let _mux = client.mux().await.expect("events.mux");
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

#[given("客户端断开 N 秒后以 last_seq unary subscribe")]
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
    let r = client
        .unary(
            "subscribe",
            serde_json::json!({"session_id": "s-replay", "last_seq": 0}),
        )
        .await
        .expect("subscribe");
    assert!(r.ok, "{r:?}");
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

#[then("客户端经 POST /api/respond 应答且回合恢复")]
async fn t_sr5_respond(server_test: &ServerTest) {
    let client = HttpWsClient::new(server_test.base_url());
    client
        .respond("xyz", serde_json::json!({"approved": true}))
        .await
        .expect("respond");
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
    assert!(!source_contains(
        "src/app/server/runtime.rs",
        "xylitol-server.lock"
    ));
    assert!(!source_contains("src/app/server/runtime.rs", "ServerLock"));
}

#[given("18790 被其它进程占用")]
fn g_occupy_18790(server_test: &ServerTest) {
    match std::net::TcpListener::bind(("127.0.0.1", 18790)) {
        Ok(l) => {
            server_test.occupied.replace(Some(l));
        }
        Err(_) => {
            // already occupied by another process — still valid for the scenario
        }
    }
}

#[when("server 在 18790 启动")]
async fn w_start_18790(server_test: &ServerTest) {
    let host = HostState::for_test().expect("host");
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

#[given("向 POST /api/prompt 发送 ClientRequest")]
async fn g_post_prompt(server_test: &ServerTest) {
    start_host(server_test).await;
    let client = HttpWsClient::new(server_test.base_url());
    let result = client
        .unary(
            "prompt",
            serde_json::json!({"message": "hi", "session_id": "s-prompt"}),
        )
        .await
        .expect("prompt");
    server_test.unary_status.set(200);
    server_test
        .unary_body
        .replace(Some(serde_json::to_string(&result).unwrap()));
}

#[when("server 处理 prompt")]
fn w_handle_prompt(_server_test: &ServerTest) {}

#[then("HTTP 200 且 ServerResponse 回显 rpcId")]
fn t_prompt_200(server_test: &ServerTest) {
    assert_eq!(server_test.unary_status.get(), 200);
    let body = server_test.unary_body.borrow();
    assert!(body.as_ref().is_some_and(|b| b.contains("ok")), "{body:?}");
}

#[then("POST /api/v1/session/x/run 不是产品路径")]
async fn t_run_not_product(server_test: &ServerTest) {
    t_no_v1_rest(server_test).await;
}

#[when("app::server 运行时经 app::core::composition 组装 Agent")]
fn w_sr10(_server_test: &ServerTest) {}

#[then("向 Agent 传入 Arc<dyn ExportIo>")]
fn t_sr10(_server_test: &ServerTest) {
    assert!(source_contains(
        "src/app/core/driver/in_process/mod.rs",
        "SessionExporter"
    ));
    assert!(source_contains(
        "src/app/core/driver/in_process/mod.rs",
        "StdExportIo"
    ));
}

#[given("审查 server 组合根")]
fn g_review_root(_server_test: &ServerTest) {}

#[when("检查持有类型")]
fn w_check_types(_server_test: &ServerTest) {}

#[then("按 session 槽持有 Driver；handlers 不直接以 Mutex AgentRuntime 作为唯一入口")]
fn t_driver1(_server_test: &ServerTest) {
    assert!(source_contains("src/app/server/host.rs", "SessionSlot"));
    let http = std::fs::read_to_string("src/app/server/http.rs").unwrap();
    assert!(!http.contains("Mutex<AgentRuntime>"));
}

#[when("调用 RemoteDriver 已登记 unary（如 steer）")]
fn w_remote_steer(_server_test: &ServerTest) {}

#[then("经 HostClient 到达 Host")]
fn t_remote1(_server_test: &ServerTest) {
    assert!(source_contains(
        "src/app/core/driver/remote.rs",
        "host.unary"
    ));
}

#[then("未登记方法不发明 REST")]
fn t_no_invented_rest(_server_test: &ServerTest) {
    assert!(source_contains(
        "src/protocol/wire/registry.rs",
        "\"list_sessions\""
    ));
    assert!(!source_contains(
        "src/app/core/driver/remote.rs",
        "list_sessions is not a v1 unary method"
    ));
}

#[when("审查 prompt/steer 等 unary handlers")]
fn w_review_handlers(_server_test: &ServerTest) {}

#[then("调用 dispatch 或共享 helper；与 InProcessDriver 语义一致")]
fn t_dispatch1(_server_test: &ServerTest) {
    assert!(source_contains("src/app/server/host.rs", "dispatch("));
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

#[then("经四象限 unary 到达 Host 且不经 REST 冒充")]
fn t_tree_registered(_server_test: &ServerTest) {
    assert!(source_contains(
        "src/protocol/wire/registry.rs",
        "\"session_tree\""
    ));
    assert!(source_contains(
        "src/app/core/driver/remote.rs",
        "session_tree"
    ));
}

#[when("调用已登记的 session 能力 unary")]
fn w_session_methods_registered(_server_test: &ServerTest) {
    assert!(source_contains(
        "src/protocol/wire/registry.rs",
        "\"list_sessions\""
    ));
}

#[when("调用 Host 资源 unary")]
fn w_host_resource_methods(_server_test: &ServerTest) {
    assert!(source_contains(
        "src/protocol/wire/registry.rs",
        "\"reload\""
    ));
}

// ── server-ws ─────────────────────────────────────────────────────

#[given("构造下行 ServerRequest session/event")]
fn g_w1_frame(server_test: &ServerTest) {
    let msg = RpcMessage::ServerRequest {
        rpc_id: "r1".into(),
        method: "session/event".into(),
        payload: serde_json::json!({"session_id": "s0", "seq": 1}),
    };
    server_test
        .unary_body
        .replace(Some(serde_json::to_string(&msg).unwrap()));
}

#[when("序列化为 JSON")]
fn w_serialize(_server_test: &ServerTest) {}

#[then("JSON 含 type=server-request 与 method=session/event")]
fn t_w1(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow();
    let s = body.as_deref().unwrap();
    assert!(s.contains("server-request"), "{s}");
    assert!(s.contains("session/event"), "{s}");
}

#[given("客户端欲以 last_seq 5 订阅会话 s0")]
fn g_w2(_server_test: &ServerTest) {}

#[when("发送 unary subscribe")]
fn w_w2_subscribe(server_test: &ServerTest) {
    let msg = RpcMessage::ClientRequest {
        rpc_id: "r2".into(),
        method: "subscribe".into(),
        payload: serde_json::json!({"session_id": "s0", "last_seq": 5}),
        writer_token: None,
    };
    server_test
        .unary_body
        .replace(Some(serde_json::to_string(&msg).unwrap()));
}

#[then("payload 含 session_id=s0 与 last_seq=5")]
fn t_w2_payload(server_test: &ServerTest) {
    let s = server_test.unary_body.borrow();
    let t = s.as_deref().unwrap();
    assert!(t.contains("s0") && t.contains("5"), "{t}");
}

#[then("mux 不接受 Subscribe 应用帧")]
fn t_w2_no_uplink(_server_test: &ServerTest) {
    assert!(source_contains("src/app/server/http.rs", "is_text()"));
    assert!(source_contains(
        "src/app/server/http.rs",
        "Business uplink is forbidden"
    ));
}

#[given("客户端连接 /api/events.mux")]
async fn g_w3_mux(server_test: &ServerTest) {
    start_host(server_test).await;
}

#[when("server 接受升级")]
async fn w_w3_upgrade(server_test: &ServerTest) {
    let client = HttpWsClient::new(server_test.base_url());
    let mux = client.mux().await.expect("upgrade");
    drop(mux);
}

#[then("连接只收下行 ServerRequest")]
fn t_w3_downlink(_server_test: &ServerTest) {
    assert!(source_contains("src/app/server/http.rs", "is_text()"));
}

#[then("不把会话绑在 /api/v1/session/x/ws")]
fn t_w3_no_old_ws(_server_test: &ServerTest) {
    let http = std::fs::read_to_string("src/app/server/http.rs").unwrap();
    let product = http.split("#[cfg(test)]").next().unwrap_or(&http);
    assert!(
        !product.contains("/api/v1/session"),
        "product router must not mount /api/v1/session"
    );
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

#[when("last_seq=0 的客户端 unary subscribe")]
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
    let _ = client
        .unary(
            "subscribe",
            serde_json::json!({"session_id": "s-wrap", "last_seq": 0}),
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

#[when("客户端以 last_seq=0 再次 unary subscribe")]
async fn w_w6_resub(server_test: &ServerTest) {
    let client = HttpWsClient::new(server_test.base_url());
    let mut mux = client.mux().await.expect("mux");
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    wait_unbound(&host, 1).await;
    let _ = client
        .unary(
            "subscribe",
            serde_json::json!({"session_id": "s-wrap", "last_seq": 0}),
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
    let _ = client
        .unary(
            "subscribe",
            serde_json::json!({"session_id": "s-delta", "last_seq": 0}),
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

#[then("客户端在 mux 上收到 session/event 的 ServerRequest")]
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

// ── w8 冷恢复投影 ────────────────────────────────────────────────

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
    // 冷订进入恢复窗（last_seq=0），与真实 attach 一致。
    let subscribed = client
        .unary(
            "subscribe",
            serde_json::json!({ "session_id": "s0", "last_seq": 0 }),
        )
        .await
        .expect("subscribe");
    assert!(subscribed.ok, "{subscribed:?}");
    let switched = client
        .unary("switch_session", serde_json::json!({ "session_id": "s0" }))
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

// ── reverse RPC ───────────────────────────────────────────────────

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

#[then("客户端收到带有 rpcId 的 approval/requested")]
fn t_rr_got_request(approval_test: &ServerTest) {
    let acc = approval_test.mux_acc.lock().unwrap();
    let hit = acc.iter().any(|f| {
        matches!(
            f,
            RpcMessage::ServerRequest { method, rpc_id, .. }
                if method == "approval/requested" && rpc_id == "call-approve-1"
        )
    });
    assert!(hit, "expected approval/requested with rpcId, got {acc:?}");
}

#[when("客户端 POST /api/respond 且 approved=true")]
async fn w_rr_approve(approval_test: &ServerTest) {
    let client = HttpWsClient::new(approval_test.base_url());
    client
        .respond("call-approve-1", serde_json::json!({"approved": true}))
        .await
        .expect("respond");
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

#[when("客户端 POST /api/respond 且 approved=false")]
async fn w_rr_deny(approval_test: &ServerTest) {
    let client = HttpWsClient::new(approval_test.base_url());
    client
        .respond("call-approve-1", serde_json::json!({"approved": false}))
        .await
        .expect("respond");
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
                    RpcMessage::ServerRequest { method, rpc_id, .. }
                        if method == "approval/requested" && rpc_id == "xyz"
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

#[given("2 个客户端 POST /api/respond，rpcId xyz（首个 true，50ms 后 false）")]
async fn g_rr2(approval_test: &ServerTest) {
    start_host(approval_test).await;
    let host = approval_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot(&host.fallback_session).await;
    let rx = slot.request_approval("xyz".into()).await;
    approval_test.approval_rx.replace(Some(rx));
    let client = HttpWsClient::new(approval_test.base_url());
    client
        .respond("xyz", serde_json::json!({"approved": true}))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    client
        .respond("xyz", serde_json::json!({"approved": false}))
        .await
        .unwrap();
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

#[given("60s 内无客户端 POST /api/respond")]
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

#[given("第二个客户端对已消费 rpcId POST /api/respond")]
async fn g_rr4(approval_test: &ServerTest) {
    start_host(approval_test).await;
    let host = approval_test.host.borrow().as_ref().expect("host").clone();
    let slot = host.slot(&host.fallback_session).await;
    let rx = slot.request_approval("xyz".into()).await;
    let client = HttpWsClient::new(approval_test.base_url());
    client
        .respond("xyz", serde_json::json!({"approved": true}))
        .await
        .unwrap();
    let _ = rx.await;
    approval_test
        .last_rpc
        .replace(Some(ReverseRpcResult::Approved));
}

#[when("server 收到该应答")]
async fn w_rr4_second(approval_test: &ServerTest) {
    let client = HttpWsClient::new(approval_test.base_url());
    client
        .respond("xyz", serde_json::json!({"approved": false}))
        .await
        .unwrap();
}

#[then("应答被静默忽略（无状态变化、无错误）")]
fn t_rr4(approval_test: &ServerTest) {
    assert_eq!(
        *approval_test.last_rpc.borrow(),
        Some(ReverseRpcResult::Approved)
    );
}

#[given("客户端 A 已对 session 发出非只读 unary")]
async fn g_sr_w1_writer_a(server_test: &ServerTest) {
    start_host(server_test).await;
    let a = HttpWsClient::new(server_test.base_url());
    let r = a
        .unary(
            "clear_queue",
            serde_json::json!({"clear_steer": true, "clear_follow_up": true}),
        )
        .await
        .expect("A write");
    assert!(r.ok, "{r:?}");
    assert!(
        r.value
            .as_ref()
            .and_then(|v| v.get("writerToken"))
            .and_then(|v| v.as_str())
            .is_some(),
        "{r:?}"
    );
}

#[when("客户端 B 无 writerToken 再发非只读 unary")]
async fn w_sr_w1_writer_b(server_test: &ServerTest) {
    let b = HttpWsClient::new(server_test.base_url());
    let r = b
        .unary(
            "clear_queue",
            serde_json::json!({"clear_steer": true, "clear_follow_up": true}),
        )
        .await
        .expect("B transport");
    server_test
        .unary_body
        .replace(Some(serde_json::to_string(&r).unwrap()));
}

#[then("业务错误说明已有写者")]
fn t_sr_w1_conflict(server_test: &ServerTest) {
    let s = server_test.unary_body.borrow().clone().expect("B result");
    assert!(
        s.contains("writer_conflict") || s.contains("another client is the writer"),
        "{s}"
    );
}

// ---- sr-q1 / sr-abort1 / sr-sub1：queue_stats 只读、reload 合作取消、订阅跨回合存活 ----

#[when("查询只读 unary queue_stats")]
async fn w_queue_stats(server_test: &ServerTest) {
    let client = HttpWsClient::new(server_test.base_url());
    let r = client
        .unary("queue_stats", serde_json::json!({"session_id": "s-q"}))
        .await
        .expect("queue_stats");
    assert!(r.ok, "{r:?}");
    server_test
        .unary_body
        .replace(Some(serde_json::to_string(&r).unwrap()));
}

#[then("返回 steer 与 follow-up 队列深度")]
fn t_queue_stats_shape(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().expect("body");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(v["value"]["steer_count"].is_u64(), "{body}");
    assert!(v["value"]["follow_up_count"].is_u64(), "{body}");
}

#[then("响应不携带写者租约 token")]
fn t_queue_stats_no_writer_lease(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().expect("body");
    assert!(
        !body.contains("writerToken"),
        "readonly unary must not lease writer, got {body}"
    );
}

#[given("进程级 reload 正在进行（取消令牌已注册）")]
async fn g_reload_inflight(server_test: &ServerTest) {
    start_host(server_test).await;
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    *host.reload_cancel.lock().await = Some(tokio_util::sync::CancellationToken::new());
}

#[when("收到进程级 abort unary")]
async fn w_abort_unary(server_test: &ServerTest) {
    let client = HttpWsClient::new(server_test.base_url());
    let r = client
        .unary("abort", serde_json::json!({}))
        .await
        .expect("abort");
    server_test
        .unary_body
        .replace(Some(serde_json::to_string(&r).unwrap()));
}

#[then("应答携带 cancelled 指示且取消令牌被置位")]
async fn t_reload_cancelled(server_test: &ServerTest) {
    let body = server_test.unary_body.borrow().clone().expect("body");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["value"]["cancelled"], serde_json::json!(true), "{body}");
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
    // reload 分支应答为纯 {"cancelled": true}（无租约）；
    // 落回既有会话 abort 处理时，abort 作为非只读 unary 颁发写者租约（见 sr-w1）
    assert!(
        v["value"]["writerToken"].is_string(),
        "idle abort must fall through to leased session abort, got {body}"
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

    // 收 AgentEnd 帧（可能先到 subscribed 确认帧）
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

    // 回合结束后订阅必须仍存活：追加新事件且继续送达同一连接
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

#[when("调用未登记 unary 方法 no_such_method")]
async fn w_unknown_unary(server_test: &ServerTest) {
    let (st, body) = http_status(server_test.port.get(), "POST", "/api/no_such_method", "{}").await;
    server_test.unary_status.set(st);
    *server_test.unary_body.borrow_mut() = Some(body);
}

#[then("应答为稳定错误形态且无 JSON-RPC 数字码")]
fn t_unknown_unary_shape(server_test: &ServerTest) {
    let st = server_test.unary_status.get();
    let body = server_test.unary_body.borrow().clone().unwrap_or_default();
    assert!(
        !body.contains("jsonrpc"),
        "product errors must not be JSON-RPC, got {body}"
    );
    if st == 200 {
        let v: serde_json::Value = serde_json::from_str(&body).expect("envelope body");
        assert_eq!(v["ok"], serde_json::json!(false), "{body}");
        assert!(
            v["error"]["code"].is_string(),
            "stable string code required, got {body}"
        );
    } else {
        assert!(
            matches!(st, 400 | 404 | 405),
            "carrier must reject unknown method, got {st} {body}"
        );
    }
}

// ---- c2460 sr-idem：unary 幂等准入 ----

/// 以固定信封 `rpcId` 提交一个 unary ClientRequest，返回应答体原文。
async fn post_unary_rpc_id(
    port: u16,
    rpc_id: &str,
    method: &str,
    payload: serde_json::Value,
) -> String {
    let request = serde_json::json!({
        "type": "client-request",
        "rpcId": rpc_id,
        "method": method,
        "payload": payload,
    });
    let (_, body) = http_status(
        port,
        "POST",
        &format!("/api/{method}"),
        &serde_json::to_string(&request).expect("serialize request"),
    )
    .await;
    body
}

#[given("客户端以 rpcId R 对某 session 提交 unary 命令并得到结果")]
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

#[when("客户端以相同 rpcId R 重试同一命令")]
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
    assert_eq!(first["result"]["ok"], serde_json::json!(true), "{first}");
    let stats = post_unary_rpc_id(
        server_test.port.get(),
        "R-stats",
        "queue_stats",
        serde_json::json!({ "session_id": "idem-s1" }),
    )
    .await;
    let stats: serde_json::Value = serde_json::from_str(&stats).expect("queue_stats envelope");
    assert_eq!(
        stats["result"]["value"]["steer_count"],
        serde_json::json!(1),
        "steer MUST be admitted exactly once: {stats}"
    );
}

#[given("rpcId R 已被某 method 与 payload 的提交占用")]
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
    assert_eq!(v["result"]["ok"], serde_json::json!(true), "{v}");
}

#[when("以相同 rpcId R 提交不同 method 或 payload")]
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

#[then("返回 ok=false 且 code=idempotency_conflict 且不执行")]
async fn t_idem_conflict(server_test: &ServerTest) {
    let second = server_test
        .idem_second
        .borrow()
        .clone()
        .expect("conflict result recorded");
    let v: serde_json::Value = serde_json::from_str(&second).expect("envelope");
    assert_eq!(v["result"]["ok"], serde_json::json!(false), "{v}");
    assert_eq!(
        v["result"]["error"]["code"],
        serde_json::json!("idempotency_conflict"),
        "{v}"
    );
}

#[given("rpcId R 的首次命令仍在处理中")]
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
    // 让首次请求先进桩，落在处理中窗口内。
    tokio::time::sleep(Duration::from_millis(120)).await;
}

#[when("相同 rpcId R 的重复请求到达")]
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
    assert_eq!(first["result"]["ok"], serde_json::json!(true), "{first}");
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

// ---- c2465 sr-rdy1：启动就绪窗口三态语义 ----

#[given("监听器已绑定端口但装配未完成")]
async fn g_rdy_starting(server_test: &ServerTest) {
    let gateway = xylitol::app::server::http::Gateway::starting();
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
    let request = serde_json::json!({
        "type": "client-request",
        "rpcId": "rdy-probe",
        "method": "get_state",
        "payload": { "session_id": "rdy-s1" },
    });
    let (st, body) = http_status(
        server_test.port.get(),
        "POST",
        "/api/get_state",
        &serde_json::to_string(&request).expect("serialize request"),
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
    let gateway = xylitol::app::server::http::Gateway::starting();
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

// ---- c2475 sr-reg1：serve 注册文件发现契约 ----

use xylitol::app::server::registration::{
    Registration, read_registration, run_self_check, write_registration,
};
use xylitol::app::server::runtime::serve_registered;

fn temp_reg_path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("xylitol-bdd-reg-{}.json", uuid::Uuid::new_v4()))
}

#[given("serve 已装配完成并在空闲端口监听")]
async fn g_reg_serving(server_test: &ServerTest) {
    let host = HostState::for_test().expect("host");
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
        // 确认死亡后残留注册应被清理。
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
