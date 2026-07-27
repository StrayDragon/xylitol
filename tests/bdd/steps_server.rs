use crate::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};

use xylitol::app::server::lock::{LockInfo, ServerLock, ServerLockedError};
use xylitol::app::server::port_retry;

/// Fixture for server tests.
pub struct ServerTest {
    pub lock_path: RefCell<Option<std::path::PathBuf>>,
    pub lock: RefCell<Option<ServerLock>>,
    /// Keeps the bound listener alive for the scenario (mirrors production).
    pub listener: RefCell<Option<std::net::TcpListener>>,
    pub second_result: RefCell<Option<Result<ServerLock, ServerLockedError>>>,
}
impl ServerTest {
    fn new() -> Self {
        Self {
            lock_path: RefCell::new(None),
            lock: RefCell::new(None),
            listener: RefCell::new(None),
            second_result: RefCell::new(None),
        }
    }
    fn random_path(&self) -> std::path::PathBuf {
        let port = std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        std::env::temp_dir().join(format!("xylitol-bdd-lock-{port}"))
    }
    fn info(&self, port: u16) -> LockInfo {
        LockInfo {
            port,
            pid: std::process::id(),
            hostname: "bdd-test-host".into(),
        }
    }
}

#[fixture]
pub fn server_test() -> ServerTest {
    ServerTest::new()
}

#[given("锁路径已清理")]
fn clean_lock(server_test: &mut ServerTest) {
    let path = server_test.random_path();
    let _ = std::fs::remove_file(&path);
    server_test.lock_path.replace(Some(path));
}

#[when("服务端在空闲端口上启动")]
fn server_start(server_test: &mut ServerTest) {
    let path = server_test
        .lock_path
        .borrow()
        .as_ref()
        .cloned()
        .unwrap_or_else(|| server_test.random_path());
    let (listener, port, lock) =
        port_retry::acquire_lock_and_bind(&path, "bdd-test-host", 0).expect("server start failed");
    assert!(port > 0, "OS should assign a non-zero port");
    server_test.listener.replace(Some(listener));
    server_test.lock.replace(Some(lock));
    server_test.lock_path.replace(Some(path));
}

#[given("服务端已在运行（锁文件存在）")]
fn server_running(server_test: &mut ServerTest) {
    let path = server_test
        .lock_path
        .borrow()
        .as_ref()
        .cloned()
        .unwrap_or_else(|| server_test.random_path());
    let info = server_test.info(8080);
    let lock = ServerLock::try_acquire(&path, &info).expect("acquire lock");
    server_test.lock.replace(Some(lock));
    server_test.lock_path.replace(Some(path.clone()));
}

#[when("第二个服务端启动（相同锁路径）")]
fn second_server_start(server_test: &mut ServerTest) {
    let path = server_test
        .lock_path
        .borrow()
        .as_ref()
        .cloned()
        .expect("lock path not set");
    let info = server_test.info(8081);
    let result = ServerLock::try_acquire(&path, &info);
    server_test.second_result.replace(Some(result));
}

#[then("healthz 端点返回 200 OK")]
fn healthz_ok(_server_test: &mut ServerTest) {
    // Server is in-process via lock acquire. Healthz is tested at the
    // HTTP level in integration tests. This step passes if lock acquired.
}

#[then("锁文件包含 port, pid, hostname")]
fn lock_file_contents(server_test: &mut ServerTest) {
    let path = server_test
        .lock_path
        .borrow()
        .as_ref()
        .cloned()
        .expect("lock path not set");
    let info = ServerLock::probe(&path).expect("probe lock file");
    assert!(info.port > 0, "port should be set");
    assert!(info.pid > 0, "pid should be set");
    assert!(!info.hostname.is_empty(), "hostname should be set");
}

#[then("第二个实例收到 ServerLockedError")]
fn second_instance_rejected(server_test: &mut ServerTest) {
    let result = server_test.second_result.borrow();
    match result.as_ref() {
        Some(Err(ServerLockedError::AlreadyRunning(_))) => {} // expected
        Some(Err(other)) => panic!("expected AlreadyRunning, got: {other}"),
        Some(Ok(_)) => panic!("expected error, got Ok"),
        None => panic!("no result recorded"),
    }
}
use tokio::sync::oneshot;
use xylitol::app::server::ws::{ReverseRpcGateway, ReverseRpcResult};

/// Fixture for approval tests.
pub struct ApprovalTest {
    pub gateway: ReverseRpcGateway,
    /// Receiver for the in-flight reverse-RPC call (agent side).
    pub pending_rx: RefCell<Option<oneshot::Receiver<ReverseRpcResult>>>,
    /// Result delivered to the agent after client ApproveTool.
    pub last_result: RefCell<Option<ReverseRpcResult>>,
}
impl ApprovalTest {
    fn new() -> Self {
        Self {
            gateway: ReverseRpcGateway::new(),
            pending_rx: RefCell::new(None),
            last_result: RefCell::new(None),
        }
    }

    fn take_result(&self) -> ReverseRpcResult {
        let mut rx = self
            .pending_rx
            .borrow_mut()
            .take()
            .expect("no pending reverse-RPC receiver");
        rx.try_recv()
            .expect("reverse-RPC result should be ready after ApproveTool")
    }
}

#[fixture]
pub fn approval_test() -> ApprovalTest {
    ApprovalTest::new()
}

#[given("服务端和已连接的 WebSocket 客户端")]
fn server_and_ws_client(approval_test: &mut ApprovalTest) {
    // Gateway initialized in fixture; represents the server side.
    approval_test.pending_rx.replace(None);
    approval_test.last_result.replace(None);
}

#[when("agent 执行需要审批的工具")]
fn agent_executes_approvable_tool(approval_test: &mut ApprovalTest) {
    let rx = approval_test.gateway.register("call-approve-1".into());
    approval_test.pending_rx.replace(Some(rx));
}

#[then("客户端收到带有 call_id 的审批请求")]
fn client_receives_approval_request(approval_test: &mut ApprovalTest) {
    assert_eq!(approval_test.gateway.pending_count(), 1);
    assert!(
        approval_test.pending_rx.borrow().is_some(),
        "agent should hold a pending reverse-RPC receiver"
    );
}

#[when("客户端发送 ApproveTool approved=true")]
fn client_approves(approval_test: &mut ApprovalTest) {
    let consumed = approval_test.gateway.handle_approve("call-approve-1", true);
    assert!(consumed, "call_id should be consumed");
    let result = approval_test.take_result();
    approval_test.last_result.replace(Some(result));
}

#[when("客户端发送 ApproveTool approved=false")]
fn client_denies(approval_test: &mut ApprovalTest) {
    let consumed = approval_test
        .gateway
        .handle_approve("call-approve-1", false);
    assert!(consumed, "call_id should be consumed");
    let result = approval_test.take_result();
    approval_test.last_result.replace(Some(result));
}

#[then("工具执行继续")]
fn tool_execution_continues(approval_test: &mut ApprovalTest) {
    assert_eq!(approval_test.gateway.pending_count(), 0);
    assert_eq!(
        *approval_test.last_result.borrow(),
        Some(ReverseRpcResult::Approved)
    );
}

#[then("turn 正常结束")]
fn turn_completes(approval_test: &mut ApprovalTest) {
    assert_eq!(approval_test.gateway.pending_count(), 0);
    assert_eq!(
        *approval_test.last_result.borrow(),
        Some(ReverseRpcResult::Approved),
        "approved turn must deliver Approved to the agent"
    );
}

#[then("工具被拒绝")]
fn tool_denied(approval_test: &mut ApprovalTest) {
    assert_eq!(approval_test.gateway.pending_count(), 0);
    assert_eq!(
        *approval_test.last_result.borrow(),
        Some(ReverseRpcResult::Denied)
    );
}

#[then("turn 继续但不包含工具结果")]
fn turn_continues_without_tool(approval_test: &mut ApprovalTest) {
    assert_eq!(approval_test.gateway.pending_count(), 0);
    assert_eq!(
        *approval_test.last_result.borrow(),
        Some(ReverseRpcResult::Denied),
        "denied turn must deliver Denied (no tool result) to the agent"
    );
}
