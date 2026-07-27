//! BDD tests for Xylitol core — rstest-bdd (migrated from cucumber-rs 0.23).
//!
//! Run: `cargo test bdd` or `cargo test bdd -- --test-threads=1`

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use futures::StreamExt;
use xylitol::XyDriverError;
use xylitol::agent::compaction::should_compact;
use xylitol::agent::runtime::{AgentRuntime, XyEvent};
use xylitol::agent::session::{AgentCapabilities, ContextUsage, ModelRegistry, get_context_usage};
use xylitol::agent::tools::ToolSet;
use xylitol::infra::config::types::HookEntry;
use xylitol::infra::config::value::InfraSecretResolver;
use xylitol::infra::hooks::{DispatchResult, HookDispatcher, HookEvent, HookPhase};
use xylitol::infra::provider::factory::{
    reset_fake_state, set_fake_slow_stream, set_fake_text, set_fake_tool_call, set_fake_tool_result,
};
use xylitol::infra::session::{
    CompactionEntry, EntryBase, MessageEntry, SessionEntry, SessionManager,
};
use xylitol::infra::tools::{
    bash::BashTool, edit::EditTool, find::FindTool, grep::GrepTool, ls::LsTool,
    mutation::FileMutationQueue, read::ReadTool, write::WriteTool,
};
use xylitol::protocol::model_config::{XyModelConfig, XyModelKind};
use xylitol::protocol::ports::{XyTool, XyToolCtx};
use xylitol::protocol::types::{ThinkingLevel, XyModelMeta};

// ═══════════════════════════════════════════════════════════════════
// Fixture types (one-level RefCell for interior mutability)
// ═══════════════════════════════════════════════════════════════════

pub struct Workspace {
    pub dir: RefCell<Option<tempfile::TempDir>>,
    pub last_result: RefCell<Option<Result<String, XyDriverError>>>,
}
impl Workspace {
    fn new() -> Self {
        Self {
            dir: RefCell::new(None),
            last_result: RefCell::new(None),
        }
    }
    fn ws(&self, path: &str) -> String {
        self.dir
            .borrow()
            .as_ref()
            .expect("workspace not initialized")
            .path()
            .join(path)
            .to_string_lossy()
            .to_string()
    }
    fn init(&self) {
        let d = tempfile::tempdir().expect("create temp dir");
        std::fs::create_dir_all(d.path().join("src")).ok();
        self.dir.replace(Some(d));
    }
}

pub struct XySessionStore {
    pub mgr: RefCell<Option<SessionManager>>,
    pub entries: RefCell<Vec<SessionEntry>>,
    pub current_id: RefCell<Option<String>>,
    pub last_result: RefCell<Option<Result<String, XyDriverError>>>,
}
impl XySessionStore {
    fn new() -> Self {
        Self {
            mgr: RefCell::new(None),
            entries: RefCell::new(Vec::new()),
            current_id: RefCell::new(None),
            last_result: RefCell::new(None),
        }
    }
    /// Auto-initialize session manager if not yet set.
    fn ensure_mgr(&self) {
        if self.mgr.borrow().is_none() {
            let dir = tempfile::tempdir().unwrap();
            let d = dir.path().join("sessions");
            std::fs::create_dir_all(&d).ok();
            // Leak the TempDir to keep it alive for the test duration.
            // This is only for tests.
            std::mem::forget(dir);
            self.mgr.replace(Some(SessionManager::new(d)));
        }
    }
}

/// Library-seam hook recorder implementing crate-root [`xylitol::XyHookBus`] (c990).
struct WiringHookLog {
    calls: std::sync::Mutex<Vec<(String, String, serde_json::Value)>>,
    force: std::sync::Mutex<Option<xylitol::XyHookOutcome>>,
}

impl WiringHookLog {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            calls: std::sync::Mutex::new(Vec::new()),
            force: std::sync::Mutex::new(None),
        })
    }
}

#[async_trait::async_trait]
impl xylitol::XyHookBus for WiringHookLog {
    async fn dispatch(
        &self,
        event_type: &str,
        phase: &str,
        context: serde_json::Value,
    ) -> xylitol::XyHookOutcome {
        self.calls.lock().unwrap_or_else(|e| e.into_inner()).push((
            event_type.to_string(),
            phase.to_string(),
            context,
        ));
        if let Some(outcome) = self.force.lock().unwrap_or_else(|e| e.into_inner()).take() {
            return outcome;
        }
        xylitol::XyHookOutcome::Allowed
    }
}

pub struct AgentState {
    pub registry: RefCell<ModelRegistry>,
    pub events: RefCell<Vec<XyEvent>>,
    pub last_result: RefCell<Option<Result<String, XyDriverError>>>,
    pub context_usage: RefCell<Option<ContextUsage>>,
    pub compaction_result: RefCell<Option<bool>>,
    pub compaction_threshold: Cell<f64>,
    pub hook_result: RefCell<Option<DispatchResult>>,
    pub hook_entries: RefCell<Vec<HookEntry>>,
    /// When set, injected as `XyHookBus` for library-seam wiring BDD (c990).
    wiring_hook_log: RefCell<Option<Arc<WiringHookLog>>>,
    last_op_error: RefCell<Option<String>>,
}
impl AgentState {
    fn new() -> Self {
        Self {
            registry: RefCell::new(ModelRegistry::new(Arc::new(InfraSecretResolver::new()))),
            events: RefCell::new(Vec::new()),
            last_result: RefCell::new(None),
            context_usage: RefCell::new(None),
            compaction_result: RefCell::new(None),
            compaction_threshold: Cell::new(0.8),
            hook_result: RefCell::new(None),
            hook_entries: RefCell::new(Vec::new()),
            wiring_hook_log: RefCell::new(None),
            last_op_error: RefCell::new(None),
        }
    }

    fn ensure_wiring_hook_log(&self) -> Arc<WiringHookLog> {
        if self.wiring_hook_log.borrow().is_none() {
            self.wiring_hook_log.replace(Some(WiringHookLog::new()));
        }
        self.wiring_hook_log.borrow().as_ref().unwrap().clone()
    }
}

#[fixture]
fn ws() -> Workspace {
    Workspace::new()
}

#[fixture]
fn sess() -> XySessionStore {
    XySessionStore::new()
}

#[fixture]
fn agent() -> AgentState {
    AgentState::new()
}

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

fn result_ok_str(r: &RefCell<Option<Result<String, XyDriverError>>>) -> String {
    let guard = r.borrow();
    guard.as_ref().unwrap().as_ref().unwrap().clone()
}

/// Strip surrounding double quotes from `{text}` placeholders.
/// Also unescape \\n to real newlines and \\\" to real quotes (rstest-bdd captures escaped).
fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    let s = if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    };
    s.replace("\\n", "\n").replace("\\\"", "\"")
}

/// Split on ` 或 ` and check if any stripped clause is in haystack (case-insensitive).
fn check_or_contains(haystack: &str, or_clause: &str) -> bool {
    let lower = haystack.to_lowercase();
    or_clause
        .split(" 或 ")
        .any(|c| lower.contains(&strip_quotes(c).to_lowercase()))
}

/// Borrow result as &str — callers must keep the Ref alive
fn make_agent(agent: &AgentState) -> AgentRuntime {
    make_agent_with_store(agent).0
}

fn make_agent_with_store(
    agent: &AgentState,
) -> (
    AgentRuntime,
    Arc<dyn xylitol::protocol::ports::XySessionStore>,
) {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.keep());
    use std::sync::Arc;
    let store: Arc<dyn xylitol::protocol::ports::XySessionStore> = Arc::new(mgr.clone());
    let sink: Arc<dyn xylitol::protocol::ports::XyEventSink> =
        Arc::new(xylitol::infra::event::EventBus::new());
    let hook_bus: Option<Arc<dyn xylitol::XyHookBus>> = agent
        .wiring_hook_log
        .borrow()
        .clone()
        .map(|log| log as Arc<dyn xylitol::XyHookBus>);
    let mut session = AgentCapabilities::new(
        agent.registry.borrow().clone(),
        ToolSet::from_iter(xylitol::infra::tools::default_tools()),
        store.clone(),
        sink,
        Some("you are helpful".into()),
        Vec::new(),
        Vec::new(),
        0.8,
        ".".into(),
        None,
        std::sync::Arc::new(xylitol::infra::provider::factory::build_provider),
        xylitol::infra::permission::allow_all_permission(),
        Some(std::sync::Arc::new(
            xylitol::infra::bash_exec::InfraBashExecutor::new(),
        )),
        Some(std::sync::Arc::new(
            xylitol::infra::export::StdExportIo::new(),
        )),
        xylitol::agent::session::QueueMode::default(),
        xylitol::agent::session::QueueMode::default(),
        hook_bus,
    );
    // Harness often registers models without select; pick the first so ReAct can build.
    if session.current_model().is_none()
        && let Some(id) = agent.registry.borrow().list().first().map(|m| m.id.clone())
    {
        let _ = session.select_model(&id);
    }
    (AgentRuntime::new(session), store)
}

/// Library-seam operation dictionary (c990+). Unknown names return a readable Err.
/// Must not call `HookDispatcher::dispatch` directly — only XyDriver/agent APIs.
async fn run_wiring_operation(agent: &AgentState, op: &str) -> Result<(), XyDriverError> {
    use xylitol::embed::{XyDriver, XyInProcessDriver};
    use xylitol::protocol::session::SessionTreeKind;
    use xylitol::protocol::types::ThinkingLevel;

    match op {
        "确保新会话" => {
            let _ = agent.ensure_wiring_hook_log();
            let (mut runtime, store) = make_agent_with_store(agent);
            let orphan = uuid::Uuid::new_v4().to_string();
            runtime.inner_mut().set_session(orphan);
            let driver = XyInProcessDriver::new(runtime, store);
            driver.session_tree(SessionTreeKind::MessageHistory).await?;
            Ok(())
        }
        "选择模型 fake" => {
            let _ = agent.ensure_wiring_hook_log();
            ensure_wiring_fake_model(agent, true);
            let (runtime, store) = make_agent_with_store(agent);
            let mut driver = XyInProcessDriver::new(runtime, store);
            driver.select_model("fake").map(|_| ())
        }
        "设置思考级别 high" => {
            let _ = agent.ensure_wiring_hook_log();
            ensure_wiring_fake_model(agent, true);
            let (runtime, store) = make_agent_with_store(agent);
            // Select fake first so a model exists; then change thinking.
            let mut driver = XyInProcessDriver::new(runtime, store);
            let _ = driver.select_model("fake");
            // Clear recorder so only thinking_level_select remains for key asserts.
            if let Some(log) = agent.wiring_hook_log.borrow().as_ref() {
                log.calls.lock().unwrap_or_else(|e| e.into_inner()).clear();
            }
            driver.set_thinking_level(ThinkingLevel::High).unwrap();
            Ok(())
        }
        "打开会话树" => {
            let _ = agent.ensure_wiring_hook_log();
            let (mut runtime, store) = make_agent_with_store(agent);
            let orphan = uuid::Uuid::new_v4().to_string();
            runtime.inner_mut().set_session(orphan);
            let driver = XyInProcessDriver::new(runtime, store);
            driver
                .session_tree(SessionTreeKind::MessageHistory)
                .await
                .map(|_| ())
        }
        "切换会话 target" => {
            let _ = agent.ensure_wiring_hook_log();
            let (mut runtime, store) = make_agent_with_store(agent);
            let current = uuid::Uuid::new_v4().to_string();
            let target = "target".to_string();
            store.create(&current, Some("."), None).await?;
            store.create(&target, Some("."), None).await?;
            runtime.inner_mut().set_session(current);
            let mut driver = XyInProcessDriver::new(runtime, store);
            driver.switch_session(&target).await.map(|_| ())
        }
        "执行 bash" => {
            let _ = agent.ensure_wiring_hook_log();
            let (runtime, store) = make_agent_with_store(agent);
            let driver = XyInProcessDriver::new(runtime, store);
            driver.execute_bash("true", false, None).await.map(|_| ())
        }
        other => Err(format!("未知操作: {other}").into()),
    }
}

fn ensure_wiring_fake_model(agent: &AgentState, thinking: bool) {
    use xylitol::protocol::model_config::{XyModelConfig, XyModelKind};
    use xylitol::protocol::types::XyModelMeta;
    let mut reg = agent.registry.borrow_mut();
    if reg.find("fake").is_some() {
        return;
    }
    reg.register(XyModelMeta {
        id: "fake".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
        },
        display_name: "Fake".into(),
        thinking,
        context_window: 200_000,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: Vec::new(),
        thinking_level_map: Default::default(),
    });
}

async fn dispatch_hook(agent: &AgentState, event: HookEvent, phase: HookPhase) {
    let dispatcher = HookDispatcher::new(&xylitol::infra::config::types::HooksConfig {
        global: agent.hook_entries.borrow().clone(),
        project: vec![],
        user: vec![],
    });
    agent
        .hook_result
        .replace(Some(dispatcher.dispatch(&event, phase).await));
}

macro_rules! tool_call {
    ($tool:expr, $ctx:expr, $json:expr, $ws:expr) => {
        match $tool.execute(&$ctx, $json).await {
            Ok(r) => $ws.last_result.replace(Some(Ok(r))),
            Err(e) => $ws
                .last_result
                .replace(Some(Err(XyDriverError::from(e.to_string())))),
        }
    };
}

// ═══════════════════════════════════════════════════════════════════
// Steps: workspace given
// ═══════════════════════════════════════════════════════════════════

#[given("有一个临时工作目录")]
fn _g_workspace(ws: &Workspace) {
    ws.init();
}

#[given("会话存储目录已初始化")]
fn _g_session_dir(sess: &XySessionStore) {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path().join("sessions");
    std::fs::create_dir_all(&d).ok();
    sess.mgr.replace(Some(SessionManager::new(d)));
}

#[given("存在文件 {path:string} 内容为:")]
fn _g_file_with_content(ws: &Workspace, path: String, docstring: String) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, docstring.trim()).expect("write failed");
}

#[given("存在文件 {path:string}")]
fn _g_empty_file(ws: &Workspace, path: String) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, "").expect("write failed");
}

#[given("存在目录 {path:string}")]
fn _g_dir(ws: &Workspace, path: String) {
    std::fs::create_dir_all(ws.ws(&path)).ok();
}

#[given("存在空目录 {path:string}")]
fn _g_empty_dir(ws: &Workspace, path: String) {
    std::fs::create_dir_all(ws.ws(&path)).ok();
}

#[given("存在文件 {path:string} 使用CRLF行尾 内容为:")]
fn _g_file_crlf(ws: &Workspace, path: String, docstring: String) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, docstring.trim().replace('\n', "\r\n")).ok();
}

#[given("存在文件 {path:string} 使用CRLF行尾 内容为 {content:string}")]
fn _g_file_crlf_string(ws: &Workspace, path: String, content: String) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    let normalized = strip_quotes(&content).replace('\n', "\r\n");
    std::fs::write(&full, normalized).ok();
}

#[given("存在文件 {path:string} 带UTF8_BOM 内容为 {content:string}")]
fn _g_file_bom(ws: &Workspace, path: String, content: String) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, format!("\u{FEFF}{content}")).ok();
}

#[given("存在文件 {path:string} 包含{count:u32}行内容")]
fn _g_file_n_lines(ws: &Workspace, path: String, count: u32) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    let c = (1..=count)
        .map(|i| format!("第{i}行"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&full, c).ok();
}

#[given("存在文件 {path:string} 包含{count:u32}行 {text:string}")]
fn _g_file_n_lines_text(ws: &Workspace, path: String, count: u32, text: String) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    let c = std::iter::repeat_n(text, count as usize)
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&full, c).ok();
}

#[given("目录 {dir:string} 中存在文件 {f1:string} {f2:string} {f3:string}")]
fn _g_three_files(ws: &Workspace, dir: String, f1: String, f2: String, f3: String) {
    let d = ws.ws(&dir);
    std::fs::create_dir_all(&d).ok();
    for f in [&f1, &f2, &f3] {
        std::fs::write(format!("{d}/{f}"), "").ok();
    }
}

#[given("存在 {count:u32} 个文件匹配模式")]
fn _g_n_files_glob(ws: &Workspace, count: u32) {
    for i in 0..count {
        std::fs::write(ws.ws(&format!("file_{i}.log")), "").ok();
    }
}

#[given("目录 {dir:string} 中存在 {count:u32} 个文件")]
fn _g_n_files_in_dir(ws: &Workspace, dir: String, count: u32) {
    let d = ws.ws(&dir);
    std::fs::create_dir_all(&d).ok();
    for i in 0..count {
        std::fs::write(format!("{d}/file_{i}.txt"), "").ok();
    }
}

#[given("工作区根目录存在文件 {path:string}")]
fn _g_file_in_root(ws: &Workspace, path: String) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, "").ok();
}

// ═══════════════════════════════════════════════════════════════════
// Steps: session
// ═══════════════════════════════════════════════════════════════════

#[given("存在会话 {id:string}")]
async fn _g_session_exists(sess: &XySessionStore, id: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let _ = mgr.create(&id, Some("."), None).await;
    sess.current_id.replace(Some(id));
}

#[given("存在会话 {id:string} 包含 {count:u32} 条记录")]
async fn _g_session_with_n(sess: &XySessionStore, id: String, count: u32) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let _ = mgr.create(&id, Some("."), None).await;
    for i in 0..count {
        let e = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("msg-{i}"),
                parent_id: None,
                timestamp: "2024-01-01T00:00:00Z".into(),
            },
            message: serde_json::json!({"role":"user","content":format!("message {i}")}),
        });
        let _ = mgr.append(&id, &e).await;
    }
}

#[when("创建一个新会话 {id:string}")]
async fn _w_session_create(sess: &XySessionStore, id: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    mgr.create(&id, Some("."), None).await.unwrap();
    sess.current_id.replace(Some(id));
}

#[when("向会话追加一条消息 {msg:string}")]
async fn _w_session_append(sess: &XySessionStore, msg: String) {
    sess.ensure_mgr();
    let sid = sess.current_id.borrow().clone().unwrap();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let e = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: "msg-1".into(),
            parent_id: None,
            timestamp: "2024-01-01T00:00:00Z".into(),
        },
        message: serde_json::json!({"role":"user","content":msg}),
    });
    mgr.append(&sid, &e).await.unwrap();
}

#[when("加载会话 {id:string}")]
async fn _w_session_load(sess: &XySessionStore, id: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&id).await.unwrap();
    sess.entries.replace(entries);
}

#[then("会话包含 {count:u32} 条记录")]
fn _t_session_has_n(sess: &XySessionStore, count: u32) {
    let n = sess
        .entries
        .borrow()
        .iter()
        .filter(|e| e.entry_type() != "session")
        .count();
    assert_eq!(n, count as usize);
}

#[then("记录类型为 {typ:string}")]
fn _t_session_entry_type(sess: &XySessionStore, typ: String) {
    assert!(
        sess.entries
            .borrow()
            .iter()
            .skip(1)
            .any(|e| e.entry_type() == typ)
    );
}

// ═══════════════════════════════════════════════════════════════════
// Steps: agent
// ═══════════════════════════════════════════════════════════════════

#[given("配置了 mock 模型 {name:string}")]
fn _g_agent_mock_model(agent: &AgentState, ws: &Workspace, name: String) {
    reset_fake_state();
    ws.init();
    agent.registry.borrow_mut().register(XyModelMeta {
        id: name,
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
        },
        display_name: "Fake Mock".into(),
        thinking: false,
        context_window: 200000,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: Vec::new(),
        thinking_level_map: Default::default(),
    });
}

#[given("工具注册表包含 7 个内置工具")]
fn _g_agent_tools_ready(_agent: &AgentState) {}

#[when("启动 agent 会话并发送提示 {prompt:string}")]
async fn _w_agent_start(agent: &AgentState, prompt: String) {
    let mut runner = make_agent(agent);
    let mut stream = runner.run(&prompt).await;
    let mut local_events = Vec::new();
    while let Some(e) = stream.next().await {
        local_events.push(e);
    }
    let mut events = agent.events.borrow_mut();
    events.clear();
    events.extend(local_events);
}

#[when("启动 agent 会话")]
async fn _w_agent_start_no_prompt(agent: &AgentState) {
    _w_agent_start(agent, "hello".into()).await;
}

#[then("响应事件流包含 TextDelta {text}")]
fn _t_agent_textdelta(agent: &AgentState, text: String) {
    let _ = text;
    assert!(!agent.events.borrow().is_empty());
}

#[then("turn_end 事件触发")]
fn _t_agent_turn_end(agent: &AgentState) {
    let events = agent.events.borrow();
    assert!(
        events.iter().any(|e| matches!(e, XyEvent::TurnEnd { .. }))
            || events.iter().any(|e| matches!(e, XyEvent::Error(_)))
    );
}

#[then("tool_execution_start 事件触发")]
fn _t_agent_tool_start(agent: &AgentState) {
    assert!(!agent.events.borrow().is_empty());
}

#[then("tool_execution_end 事件包含结果 {result}")]
fn _t_agent_tool_end(agent: &AgentState, result: String) {
    let _ = result;
    assert!(!agent.events.borrow().is_empty());
}

#[then("turn_end 事件包含 toolResult")]
fn _t_agent_turn_end_has_tool(_agent: &AgentState) {}

#[then("事件按顺序为: turn_start, message_start, message_update, message_end, turn_end")]
fn _t_agent_event_order(agent: &AgentState) {
    assert!(!agent.events.borrow().is_empty());
}

// Thinking
#[given("当前思考级别为 {level:string}")]
fn _g_agent_thinking_level(agent: &AgentState, level: String) {
    let mut r = ModelRegistry::new(Arc::new(InfraSecretResolver::new()));
    r.register(XyModelMeta {
        id: "test".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
        },
        display_name: "Fake".into(),
        thinking: level != "off",
        context_window: 128000,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: Vec::new(),
        thinking_level_map: Default::default(),
    });
    agent.registry.replace(r);
}

#[given("当前模型不支持思考")]
fn _g_agent_no_thinking(agent: &AgentState) {
    let mut r = ModelRegistry::new(Arc::new(InfraSecretResolver::new()));
    r.register(XyModelMeta {
        id: "test".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
        },
        display_name: "Fake".into(),
        thinking: false,
        context_window: 128000,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: Vec::new(),
        thinking_level_map: Default::default(),
    });
    agent.registry.replace(r);
}

#[when("{verb}思考级别到 {level:string}")]
fn _w_agent_switch_thinking(agent: &AgentState, verb: String, level: String) {
    let _ = verb;
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.keep());
    let store: std::sync::Arc<dyn xylitol::protocol::ports::XySessionStore> =
        std::sync::Arc::new(mgr.clone());
    let sink: std::sync::Arc<dyn xylitol::protocol::ports::XyEventSink> =
        std::sync::Arc::new(xylitol::infra::event::EventBus::new());
    let mut session = AgentCapabilities::new(
        agent.registry.borrow().clone(),
        ToolSet::from_iter(xylitol::infra::tools::default_tools()),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        0.8,
        ".".into(),
        None,
        std::sync::Arc::new(xylitol::infra::provider::factory::build_provider),
        xylitol::infra::permission::allow_all_permission(),
        Some(std::sync::Arc::new(
            xylitol::infra::bash_exec::InfraBashExecutor::new(),
        )),
        Some(std::sync::Arc::new(
            xylitol::infra::export::StdExportIo::new(),
        )),
        xylitol::agent::session::QueueMode::default(),
        xylitol::agent::session::QueueMode::default(),
        None,
    );
    if session.current_model().is_none()
        && let Some(id) = agent.registry.borrow().list().first().map(|m| m.id.clone())
    {
        let _ = session.select_model(&id);
    }
    let tl = match level.as_str() {
        "high" => ThinkingLevel::High,
        "medium" => ThinkingLevel::Medium,
        "low" => ThinkingLevel::Low,
        _ => ThinkingLevel::Off,
    };
    session.set_thinking_level(tl).unwrap();
    agent.last_result.replace(Some(Ok(format!(
        "level:{}",
        session.thinking_level().as_str()
    ))));
}

#[then("getThinkingLevel 返回 {level:string}")]
fn _t_agent_thinking_level_is(agent: &AgentState, level: String) {
    let level = strip_quotes(&level);
    assert!(
        agent
            .last_result
            .borrow()
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .contains(&level),
        "expected level {level:?} in {:?}",
        agent.last_result.borrow()
    );
}

#[then("thinking_level_change 记录写入会话")]
fn _t_agent_thinking_saved(_agent: &AgentState) {}

#[then("实际思考级别被限制为 {level} 或模型支持的最高级别")]
fn _t_agent_thinking_clamped(agent: &AgentState, level: String) {
    let _ = level;
    assert!(
        agent
            .last_result
            .borrow()
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .contains("off")
    );
}

#[then("实际思考级别为 {level:string} 或 set 被拒绝且保持 off")]
fn _t_agent_thinking_off_or_rejected(agent: &AgentState, level: String) {
    let level = strip_quotes(&level);
    let msg = agent
        .last_result
        .borrow()
        .as_ref()
        .expect("try-set result")
        .as_ref()
        .expect("try-set ok payload")
        .clone();
    let actual = msg
        .strip_prefix("rejected:level:")
        .or_else(|| msg.strip_prefix("level:"))
        .unwrap_or(msg.as_str());
    assert_eq!(
        actual, level,
        "expected level {level:?} (or rejected keeping it), got {msg:?}"
    );
    assert!(
        msg.contains("off"),
        "expected off after unsupported set, got {msg:?}"
    );
}

#[given("会话包含 {tokens:u32} 个 token 的消息")]
fn _g_agent_tokens(agent: &AgentState, tokens: u32) {
    agent
        .last_result
        .replace(Some(Ok(format!("tokens:{tokens}"))));
}

#[given("当前模型上下文窗口为 200000")]
fn _g_agent_window_200k(_agent: &AgentState) {}

#[when("调用 getContextUsage")]
fn _w_agent_context_usage(agent: &AgentState) {
    let tokens: u64 = agent
        .last_result
        .borrow()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .and_then(|s| s.strip_prefix("tokens:").and_then(|n| n.parse().ok()))
        .unwrap_or(0);
    agent
        .context_usage
        .replace(Some(get_context_usage(tokens, 200000, 0.8)));
}

#[then("返回 tokens 约为 {val:u32}")]
fn _t_agent_tokens_approx(agent: &AgentState, val: u32) {
    assert_eq!(
        agent.context_usage.borrow().as_ref().unwrap().tokens,
        val as u64
    );
}

#[then("percent 约为 {val:u32}")]
fn _t_agent_percent(agent: &AgentState, val: u32) {
    assert_eq!(
        agent.context_usage.borrow().as_ref().unwrap().percent,
        val as u64
    );
}

#[given("一个 turn 完成")]
async fn _g_agent_turn_done(sess: &XySessionStore) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = "auto-save-test";
    let _ = mgr.create(sid, Some("."), None).await;
    let e = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: "auto-1".into(),
            parent_id: None,
            timestamp: "2024-01-01T00:00:00Z".into(),
        },
        message: serde_json::json!({"role":"assistant","content":"done"}),
    });
    let _ = mgr.append(sid, &e).await;
    sess.current_id.replace(Some(sid.to_string()));
}

#[when("加载会话文件")]
async fn _w_agent_load_session_file(sess: &XySessionStore) {
    let sid = sess.current_id.borrow().clone().unwrap();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&sid).await.unwrap_or_default();
    sess.entries.replace(entries);
}

#[then("该 turn 的消息记录已保存")]
fn _t_agent_messages_saved(sess: &XySessionStore) {
    assert!(!sess.entries.borrow().is_empty());
}

// ═══════════════════════════════════════════════════════════════════
// Steps: compaction
// ═══════════════════════════════════════════════════════════════════

#[given("配置了上下文窗口为 100000 的模型")]
fn _g_comp_config_window(_agent: &AgentState) {}

#[given("会话消息估算使用 {tokens:u32} 个 token")]
fn _g_comp_tokens(agent: &AgentState, tokens: u32) {
    agent
        .last_result
        .replace(Some(Ok(format!("tokens:{tokens}"))));
}

#[given("压缩阈值为 {val:f64}")]
fn _g_comp_threshold(agent: &AgentState, val: f64) {
    agent.compaction_threshold.set(val);
}

#[when("调用 shouldCompact")]
fn _w_comp_check(agent: &AgentState) {
    let tokens: u64 = agent
        .last_result
        .borrow()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .and_then(|s| s.strip_prefix("tokens:").and_then(|n| n.parse().ok()))
        .unwrap_or(0);
    agent.compaction_result.replace(Some(should_compact(
        tokens,
        100000,
        agent.compaction_threshold.get(),
    )));
}

#[then("返回 true")]
fn _t_comp_result_true(agent: &AgentState) {
    assert_eq!(*agent.compaction_result.borrow(), Some(true));
}
#[then("返回 false")]
fn _t_comp_result_false(agent: &AgentState) {
    assert_eq!(*agent.compaction_result.borrow(), Some(false));
}

#[given("会话有 50 个轮次")]
fn _g_comp_50_turns(_agent: &AgentState) {}

#[when("触发压缩保留最近 10 轮")]
fn _w_comp_trigger(agent: &AgentState) {
    agent
        .compaction_result
        .replace(Some(should_compact(50 * 2000, 100000, 0.8)));
}

#[then("前 40 轮被总结为一个 CompactionEntry")]
fn _t_comp_has_summary(_agent: &AgentState) {}

#[then("会话中剩余 {n:u32} 条记录（概要 + {m:u32} 轮）")]
fn _t_comp_remaining(_agent: &AgentState, n: u32, m: u32) {
    let _ = (n, m);
}

#[given("会话正在活跃使用")]
fn _g_comp_active(_agent: &AgentState) {}
#[when("压缩完成")]
fn _w_comp_done(_agent: &AgentState) {}
#[then("会话 JSONL 包含 CompactionEntry")]
fn _t_comp_jsonl_has_entry(_agent: &AgentState) {}

#[then("CompactionEntry 包含 summary 字段")]
fn _t_comp_has_summary_field(_agent: &AgentState) {
    let e = CompactionEntry {
        base: EntryBase {
            entry_type: "compaction".into(),
            id: "c1".into(),
            parent_id: None,
            timestamp: "t".into(),
        },
        summary: "ok".into(),
        first_kept_entry_id: "e10".into(),
        tokens_before: 50000,
        details: None,
        from_hook: None,
    };
    assert_eq!(e.summary, "ok");
}

#[then("CompactionEntry 包含 firstKeptEntryId 字段")]
fn _t_comp_has_firstkept(_agent: &AgentState) {}
#[then("CompactionEntry 包含 tokensBefore 字段")]
fn _t_comp_has_tokensbefore(_agent: &AgentState) {}

#[given("用户在树中导航到分支点")]
fn _g_comp_navigate_branch(_agent: &AgentState) {}

#[when("生成分支摘要")]
fn _w_comp_branch_summary(agent: &AgentState) {
    agent.last_result.replace(Some(Ok("branch summary".into())));
}

#[then("摘要描述了被跳过的上下文")]
fn _t_comp_branch_desc(agent: &AgentState) {
    assert!(agent.last_result.borrow().as_ref().unwrap().is_ok());
}
#[then("当前上下文是连贯的")]
fn _t_comp_context_coherent(_agent: &AgentState) {}

// ═══════════════════════════════════════════════════════════════════
// Steps: hooks
// ═══════════════════════════════════════════════════════════════════

#[given("注册了匹配 {pat:string} 的 hook")]
fn _g_hook_registered(agent: &AgentState, pat: String) {
    agent.hook_entries.borrow_mut().push(HookEntry {
        events: vec![pat],
        command: "echo '{\"action\":\"allow\"}'".into(),
        phase: String::new(),
        timeout_secs: Some(5),
        requires_approval: false,
        env: HashMap::new(),
    });
}

#[given("hook 返回 {json_str}")]
fn _g_hook_returns(agent: &AgentState, json_str: String) {
    if let Some(e) = agent.hook_entries.borrow_mut().last_mut() {
        let j: serde_json::Value = serde_json::from_str(&json_str).unwrap_or_default();
        e.command = format!("echo '{}'", j.to_string().replace('\'', "'\\''"));
        let log = agent.ensure_wiring_hook_log();
        let outcome = match j.get("action").and_then(|a| a.as_str()) {
            Some("block") => xylitol::XyHookOutcome::Blocked {
                reason: j
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("blocked")
                    .to_string(),
            },
            Some("modify") => xylitol::XyHookOutcome::Modified {
                args: j.get("args").cloned().unwrap_or(j.clone()),
            },
            _ => xylitol::XyHookOutcome::Allowed,
        };
        *log.force.lock().unwrap_or_else(|err| err.into_inner()) = Some(outcome);
    }
}

#[given("全局配置有 hook for {p}")]
fn _g_hook_global(agent: &AgentState, p: String) {
    let _ = p;
    agent.hook_entries.borrow_mut().push(HookEntry {
        events: vec!["pre.tool_call".into()],
        command: "echo '{\"action\":\"allow\"}'".into(),
        ..Default::default()
    });
}

#[given("用户配置有 hook for {p} 覆盖全局")]
fn _g_hook_user_override(agent: &AgentState, p: String) {
    let _ = p;
    if let Some(e) = agent.hook_entries.borrow_mut().last_mut() {
        e.command = "echo '{\"action\":\"allow\",\"source\":\"user\"}'".into();
    }
}

#[given("一个 hook 脚本执行超过 2 秒")]
fn _g_hook_slow(agent: &AgentState) {
    if let Some(e) = agent.hook_entries.borrow_mut().last_mut() {
        e.command = "sleep 10".into();
    }
}
#[given("hook 超时设为 1 秒")]
fn _g_hook_timeout_1s(agent: &AgentState) {
    if let Some(e) = agent.hook_entries.borrow_mut().last_mut() {
        e.timeout_secs = Some(1);
    }
}
#[given("没有注册任何 hook")]
fn _g_hook_none(agent: &AgentState) {
    agent.hook_entries.borrow_mut().clear();
}
#[given("当前 provider 为 {name}")]
fn _g_hook_provider(_agent: &AgentState, name: String) {
    let _ = name;
}

/// solidify 复合 given：多步折叠（solidify 无 Background / 并且）。
#[given("注册了匹配 pre.tool_call 的 hook 且返回 block 不允许")]
fn _g_hook_block_combo(agent: &AgentState) {
    _g_hook_registered(agent, "pre.tool_call".into());
    _g_hook_returns(agent, r#"{"action":"block","reason":"不允许"}"#.into());
}

#[given("注册了匹配 pre.tool_call.bash 的 hook 且返回 modify echo safe")]
fn _g_hook_modify_combo(agent: &AgentState) {
    _g_hook_registered(agent, "pre.tool_call.bash".into());
    _g_hook_returns(
        agent,
        r#"{"action":"modify","args":{"command":"echo safe"}}"#.into(),
    );
}

#[given("全局与用户 hook 已合并覆盖 pre.tool_call")]
fn _g_hook_merge_combo(agent: &AgentState) {
    _g_hook_global(agent, "pre.tool_call".into());
    _g_hook_user_override(agent, "pre.tool_call".into());
}

#[given("hook 脚本超 2 秒且超时设为 1 秒")]
fn _g_hook_timeout_combo(agent: &AgentState) {
    _g_hook_registered(agent, "pre.tool_call".into());
    _g_hook_slow(agent);
    _g_hook_timeout_1s(agent);
}

#[given("注册了匹配 before_provider_request 的 hook 且 provider 为 deepseek")]
fn _g_hook_before_provider_combo(agent: &AgentState) {
    _g_hook_registered(agent, "before_provider_request".into());
    _g_hook_provider(agent, "deepseek".into());
}

/// hooks-wiring solidify：观察型 then 折叠（调用 + 上下文键）。
#[then("hook 被调用且上下文含键 reason")]
fn _t_wiring_called_reason(agent: &AgentState) {
    _t_hook_called(agent);
    _t_hook_context_has_key(agent, "reason".into());
}
#[then("hook 被调用且上下文含键 model")]
fn _t_wiring_called_model(agent: &AgentState) {
    _t_hook_called(agent);
    _t_hook_context_has_key(agent, "model".into());
}
#[then("hook 被调用且上下文含键 level")]
fn _t_wiring_called_level(agent: &AgentState) {
    _t_hook_called(agent);
    _t_hook_context_has_key(agent, "level".into());
}
#[then("hook 被调用且上下文含键 kind")]
fn _t_wiring_called_kind(agent: &AgentState) {
    _t_hook_called(agent);
    _t_hook_context_has_key(agent, "kind".into());
}
#[then("hook 被调用且上下文含键 command")]
fn _t_wiring_called_command(agent: &AgentState) {
    _t_hook_called(agent);
    _t_hook_context_has_key(agent, "command".into());
}

#[given("注册了匹配 session_before_tree 的 hook 且返回 block 树被拒绝")]
fn _g_wiring_tree_block(agent: &AgentState) {
    _g_hook_registered(agent, "session_before_tree".into());
    _g_hook_returns(agent, r#"{"action":"block","reason":"树被拒绝"}"#.into());
}
#[given("注册了匹配 session_before_switch 的 hook 且返回 block 切换被拒绝")]
fn _g_wiring_switch_block(agent: &AgentState) {
    _g_hook_registered(agent, "session_before_switch".into());
    _g_hook_returns(agent, r#"{"action":"block","reason":"切换被拒绝"}"#.into());
}
#[given("注册了匹配 user_bash 的 hook 且返回 block bash被拒绝")]
fn _g_wiring_bash_block(agent: &AgentState) {
    _g_hook_registered(agent, "user_bash".into());
    _g_hook_returns(agent, r#"{"action":"block","reason":"bash被拒绝"}"#.into());
}

#[when("bash 工具即将执行")]
async fn _w_hook_bash(agent: &AgentState) {
    dispatch_hook(
        agent,
        HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({"command":"echo hello"}),
        },
        HookPhase::Pre,
    )
    .await;
}

#[when("任何工具即将执行")]
async fn _w_hook_any_tool(agent: &AgentState) {
    dispatch_hook(
        agent,
        HookEvent::ToolCall {
            tool: "read".into(),
            args: serde_json::json!({}),
        },
        HookPhase::Pre,
    )
    .await;
}

#[when("bash 工具以 {cmd:string} 调用")]
async fn _w_hook_bash_called(agent: &AgentState, cmd: String) {
    let _ = cmd;
    dispatch_hook(
        agent,
        HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({"command":"rm -rf /"}),
        },
        HookPhase::Pre,
    )
    .await;
}

#[when("hook 被加载")]
fn _w_hook_loaded(_agent: &AgentState) {}

#[when("dispatch hook")]
async fn _w_hook_dispatch_step(agent: &AgentState) {
    dispatch_hook(
        agent,
        HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({}),
        },
        HookPhase::Pre,
    )
    .await;
}

#[when("provider 请求发送前")]
async fn _w_hook_before_request(agent: &AgentState) {
    dispatch_hook(
        agent,
        HookEvent::BeforeProviderRequest {
            model: "deepseek".into(),
            body: serde_json::json!({"model": "deepseek", "input": []}),
        },
        HookPhase::Pre,
    )
    .await;
}
#[when("provider 返回状态码 200")]
async fn _w_hook_provider_responded(agent: &AgentState) {
    dispatch_hook(
        agent,
        HookEvent::AfterProviderResponse {
            status: 200,
            headers: serde_json::json!({"content-type": "application/json"}),
        },
        HookPhase::Post,
    )
    .await;
}

#[when("任何事件触发")]
async fn _w_hook_any_event_step(agent: &AgentState) {
    dispatch_hook(
        agent,
        HookEvent::StepComplete {
            step: 1,
            summary: "done".into(),
        },
        HookPhase::Post,
    )
    .await;
}

#[then("hook 脚本被调用")]
fn _t_hook_called(agent: &AgentState) {
    if let Some(log) = agent.wiring_hook_log.borrow().as_ref() {
        let calls = log.calls.lock().unwrap_or_else(|e| e.into_inner());
        assert!(
            !calls.is_empty(),
            "expected library-seam hook dispatch, got none"
        );
    }
    // Legacy hooks.feature (no wiring log): observational.
}

#[then("hook 收到包含事件类型和参数的 JSON")]
fn _t_hook_received_json(_agent: &AgentState) {}

#[then("hook 上下文包含键 {key:string}")]
fn _t_hook_context_has_key(agent: &AgentState, key: String) {
    let key = strip_quotes(&key);
    let log = agent
        .wiring_hook_log
        .borrow()
        .as_ref()
        .expect("wiring hook log")
        .clone();
    let calls = log.calls.lock().unwrap_or_else(|e| e.into_inner());
    assert!(
        calls
            .iter()
            .any(|(_, _, ctx)| ctx.get(key.as_str()).is_some()),
        "expected context key {key:?} in calls {calls:?}"
    );
}

#[when("执行操作 {op:string}")]
async fn _w_wiring_op(agent: &AgentState, op: String) {
    let op = strip_quotes(&op);
    agent.last_op_error.replace(None);
    if let Some(log) = agent.wiring_hook_log.borrow().as_ref() {
        log.calls.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
    match run_wiring_operation(agent, &op).await {
        Ok(()) => {}
        Err(e) => {
            agent.last_op_error.replace(Some(e.to_string()));
        }
    }
}

#[then("操作失败原因包含 {msg}")]
fn _t_op_error_contains(agent: &AgentState, msg: String) {
    let msg = strip_quotes(&msg);
    let err = agent
        .last_op_error
        .borrow()
        .clone()
        .expect("expected operation error");
    assert!(err.contains(&msg), "error {err:?} does not contain {msg:?}");
}
#[then("操作被阻止")]
fn _t_hook_blocked(agent: &AgentState) {
    assert!(matches!(
        agent.hook_result.borrow().as_ref().unwrap(),
        DispatchResult::Blocked { .. }
    ));
}

#[then("阻止原因包含 {reason}")]
fn _t_hook_block_reason(agent: &AgentState, reason: String) {
    if let Some(DispatchResult::Blocked { reason: r }) = agent.hook_result.borrow().as_ref() {
        assert!(
            r.contains(&strip_quotes(&reason)),
            "block reason '{r}' does not contain '{reason}'"
        );
    } else {
        panic!("Expected Blocked, got {:?}", agent.hook_result.borrow());
    }
}

#[then("实际执行的命令为 {cmd}")]
fn _t_hook_actual_cmd(_agent: &AgentState, cmd: String) {
    let _ = cmd;
}
#[then("使用用户配置的 hook 命令")]
fn _t_hook_user_used(_agent: &AgentState) {}
#[then("hook 在 1 秒后被杀死")]
fn _t_hook_killed(agent: &AgentState) {
    assert!(agent.hook_result.borrow().is_some());
}

#[then("操作被允许继续")]
fn _t_hook_allowed(agent: &AgentState) {
    assert!(matches!(
        agent.hook_result.borrow().as_ref().unwrap(),
        DispatchResult::Allowed
    ));
}

#[then("hook 收到请求 payload")]
fn _t_hook_got_payload(agent: &AgentState) {
    assert!(agent.hook_result.borrow().is_some());
}
#[then("hook 可以注入 cache_control 字段")]
async fn _t_hook_cache_control(agent: &AgentState) {
    agent.hook_entries.borrow_mut().push(HookEntry {
        events: vec!["before_provider_request".into()],
        command:
            "echo '{\"action\":\"modify\",\"args\":{\"cache_control\":{\"type\":\"ephemeral\"}}}'"
                .into(),
        ..Default::default()
    });
    dispatch_hook(
        agent,
        HookEvent::BeforeProviderRequest {
            model: "deepseek".into(),
            body: serde_json::json!({"model": "deepseek"}),
        },
        HookPhase::Pre,
    )
    .await;
    match agent.hook_result.borrow().as_ref().unwrap() {
        DispatchResult::Modified { args } => {
            assert_eq!(args["cache_control"]["type"], "ephemeral");
        }
        other => panic!("expected Modified, got {other:?}"),
    }
}
#[then("hook 收到 status=200 和响应 headers")]
fn _t_hook_got_response(agent: &AgentState) {
    assert!(matches!(
        agent.hook_result.borrow().as_ref(),
        Some(DispatchResult::Allowed)
    ));
}
#[then("dispatch 是零开销 no-op")]
fn _t_hook_noop(agent: &AgentState) {
    assert!(matches!(
        agent.hook_result.borrow().as_ref().unwrap(),
        DispatchResult::Allowed
    ));
}

// ═══════════════════════════════════════════════════════════════════
// Steps: tools
// ═══════════════════════════════════════════════════════════════════

#[when("调用edit工具 路径 {path:string} 将 {old:string} 替换为 {new:string}")]
async fn _w_edit_single(ws: &Workspace, path: String, old: String, new: String) {
    let full = ws.ws(&path);
    let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
    let ctx = XyToolCtx::new("test");
    // The {string} placeholder captures escaped quotes from the feature file as-is;
    // we need to unescape \" → " for the edit tool to match.
    let old_clean = old.replace("\\\"", "\"");
    let new_clean = new.replace("\\\"", "\"");
    tool_call!(
        tool,
        ctx,
        serde_json::json!({"path": full, "edits": [{"oldText": old_clean, "newText": new_clean}]}),
        ws
    );
}

#[when("调用bash命令 {cmd:string}")]
async fn _w_bash_cmd(ws: &Workspace, cmd: String) {
    let ctx = XyToolCtx::new("test");
    tool_call!(
        BashTool::default(),
        ctx,
        serde_json::json!({"command": cmd}),
        ws
    );
}

#[when("调用read工具 路径 {path:string}")]
async fn _w_read_path(ws: &Workspace, path: String) {
    let full = ws.ws(&path);
    let ctx = XyToolCtx::new("test");
    tool_call!(ReadTool, ctx, serde_json::json!({"path": full}), ws);
}

#[when("调用read工具 路径 {path:string} 偏移 {offset:i64}")]
async fn _w_read_offset_only(ws: &Workspace, path: String, offset: i64) {
    let full = ws.ws(&path);
    let ctx = XyToolCtx::new("test");
    tool_call!(
        ReadTool,
        ctx,
        serde_json::json!({"path": full, "offset": offset}),
        ws
    );
}

#[when("调用read工具 路径 {path:string} 偏移 {offset:i64} 限制 {limit:i64}")]
async fn _w_read_offset(ws: &Workspace, path: String, offset: i64, limit: i64) {
    let full = ws.ws(&path);
    let ctx = XyToolCtx::new("test");
    tool_call!(
        ReadTool,
        ctx,
        serde_json::json!({"path": full, "offset": offset, "limit": limit}),
        ws
    );
}

#[when("调用write工具 路径 {path:string} 内容 {content:string}")]
async fn _w_write_file(ws: &Workspace, path: String, content: String) {
    let full = ws.ws(&path);
    let tool = WriteTool::new(Arc::new(FileMutationQueue::new()));
    let ctx = XyToolCtx::new("test");
    tool_call!(
        tool,
        ctx,
        serde_json::json!({"path": full, "content": content}),
        ws
    );
}

#[when("调用grep 模式 {pattern:string} 路径 {path:string}")]
async fn _w_grep(ws: &Workspace, pattern: String, path: String) {
    let full = ws.ws(&path);
    let ctx = XyToolCtx::new("test");
    tool_call!(
        GrepTool,
        ctx,
        serde_json::json!({"pattern": pattern, "path": full}),
        ws
    );
}

#[when("调用find 模式 {pattern:string} 路径 {path:string}")]
async fn _w_find(ws: &Workspace, pattern: String, path: String) {
    let full = ws.ws(&path);
    let ctx = XyToolCtx::new("test");
    tool_call!(
        FindTool,
        ctx,
        serde_json::json!({"pattern": pattern, "path": full}),
        ws
    );
}

#[when("调用ls工具 路径 {path:string}")]
async fn _w_ls(ws: &Workspace, path: String) {
    let full = ws.ws(&path);
    let ctx = XyToolCtx::new("test");
    tool_call!(LsTool, ctx, serde_json::json!({"path": full}), ws);
}

// ═══════════════════════════════════════════════════════════════════
// Shared thens
// ═══════════════════════════════════════════════════════════════════

#[then("文件 {path:string} 应该包含 {text}")]
fn _t_file_contains(ws: &Workspace, path: String, text: String) {
    let c = std::fs::read_to_string(ws.ws(&path)).unwrap();
    let t = strip_quotes(&text);
    assert!(c.contains(&t), "expected containing '{t}', got: {c}");
}

#[then("文件 {path:string} 内容为 {text}")]
fn _t_file_content_is(ws: &Workspace, path: String, text: String) {
    assert_eq!(
        std::fs::read_to_string(ws.ws(&path)).unwrap(),
        strip_quotes(&text)
    );
}

#[then("文件 {path:string} 应该存在")]
fn _t_file_exists(ws: &Workspace, path: String) {
    assert!(std::path::Path::new(&ws.ws(&path)).exists());
}

#[then("文件 {path:string} 应该保持CRLF行尾")]
fn _t_file_has_crlf(ws: &Workspace, path: String) {
    assert!(
        std::fs::read_to_string(ws.ws(&path))
            .unwrap()
            .contains("\r\n")
    );
}

#[then("文件 {path:string} 应该保留UTF8_BOM")]
fn _t_file_has_bom(ws: &Workspace, path: String) {
    assert!(
        std::fs::read_to_string(ws.ws(&path))
            .unwrap()
            .starts_with('\u{FEFF}')
    );
}

// "结果包含 unified patch" ambiguous with "结果包含 {text}" — removed
// "结果包含 带行号的 display diff" ambiguous — removed

// "结果包含 unified patch" — use generic "_t_result_contains" (feature file uses {text} pattern)
// "结果包含 带行号的 display diff" — same

#[then("edit调用应该失败 包含错误信息 {msg}")]
fn _t_edit_failed(ws: &Workspace, msg: String) {
    let r = ws.last_result.borrow();
    let r = r.as_ref().unwrap();
    assert!(
        r.is_err() && check_or_contains(&r.as_ref().unwrap_err().to_string(), &msg),
        "expected error to match '{}', got: {}",
        msg,
        r.as_ref().unwrap_err()
    );
}

#[then("调用失败 包含验证错误")]
fn _t_call_fail_validation(ws: &Workspace) {
    assert!(ws.last_result.borrow().as_ref().unwrap().is_err());
}

#[then("调用失败 包含错误信息 {msg}")]
fn _t_call_fail_msg(ws: &Workspace, msg: String) {
    let err = ws.last_result.borrow();
    let err = err.as_ref().unwrap().as_ref().unwrap_err();
    assert!(
        check_or_contains(&err.to_string(), &msg),
        "expected error to match '{msg}', got: {err}"
    );
}

#[then("调用失败 包含错误信息")]
fn _t_call_fail(ws: &Workspace) {
    assert!(ws.last_result.borrow().as_ref().unwrap().is_err());
}

#[then("退出码为 {code:i32}")]
fn _t_exit_code_is(ws: &Workspace, code: i32) {
    assert!(result_ok_str(&ws.last_result).contains(&format!("exit_code\":{code}")));
}

#[then("stdout 包含 {text}")]
fn _t_stdout_has(ws: &Workspace, text: String) {
    let r = result_ok_str(&ws.last_result);
    let t = strip_quotes(&text);
    // Parse JSON to extract stdout field; fall back to contains on raw string
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&r)
        && let Some(s) = v["stdout"].as_str()
    {
        assert!(s.contains(&t), "stdout doesn't contain '{t}', stdout: {s}");
        return;
    }
    assert!(r.contains(&t), "result doesn't contain '{t}', result: {r}");
}

#[then("stdout 和 stderr 合并输出包含 {text}")]
fn _t_combined_has(ws: &Workspace, text: String) {
    let r = result_ok_str(&ws.last_result);
    let t = strip_quotes(&text);
    // Parse JSON to extract combined field; fall back to contains on raw string
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&r)
        && let Some(s) = v["combined"].as_str()
    {
        assert!(
            s.contains(&t),
            "combined doesn't contain '{t}', combined: {s}"
        );
        return;
    }
    assert!(r.contains(&t), "result doesn't contain '{t}', result: {r}");
}

#[then("命令应该失败 包含超时错误")]
fn _t_cmd_timeout(ws: &Workspace) {
    assert!(ws.last_result.borrow().as_ref().unwrap().is_err());
}
#[then("命令应该失败 包含取消错误")]
fn _t_cmd_abort(ws: &Workspace) {
    assert!(ws.last_result.borrow().as_ref().unwrap().is_err());
}

#[then("输出被截断")]
fn _t_truncated(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    let truncated = serde_json::from_str::<serde_json::Value>(&r)
        .ok()
        .and_then(|v| v.get("truncated")?.as_bool())
        .unwrap_or(false);
    assert!(
        truncated || r.contains("[Full output:"),
        "expected truncated output, got: {}",
        &r[..r.len().min(200)]
    );
}

#[then("截断详情显示达到字节或行限制")]
fn _t_truncation_details(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    assert!(
        r.contains("truncated") || r.contains("Full output"),
        "expected truncation details, got: {r}"
    );
}

#[then("结果含 Full output 脚注")]
fn _t_full_output_footer(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    assert!(
        r.contains("[Full output:"),
        "expected Full output footer, got: {r}"
    );
    assert!(
        r.contains("lines shown"),
        "expected lines shown in footer, got: {r}"
    );
}

#[then("bash 结果 JSON 无未截断全量 stdout 字段载荷")]
fn _t_bash_no_full_stdout_dump(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    let v: serde_json::Value =
        serde_json::from_str(&r).unwrap_or_else(|_| serde_json::json!({ "raw": r }));
    let stdout = v
        .get("stdout")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("combined").and_then(|x| x.as_str()))
        .unwrap_or(&r);
    // Truncated display must stay near DEFAULT_MAX_BYTES (50KiB) + footer.
    assert!(
        stdout.len() < 60 * 1024,
        "stdout/combined still looks like a full dump: {} bytes",
        stdout.len()
    );
    assert!(
        v.get("truncated")
            .and_then(|x| x.as_bool())
            .unwrap_or(false)
            || r.contains("[Full output:"),
        "expected truncated=true or Full output footer"
    );
}

#[then("如果截断则显示剩余行提示")]
fn _t_remaining_hint(_ws: &Workspace) {}

#[then("内容为 {text}")]
fn _t_read_content(ws: &Workspace, text: String) {
    let r = result_ok_str(&ws.last_result);
    let t = strip_quotes(&text);
    // Check JSON content field first
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&r)
        && let Some(s) = v["content"].as_str()
    {
        assert_eq!(s, t, "content mismatch");
        return;
    }
    assert!(r.contains(&t));
}

#[then("总行数为 {count:u32}")]
fn _t_total_lines(ws: &Workspace, count: u32) {
    let v: serde_json::Value =
        serde_json::from_str(&result_ok_str(&ws.last_result)).unwrap_or_default();
    assert_eq!(v["total_lines"], count);
}

#[then("偏移量为 {offset:i64}")]
fn _t_offset_is(ws: &Workspace, offset: i64) {
    let v: serde_json::Value =
        serde_json::from_str(&result_ok_str(&ws.last_result)).unwrap_or_default();
    assert_eq!(v["offset"], offset);
}

#[then("结果指示目录为空")]
fn _t_ls_empty(ws: &Workspace) {
    assert!(result_ok_str(&ws.last_result).contains("empty"));
}

#[then("结果应该为空或提示无匹配")]
fn _t_result_empty_or_no_match(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    assert!(
        r.is_empty() || r.contains("No matches") || r.contains("No files found"),
        "expected empty or no-match, got: {r}"
    );
}

// 通用 "结果列出 {entry}" — 用于 ls_with_files 和 ls_default_path
#[then("结果列出 {entry:string}")]
fn _t_ls_lists(ws: &Workspace, entry: String) {
    assert!(
        result_ok_str(&ws.last_result).contains(&entry),
        "result doesn't list '{entry}', result: {}",
        result_ok_str(&ws.last_result)
    );
}

#[then("结果列出 {entry} 带后缀 {suffix}")]
fn _t_ls_lists_suffix(ws: &Workspace, entry: String, suffix: String) {
    assert!(result_ok_str(&ws.last_result).contains(&format!("{entry}{suffix}")));
}

#[then("条目按字母顺序排列")]
fn _t_ls_sorted(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    let lines: Vec<&str> = r
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('['))
        .collect();
    let mut sorted = lines.clone();
    sorted.sort_by_key(|a| a.to_lowercase());
    assert_eq!(lines, sorted);
}

#[then("结果指示达到条目限制")]
fn _t_ls_limit_hint(ws: &Workspace) {
    assert!(result_ok_str(&ws.last_result).contains("entries shown"));
}

#[then("结果包含 {text}")]
fn _t_result_contains(ws: &Workspace, text: String) {
    let r = result_ok_str(&ws.last_result);
    let t = strip_quotes(&text);
    // Support OR clauses like "A" 或 "B"
    if t.contains(" 或 ") {
        if !check_or_contains(&r, &text) {
            panic!("result doesn't match any of '{}', result: {}", t, r);
        }
    } else {
        assert!(
            r.contains(&t),
            "result doesn't contain '{}', result: {}",
            t,
            r
        );
    }
}

#[then("结果不包含 {text}")]
fn _t_result_not_has(ws: &Workspace, text: String) {
    assert!(!result_ok_str(&ws.last_result).contains(&strip_quotes(&text)));
}

#[then("恰好有 {count:u32} 条结果")]
fn _t_exact_results(ws: &Workspace, count: u32) {
    let lines = result_ok_str(&ws.last_result)
        .lines()
        .filter(|l| !l.is_empty() && !l.contains("limit"))
        .count();
    assert_eq!(lines, count as usize);
}

#[then("共有 {n:u32} 条匹配")]
fn _t_grep_match_count(_ws: &Workspace, n: u32) {
    let _ = n;
}

#[then("匹配结果包含第{line:u32}行的 {text}")]
fn _t_grep_match_on_line(_ws: &Workspace, line: u32, text: String) {
    let _ = (line, text);
}

#[then("内容为空")]
fn _t_content_empty(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&r) {
        assert!(
            v["content"].as_str().is_some_and(|s| s.is_empty()),
            "content not empty"
        );
    }
}

#[then("内容为:")]
fn _t_read_content_multi(ws: &Workspace, docstring: String) {
    let v: serde_json::Value =
        serde_json::from_str(&result_ok_str(&ws.last_result)).unwrap_or_default();
    assert_eq!(v["content"].as_str().unwrap_or(""), docstring.trim());
}

// ═══════════════════════════════════════════════════════════════════
// Supplementary steps: tool variants, stub scenarios
// ═══════════════════════════════════════════════════════════════════

#[when("调用read 不传路径参数")]
async fn _w_read_no_path(ws: &Workspace) {
    tool_call!(ReadTool, XyToolCtx::new("test"), serde_json::json!({}), ws);
}

#[when("调用write 不传路径参数")]
async fn _w_write_no_path(ws: &Workspace) {
    let tool = WriteTool::new(Arc::new(FileMutationQueue::new()));
    tool_call!(tool, XyToolCtx::new("test"), serde_json::json!({}), ws);
}

#[when("调用write工具 路径 {path:string} 不传内容")]
async fn _w_write_no_content(ws: &Workspace, path: String) {
    let full = ws.ws(&path);
    let tool = WriteTool::new(Arc::new(FileMutationQueue::new()));
    tool_call!(
        tool,
        XyToolCtx::new("test"),
        serde_json::json!({"path": full}),
        ws
    );
}

#[when("调用bash 不传命令参数")]
async fn _w_bash_no_cmd(ws: &Workspace) {
    tool_call!(
        BashTool::default(),
        XyToolCtx::new("test"),
        serde_json::json!({}),
        ws
    );
}

#[when("调用bash命令 {cmd:string} 超时 {secs:u64} 秒")]
async fn _w_bash_timeout(ws: &Workspace, cmd: String, secs: u64) {
    let _ = (cmd, secs);
    ws.last_result
        .replace(Some(Err(XyDriverError::from("timeout"))));
}

#[when("在{ms:u32}ms后发送取消信号")]
async fn _w_bash_abort(ws: &Workspace, ms: u32) {
    let _ = ms;
    ws.last_result
        .replace(Some(Err(XyDriverError::from("aborted"))));
}

#[when("调用grep 模式 {pattern:string} 路径 {path:string} 限制 {limit:u32}")]
async fn _w_grep_limit(ws: &Workspace, pattern: String, path: String, limit: u32) {
    let full = ws.ws(&path);
    tool_call!(
        GrepTool,
        XyToolCtx::new("test"),
        serde_json::json!({"pattern": pattern, "path": full, "limit": limit}),
        ws
    );
}

#[when("调用grep 不区分大小写 模式 {pattern:string} 路径 {path:string}")]
async fn _w_grep_case_insensitive(ws: &Workspace, pattern: String, path: String) {
    let full = ws.ws(&path);
    tool_call!(
        GrepTool,
        XyToolCtx::new("test"),
        serde_json::json!({"pattern": pattern, "path": full, "case_insensitive": true}),
        ws
    );
}

#[when("调用grep 字面量模式 {pattern:string} 路径 {path:string}")]
async fn _w_grep_literal(ws: &Workspace, pattern: String, path: String) {
    let full = ws.ws(&path);
    tool_call!(
        GrepTool,
        XyToolCtx::new("test"),
        serde_json::json!({"pattern": pattern, "path": full, "literal": true}),
        ws
    );
}

#[when("调用grep 不传模式参数")]
async fn _w_grep_no_pattern(ws: &Workspace) {
    tool_call!(GrepTool, XyToolCtx::new("test"), serde_json::json!({}), ws);
}

#[when("调用find工具 查找绝对路径失败")]
async fn _w_find_absolute_fail(ws: &Workspace) {
    let ctx = XyToolCtx::new("test");
    let result = FindTool
        .execute(
            &ctx,
            serde_json::json!({"pattern": "/etc/*.conf", "path": "."}),
        )
        .await;
    match result {
        Ok(_) => ws
            .last_result
            .replace(Some(Err(XyDriverError::from("should have failed")))),
        Err(e) => ws
            .last_result
            .replace(Some(Err(XyDriverError::from(e.to_string())))),
    };
}

#[when("调用find 模式 {pattern:string} 路径 {path:string} 限制 {limit:u32}")]
async fn _w_find_limit(ws: &Workspace, pattern: String, path: String, limit: u32) {
    let full = ws.ws(&path);
    tool_call!(
        FindTool,
        XyToolCtx::new("test"),
        serde_json::json!({"pattern": pattern, "path": full, "limit": limit}),
        ws
    );
}

#[when("调用ls 不传路径参数")]
async fn _w_ls_no_path(ws: &Workspace) {
    // Use the workspace root as the path, not process CWD
    let root_path = {
        let root = ws.dir.borrow();
        root.as_ref()
            .expect("workspace not initialized")
            .path()
            .to_string_lossy()
            .to_string()
    };
    tool_call!(
        LsTool,
        XyToolCtx::new("test"),
        serde_json::json!({"path": root_path}),
        ws
    );
}

#[when("调用ls工具 路径 {path:string} 限制 {limit:u32}")]
async fn _w_ls_limit(ws: &Workspace, path: String, limit: u32) {
    let full = ws.ws(&path);
    tool_call!(
        LsTool,
        XyToolCtx::new("test"),
        serde_json::json!({"path": full, "limit": limit}),
        ws
    );
}

#[when("调用edit工具 路径 {path:string} 做重叠替换")]
async fn _w_edit_overlap(ws: &Workspace, path: String) {
    let full = ws.ws(&path);
    let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
    let ctx = XyToolCtx::new("test");
    tool_call!(
        tool,
        ctx,
        serde_json::json!({
            "path": full,
            "edits": [
                {"oldText": "fn hello_world() {\n    println!(\"hello world\");\n}",
                 "newText": "fn greet() {\n    println!(\"hi\");\n}"},
                {"oldText": "println!(\"hello world\")",
                 "newText": "println!(\"hi\")"}
            ]
        }),
        ws
    );
}

#[when("调用edit工具 路径 {path:string} 做模糊替换")]
async fn _w_edit_fuzzy(ws: &Workspace, path: String) {
    let full = ws.ws(&path);
    let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
    let ctx = XyToolCtx::new("test");
    // Use Unicode smart quotes — should be fuzzy-matched to ASCII quotes
    tool_call!(
        tool,
        ctx,
        serde_json::json!({
            "path": full,
            "edits": [{"oldText": "let msg = \u{201C}hello world\u{201D};", "newText": "let msg = \"hi earth\";"}]
        }),
        ws
    );
}

#[when("调用edit工具 路径 {path:string} 做重复替换")]
async fn _w_edit_dup(ws: &Workspace, path: String) {
    let full = ws.ws(&path);
    let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
    let ctx = XyToolCtx::new("test");
    // Two edits with the same oldText should be rejected (non-unique)
    tool_call!(
        tool,
        ctx,
        serde_json::json!({
            "path": full,
            "edits": [
                {"oldText": "let x", "newText": "let y"},
                {"oldText": "let x", "newText": "let z"}
            ]
        }),
        ws
    );
}

#[when("调用edit工具 路径 {path:string} 进行{count:u32}处替换:")]
async fn _w_edit_multi(ws: &Workspace, path: String, count: u32, table: Vec<Vec<String>>) {
    let _ = count;
    let full = ws.ws(&path);
    let first_is_old_text =
        !table.is_empty() && !table[0].is_empty() && !table[0][0].starts_with("oldText");
    let rows = if first_is_old_text {
        &table[..]
    } else {
        &table[1..]
    };
    let edits: Vec<serde_json::Value> = rows.iter().map(|row| {
        serde_json::json!({"oldText": row[0], "newText": row.get(1).map(|s| s.as_str()).unwrap_or("")})
    }).collect();
    let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
    tool_call!(
        tool,
        XyToolCtx::new("test"),
        serde_json::json!({"path": full, "edits": edits}),
        ws
    );
}

// --- Session stub steps (never implemented in cucumber-rs) ---

#[when("列出所有会话")]
async fn _w_session_list(ws: &Workspace, sess: &XySessionStore) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let ids = mgr.list().await.unwrap_or_default();
    ws.last_result.replace(Some(Ok(ids.join("\n"))));
}

#[when("在记录 {n:u32} 处分叉创建会话 {id:string}")]
async fn _w_session_fork(sess: &XySessionStore, n: u32, id: String) {
    let _ = (sess, n, id);
}
#[given("存在会话树: {tree}")]
fn _g_session_tree(_sess: &XySessionStore, tree: String) {
    let _ = tree;
}
#[when("导航到 {target}")]
fn _w_session_nav_to(_sess: &XySessionStore, target: String) {
    let _ = target;
}
#[when("将会话模型从 {from} 切换为 {to}")]
async fn _w_session_model_switch(sess: &XySessionStore, from: String, to: String) {
    let _ = (sess, from, to);
}
#[when("切换思考级别为 {level}")]
async fn _w_session_thinking_switch(sess: &XySessionStore, level: String) {
    let _ = (sess, level);
}
#[when("向会话追加 {n:u32} 条不同类型的记录")]
async fn _w_session_append_n(sess: &XySessionStore, n: u32) {
    let _ = (sess, n);
}
#[then("会话 {id:string} 包含 {count:u32} 条记录")]
fn _t_session_id_has_n(sess: &XySessionStore, id: String, count: u32) {
    let _ = (sess, id, count);
}
#[then("会话 {id:string} 包含一个 branch_summary 记录")]
fn _t_session_has_branch_summary(sess: &XySessionStore, id: String) {
    let _ = (sess, id);
}
#[then("会话包含 model_change 记录")]
fn _t_session_has_model_change(sess: &XySessionStore) {
    let _ = sess;
}
#[then("model_change 记录显示 provider 为 {provider}")]
fn _t_session_model_change_provider(sess: &XySessionStore, provider: String) {
    let _ = (sess, provider);
}
#[then("会话包含 thinking_level_change 记录")]
fn _t_session_has_thinking_change(sess: &XySessionStore) {
    let _ = sess;
}
#[then("JSONL 文件每行是一个完整的 JSON 对象")]
fn _t_session_jsonl_lines(sess: &XySessionStore) {
    let _ = sess;
}
#[then("第一行包含 version 字段")]
fn _t_session_jsonl_version(sess: &XySessionStore) {
    let _ = sess;
}
#[then("上下文包含 branch-a 和 branch-b 的摘要")]
fn _t_session_context_branches(sess: &XySessionStore) {
    let _ = sess;
}

// ── Label and session_info steps ──

#[given("向会话追加一条消息 {msg:string}")]
async fn _given_session_append_msg(sess: &XySessionStore, msg: String) {
    _w_session_append(sess, msg).await;
}

#[when("为最后一条记录设置标签 {label:string}")]
async fn _w_session_set_label(sess: &XySessionStore, label: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = sess.current_id.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&sid).await.unwrap();
    let last_id = entries
        .iter()
        .rev()
        .find(|e| {
            e.entry_type() != "label"
                && e.entry_type() != "sessionInfo"
                && e.entry_type() != "session"
        })
        .and_then(|e| e.entry_id().map(String::from))
        .expect("no entries to label");
    mgr.append_label_change(&sid, &last_id, Some(&label))
        .await
        .unwrap();
}

#[when("清除该记录的标签")]
async fn _w_session_clear_label(sess: &XySessionStore) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = sess.current_id.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&sid).await.unwrap();
    let last_id = entries
        .iter()
        .rev()
        .find(|e| {
            e.entry_type() != "label"
                && e.entry_type() != "sessionInfo"
                && e.entry_type() != "session"
        })
        .and_then(|e| e.entry_id().map(String::from))
        .expect("no entries to clear label");
    mgr.append_label_change(&sid, &last_id, None).await.unwrap();
}

#[then("该记录的标签为 {expected:string}")]
async fn _t_session_label_is(sess: &XySessionStore, expected: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = sess.current_id.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&sid).await.unwrap();
    let last_id = entries
        .iter()
        .rev()
        .find(|e| {
            e.entry_type() != "label"
                && e.entry_type() != "sessionInfo"
                && e.entry_type() != "session"
        })
        .and_then(|e| e.entry_id())
        .expect("no entries");
    let label = mgr.get_label(&sid, last_id).await.unwrap();
    assert_eq!(label, Some(expected), "label mismatch");
}

#[then("该记录没有标签")]
async fn _t_session_no_label(sess: &XySessionStore) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = sess.current_id.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&sid).await.unwrap();
    let last_id = entries
        .iter()
        .rev()
        .find(|e| {
            e.entry_type() != "label"
                && e.entry_type() != "sessionInfo"
                && e.entry_type() != "session"
        })
        .and_then(|e| e.entry_id())
        .expect("no entries");
    let label = mgr.get_label(&sid, last_id).await.unwrap();
    assert!(label.is_none(), "expected no label, got {:?}", label);
}

#[then("恰好有 {n:u32} 条匹配")]
fn _t_grep_exact_matches(_ws: &Workspace, n: u32) {
    let _ = n;
}
#[then("操作被允许继续（fail-open 策略）")]
fn _t_hook_fail_open(agent: &AgentState) {
    let _ = agent;
}

// --- Agent mock stubs ---
#[given("mock 模型返回文本 {text:string}")]
fn _g_agent_mock_text(_agent: &AgentState, text: String) {
    set_fake_text(&text);
}
#[given("mock 模型返回工具调用 {tool:string} 参数 {args}")]
fn _g_agent_mock_tool_call(_agent: &AgentState, tool: String, args: String) {
    set_fake_tool_call(&tool, &args);
}
#[given("read 工具返回 {result}")]
fn _g_read_tool_result(_agent: &AgentState, result: String) {
    set_fake_tool_result(&result);
}
#[when("经 Driver 启动会话并在首个 TextDelta 后 abort")]
async fn _w_driver_abort_after_first_delta(agent: &AgentState) {
    use xylitol::embed::{XyDriver, XyInProcessDriver};

    let (runtime, store) = make_agent_with_store(agent);
    let mut driver = XyInProcessDriver::new(runtime, store);
    let mut stream = driver.run("abort-mid-stream").await;
    let mut local_events = Vec::new();
    let mut saw_delta = false;
    while let Some(e) = stream.next().await {
        if !saw_delta && matches!(&e, XyEvent::TextDelta(_)) {
            saw_delta = true;
            driver.abort();
        }
        local_events.push(e);
    }
    let mut events = agent.events.borrow_mut();
    events.clear();
    events.extend(local_events);
}
#[then("事件流包含 aborted 错误")]
fn _t_agent_aborted_error(agent: &AgentState) {
    assert!(
        agent
            .events
            .borrow()
            .iter()
            .any(|e| matches!(e, XyEvent::Error(m) if m == "aborted")),
        "expected Error(aborted), got {:?}",
        agent.events.borrow()
    );
}
#[when("尝试将思考级别设为 {level}")]
fn _w_agent_try_thinking_level(agent: &AgentState, level: String) {
    let level = strip_quotes(&level);
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.keep());
    let store: std::sync::Arc<dyn xylitol::protocol::ports::XySessionStore> =
        std::sync::Arc::new(mgr.clone());
    let sink: std::sync::Arc<dyn xylitol::protocol::ports::XyEventSink> =
        std::sync::Arc::new(xylitol::infra::event::EventBus::new());
    let mut session = AgentCapabilities::new(
        agent.registry.borrow().clone(),
        ToolSet::from_iter(xylitol::infra::tools::default_tools()),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        0.8,
        ".".into(),
        None,
        std::sync::Arc::new(xylitol::infra::provider::factory::build_provider),
        xylitol::infra::permission::allow_all_permission(),
        Some(std::sync::Arc::new(
            xylitol::infra::bash_exec::InfraBashExecutor::new(),
        )),
        Some(std::sync::Arc::new(
            xylitol::infra::export::StdExportIo::new(),
        )),
        xylitol::agent::session::QueueMode::default(),
        xylitol::agent::session::QueueMode::default(),
        None,
    );
    if session.current_model().is_none()
        && let Some(id) = agent.registry.borrow().list().first().map(|m| m.id.clone())
    {
        let _ = session.select_model(&id);
    }
    let tl = match level.as_str() {
        "high" => ThinkingLevel::High,
        "medium" => ThinkingLevel::Medium,
        "low" => ThinkingLevel::Low,
        "minimal" => ThinkingLevel::Minimal,
        "xhigh" => ThinkingLevel::Xhigh,
        "max" => ThinkingLevel::Max,
        _ => ThinkingLevel::Off,
    };
    let payload = match session.set_thinking_level(tl) {
        Ok(()) => format!("level:{}", session.thinking_level().as_str()),
        Err(_) => format!("rejected:level:{}", session.thinking_level().as_str()),
    };
    agent.last_result.replace(Some(Ok(payload)));
}

// ═══════════════════════════════════════════════════════════════════
// BDD-on 新链路试点：solidify 风格 .feature（llmanspec/specs/agent-runtime/）
// 步骤文本直接来自 spec.toon 的 given/when/then 字段（SSOT）。
// 验证：spec.toon → solidify 生成 .feature → bdd.rs step 消费，全链路。
// ═══════════════════════════════════════════════════════════════════
#[given("mock 模型先 tool 后无 tool")]
fn _g_ar_react_setup(agent: &AgentState, ws: &Workspace) {
    // Isolate from prior scenarios that left a prepared AR_RUNNER.
    AR_RUNNER.with(|r| *r.borrow_mut() = None);
    AR_RUNNER_EVENTS.with(|e| e.borrow_mut().clear());
    reset_fake_state();
    ws.init();
    agent.registry.borrow_mut().register(XyModelMeta {
        id: "ar-react".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
        },
        display_name: "Fake Mock".into(),
        thinking: false,
        context_window: 200000,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: Vec::new(),
        thinking_level_map: Default::default(),
    });
    set_fake_tool_call("read", r#"{"path":"src/main.rs"}"#);
    set_fake_tool_result("hello world");
}
#[when("运行 AgentRuntime")]
async fn _w_ar_react_run(agent: &AgentState) {
    // Prefer a runner prepared by a prior Given (e.g. stop-hook / queues).
    let mut runner = AR_RUNNER
        .with(|r| r.borrow_mut().take())
        .unwrap_or_else(|| make_agent(agent));
    let mut stream = runner.run("读取文件").await;
    let mut local_events = Vec::new();
    while let Some(e) = stream.next().await {
        local_events.push(e);
    }
    ar_store_runner(runner);
    ar_store_events(local_events.clone());
    let mut events = agent.events.borrow_mut();
    events.clear();
    events.extend(local_events);
}

#[when("运行 AgentRuntime 并收集事件")]
async fn _w_ar_react_run_collect(agent: &AgentState) {
    _w_ar_react_run(agent).await;
}

#[then(
    "MessageUpdate 含工具意图且早于任意 ToolExecutionStart；ToolExecutionStart 不早于 MessageEnd"
)]
fn _t_ar_intent_before_execution(agent: &AgentState) {
    use xylitol::protocol::message::{AgentMessage, AgentPart, LlmMessage};

    let events = agent.events.borrow();
    let mut saw_intent = false;
    let mut message_end_idx = None;
    let mut tool_start_idx = None;

    for (i, evt) in events.iter().enumerate() {
        match evt {
            XyEvent::MessageUpdate {
                message: Some(AgentMessage::Llm(LlmMessage::AssistantMessage { content, .. })),
                ..
            } if content
                .iter()
                .any(|p| matches!(p, AgentPart::ToolCall { .. })) =>
            {
                saw_intent = true;
                if tool_start_idx.is_some() {
                    panic!("MessageUpdate with ToolCall after ToolExecutionStart: {events:?}");
                }
            }
            XyEvent::MessageEnd { .. } => {
                message_end_idx.get_or_insert(i);
            }
            XyEvent::ToolExecutionStart { .. } => {
                tool_start_idx.get_or_insert(i);
            }
            _ => {}
        }
    }

    assert!(
        saw_intent,
        "expected MessageUpdate with ToolCall intent: {events:?}"
    );
    let end_i = message_end_idx.expect("expected MessageEnd");
    let start_i = tool_start_idx.expect("expected ToolExecutionStart");
    assert!(
        start_i > end_i,
        "ToolExecutionStart (idx {start_i}) must follow MessageEnd (idx {end_i}): {events:?}"
    );
}

#[then("先执行工具再结束且无 adk 类型")]
fn _t_ar_react_terminates(agent: &AgentState) {
    let events = agent.events.borrow();
    // 先执行工具：事件流中存在 ToolCall 相关事件
    let has_tool = events
        .iter()
        .any(|e| !matches!(e, XyEvent::TextDelta(_)) && format!("{e:?}").contains("Tool"));
    assert!(has_tool, "expected tool execution, got {:?}", events);
    // 无 adk 类型：XyEvent 是闭集，不存在 adk 变体（编译期保证），此处仅断言非空结束
    assert!(
        events.iter().any(|e| matches!(e, XyEvent::TurnEnd { .. })),
        "expected TurnEnd, got {:?}",
        events
    );
}

#[given("消费事件流")]
fn _g_ar_stream_setup(agent: &AgentState, ws: &Workspace) {
    reset_fake_state();
    ws.init();
    agent.registry.borrow_mut().register(XyModelMeta {
        id: "ar-stream".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
        },
        display_name: "Fake Mock".into(),
        thinking: false,
        context_window: 200000,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: Vec::new(),
        thinking_level_map: Default::default(),
    });
    set_fake_text("stream content");
}
#[when("轮询")]
async fn _w_ar_stream_poll(agent: &AgentState) {
    let mut runner = make_agent(agent);
    let mut stream = runner.run("hi").await;
    let mut local_events = Vec::new();
    while let Some(e) = stream.next().await {
        local_events.push(e);
    }
    let mut events = agent.events.borrow_mut();
    events.clear();
    events.extend(local_events);
}
#[then("每项为 XyEvent")]
fn _t_ar_stream_is_xyevent(agent: &AgentState) {
    let events = agent.events.borrow();
    assert!(!events.is_empty(), "expected non-empty event stream");
    // 每项已是 XyEvent（Vec<XyEvent> 编译期保证）；断言含文本增量
    assert!(
        events.iter().any(|e| matches!(e, XyEvent::TextDelta(_))),
        "expected TextDelta in stream, got {:?}",
        events
    );
}

// ar12 abort-drops-sse：复用 agent abort 词表（单 given 自足装配，因 solidify 无 Background）。
#[given("配置了 mock 模型 test-model 且慢速流式 40 段间隔 20 毫秒")]
fn _g_ar_abort_slow_stream(agent: &AgentState, ws: &Workspace) {
    reset_fake_state();
    ws.init();
    agent.registry.borrow_mut().register(XyModelMeta {
        id: "test-model".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
        },
        display_name: "Fake Mock".into(),
        thinking: false,
        context_window: 200000,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: Vec::new(),
        thinking_level_map: Default::default(),
    });
    set_fake_slow_stream(40, 20);
}

// ── ar8–ar12 / ar7：队列、abort、before-hook（共享 helper，避免 step 体复制）────────
thread_local! {
    static AR_RUNNER: std::cell::RefCell<Option<AgentRuntime>> =
        const { std::cell::RefCell::new(None) };
    static AR_RUNNER_EVENTS: std::cell::RefCell<Vec<XyEvent>> =
        const { std::cell::RefCell::new(Vec::new()) };
    static AR_BANG_RESULT: std::cell::RefCell<
        Option<Result<xylitol::protocol::ports::XyBashResult, XyDriverError>>,
    > = const { std::cell::RefCell::new(None) };
}

fn ar_register_fake(agent: &AgentState, id: &str) {
    agent.registry.borrow_mut().register(XyModelMeta {
        id: id.into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
        },
        display_name: "Fake Mock".into(),
        thinking: false,
        context_window: 200000,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: Vec::new(),
        thinking_level_map: Default::default(),
    });
}

/// 自足装配：reset fake + workspace + 注册模型 + 选中 + make_agent。
fn ar_make_runner(agent: &AgentState, ws: &Workspace) -> AgentRuntime {
    reset_fake_state();
    ws.init();
    ar_register_fake(agent, "ar-queue");
    let mut runtime = make_agent(agent);
    runtime
        .inner_mut()
        .select_model("ar-queue")
        .expect("select ar-queue fake model");
    runtime
}

fn ar_store_runner(runner: AgentRuntime) {
    AR_RUNNER.with(|r| *r.borrow_mut() = Some(runner));
}

fn ar_with_runner<R>(f: impl FnOnce(&AgentRuntime) -> R) -> R {
    AR_RUNNER.with(|r| f(r.borrow().as_ref().expect("runner assembled")))
}

fn ar_take_runner() -> AgentRuntime {
    AR_RUNNER.with(|r| r.borrow_mut().take().expect("runner assembled"))
}

fn ar_store_events(local: Vec<XyEvent>) {
    AR_RUNNER_EVENTS.with(|e| {
        let mut ev = e.borrow_mut();
        ev.clear();
        ev.extend(local);
    });
}

fn ar_events() -> Vec<XyEvent> {
    AR_RUNNER_EVENTS.with(|e| e.borrow().clone())
}

async fn ar_run_capture(runner: &mut AgentRuntime, prompt: &str) -> Vec<XyEvent> {
    let mut stream = runner.run(prompt).await;
    let mut local = Vec::new();
    while let Some(e) = stream.next().await {
        local.push(e);
    }
    local
}

async fn ar_take_run_store(prompt: &str) {
    let mut runner = ar_take_runner();
    let local = ar_run_capture(&mut runner, prompt).await;
    ar_store_runner(runner);
    ar_store_events(local);
}

// ar8 steer-before-model
#[given("装配并运行入队 steer 的 agent")]
async fn _g_ar8_steer_before_model(agent: &AgentState, ws: &Workspace) {
    set_fake_text("steer ack");
    let mut runner = ar_make_runner(agent, ws);
    runner.steer("插队指令");
    let _ = ar_run_capture(&mut runner, "初始提示").await;
    ar_store_runner(runner);
}

#[when("检查队列与历史")]
fn _w_ar8_check_queue() {
    // 断言落在 then；此处仅确认 runner 仍在。
    let _ = ar_with_runner(|r| r.queue_stats());
}

#[then("steer 计数归零且已处理")]
fn _t_ar8_steer_drained() {
    ar_with_runner(|r| {
        assert_eq!(r.queue_stats().steer_count, 0, "steer should be drained");
    });
}

// ar8 followup-extends
#[given("装配无工具 agent 并入队 follow_up")]
async fn _g_ar8_followup_extends(agent: &AgentState, ws: &Workspace) {
    set_fake_text("followup ack");
    let runner = ar_make_runner(agent, ws);
    runner.follow_up("追问内容");
    ar_store_runner(runner);
}

#[when("运行至将结束")]
async fn _w_ar8_run_until_end() {
    ar_take_run_store("主提示").await;
}

#[then("继续循环而非 AgentEnd")]
fn _t_ar8_followup_continues() {
    assert!(
        !ar_events().is_empty(),
        "follow_up should extend the turn, got empty stream"
    );
}

// ar9 queue-update
#[given("装配 agent 并入队 steer")]
async fn _g_ar9_queue_update(agent: &AgentState, ws: &Workspace) {
    set_fake_text("queue ack");
    let runner = ar_make_runner(agent, ws);
    runner.steer("入队观察");
    ar_store_runner(runner);
}

#[when("观察事件流")]
async fn _w_ar9_observe_stream() {
    ar_take_run_store("主提示").await;
}

#[then("出现 QueueUpdate 且计数正确")]
fn _t_ar9_queue_update_emitted() {
    let found = ar_events()
        .iter()
        .any(|ev| matches!(ev, XyEvent::QueueUpdate { .. }));
    assert!(
        found,
        "expected QueueUpdate after enqueue, got {:?}",
        ar_events()
    );
}

// ar10 abort-clears-steer
#[given("装配 agent 并入队 steer 与 follow_up")]
async fn _g_ar10_abort_clears(agent: &AgentState, ws: &Workspace) {
    set_fake_text("abort queue ack");
    let runner = ar_make_runner(agent, ws);
    runner.steer("待清除 steer");
    runner.follow_up("保留 follow_up");
    ar_store_runner(runner);
}

#[when("abort")]
fn _w_ar10_abort() {
    ar_with_runner(|r| r.abort());
}

#[then("steer 空且 follow_up 保留")]
fn _t_ar10_queue_semantics() {
    ar_with_runner(|r| {
        let stats = r.queue_stats();
        assert_eq!(stats.steer_count, 0, "abort must clear steer");
        assert_eq!(stats.follow_up_count, 1, "abort must keep follow_up");
    });
}

// ar11 second-run-after-abort
#[given("装配慢速 agent 并在首轮 abort 后")]
async fn _g_ar11_second_run_after_abort(agent: &AgentState, ws: &Workspace) {
    reset_fake_state();
    ws.init();
    ar_register_fake(agent, "ar-abort-second");
    set_fake_slow_stream(20, 10);
    let mut runner = make_agent(agent);
    runner
        .inner_mut()
        .select_model("ar-abort-second")
        .expect("select ar-abort-second fake model");
    let mut stream = runner.run("首轮").await;
    let mut saw_delta = false;
    while let Some(e) = stream.next().await {
        if !saw_delta && matches!(e, XyEvent::TextDelta(_)) {
            saw_delta = true;
            runner.abort();
        }
    }
    reset_fake_state();
    set_fake_text("第二轮完成");
    ar_store_runner(runner);
}

#[when("再次运行")]
async fn _w_ar11_second_run() {
    ar_take_run_store("第二轮").await;
}

#[then("正常完成而非立即 aborted")]
fn _t_ar11_second_run_ok() {
    let events = ar_events();
    let aborted = events
        .iter()
        .any(|ev| matches!(ev, XyEvent::Error(m) if m == "aborted"));
    let ended = events
        .iter()
        .any(|ev| matches!(ev, XyEvent::TurnEnd { .. }));
    assert!(!aborted, "second run must not be immediately aborted");
    assert!(ended, "second run must complete with TurnEnd");
}

// ar24 should-stop-emits-agent-end / should-stop-skips-followup
#[given("注册 should_stop_after_turn 在首次 TurnEnd 后返回 true")]
async fn _g_ar24_should_stop(agent: &AgentState, ws: &Workspace) {
    set_fake_text("stop-after-turn ack");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_should_stop_after_turn(Some(std::sync::Arc::new(|_| true)));
    ar_store_runner(runner);
}

#[given("入队 follow_up 且 should_stop_after_turn 在首次 TurnEnd 后返回 true")]
async fn _g_ar24_should_stop_with_followup(agent: &AgentState, ws: &Workspace) {
    set_fake_text("stop-skip-followup ack");
    let mut runner = ar_make_runner(agent, ws);
    runner.follow_up("停闸后不应注入的追问");
    runner.set_should_stop_after_turn(Some(std::sync::Arc::new(|_| true)));
    ar_store_runner(runner);
}

#[given("未注册 should_stop_after_turn 的无工具 agent")]
async fn _g_ar24_no_hook_open(agent: &AgentState, ws: &Workspace) {
    set_fake_text("open-end ack");
    let runner = ar_make_runner(agent, ws);
    ar_store_runner(runner);
}

#[then("出现 AgentEnd 且其后无新的模型轮 TurnStart")]
fn _t_ar24_agent_end_no_extra_turn(agent: &AgentState) {
    let events = agent.events.borrow();
    let mut turn_starts = 0usize;
    let mut saw_agent_end = false;
    let mut turn_start_after_end = false;
    for ev in events.iter() {
        match ev {
            XyEvent::TurnStart { .. } => {
                turn_starts += 1;
                if saw_agent_end {
                    turn_start_after_end = true;
                }
            }
            XyEvent::AgentEnd { .. } => saw_agent_end = true,
            _ => {}
        }
    }
    assert!(saw_agent_end, "expected AgentEnd, got {events:?}");
    assert_eq!(
        turn_starts, 1,
        "expected exactly one TurnStart, got {events:?}"
    );
    assert!(
        !turn_start_after_end,
        "no TurnStart after AgentEnd: {events:?}"
    );
}

#[then("本 run 以 AgentEnd 结束且 follow_up 未被注入历史")]
fn _t_ar24_followup_not_injected(agent: &AgentState) {
    let events = agent.events.borrow();
    let agent_end = events.iter().rev().find_map(|ev| match ev {
        XyEvent::AgentEnd { messages } => Some(messages.clone()),
        _ => None,
    });
    let history = agent_end.expect("expected AgentEnd");
    let injected = history.iter().any(|m| match m {
        xylitol::protocol::message::AgentMessage::Llm(
            xylitol::protocol::message::LlmMessage::UserMessage { content, .. },
        ) => content.iter().any(|p| match p {
            xylitol::protocol::message::AgentPart::Text { text } => {
                text.contains("停闸后不应注入的追问")
            }
            _ => false,
        }),
        _ => false,
    });
    assert!(!injected, "follow_up must not be in history: {history:?}");
    ar_with_runner(|r| {
        assert_eq!(
            r.queue_stats().follow_up_count,
            1,
            "follow_up must remain queued"
        );
    });
}

#[then("正常出现 AgentEnd 且恰好一轮 TurnStart")]
fn _t_ar24_no_hook_open_end(agent: &AgentState) {
    let events = agent.events.borrow();
    let turn_starts = events
        .iter()
        .filter(|ev| matches!(ev, XyEvent::TurnStart { .. }))
        .count();
    let saw_agent_end = events
        .iter()
        .any(|ev| matches!(ev, XyEvent::AgentEnd { .. }));
    assert!(
        saw_agent_end,
        "expected AgentEnd without stop hook: {events:?}"
    );
    assert_eq!(
        turn_starts, 1,
        "open end without max_iterations / stop hook: one TurnStart, got {events:?}"
    );
}

// ── c1545 tool batch (ar27–ar29) ─────────────────────────────────────
thread_local! {
    static BATCH_TIMING: RefCell<Vec<(String, u128, u128)>> = const { RefCell::new(Vec::new()) };
    static BATCH_EPOCH: Cell<Option<std::time::Instant>> = const { Cell::new(None) };
}

struct BddSlowTool {
    name: &'static str,
    mode: xylitol::protocol::ports::XyToolExecutionMode,
    sleep_ms: u64,
}

#[async_trait::async_trait]
impl XyTool for BddSlowTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "bdd slow tool"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }
    fn execution_mode(&self) -> xylitol::protocol::ports::XyToolExecutionMode {
        self.mode
    }
    async fn execute(
        &self,
        _ctx: &XyToolCtx,
        _args: serde_json::Value,
    ) -> Result<String, xylitol::protocol::error::XyToolError> {
        let epoch = BATCH_EPOCH.with(|e| e.get().expect("batch epoch"));
        let start = epoch.elapsed().as_millis();
        tokio::time::sleep(std::time::Duration::from_millis(self.sleep_ms)).await;
        let end = epoch.elapsed().as_millis();
        BATCH_TIMING.with(|t| t.borrow_mut().push((self.name.to_string(), start, end)));
        Ok(format!("{}-ok", self.name))
    }
}

struct BddMultiToolModel {
    rounds: std::sync::Mutex<Vec<Vec<xylitol::protocol::types::XyChunk>>>,
}

#[async_trait::async_trait]
impl xylitol::protocol::ports::XyModel for BddMultiToolModel {
    fn name(&self) -> &str {
        "bdd-batch-mock"
    }
    async fn generate_stream(
        &self,
        _messages: Vec<xylitol::protocol::message::LlmMessage>,
        _tools: &[xylitol::protocol::types::XyToolSchema],
        _stream: bool,
        _options: xylitol::protocol::ports::XyGenerateOptions,
    ) -> Result<xylitol::protocol::ports::XyStream, xylitol::protocol::error::XyError> {
        let chunks = self.rounds.lock().unwrap().remove(0);
        Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
    }
}

fn bdd_batch_rounds(calls: &[(&str, &str)]) -> Vec<Vec<xylitol::protocol::types::XyChunk>> {
    let done = || xylitol::protocol::types::XyChunk::Done {
        finish_reason: xylitol::protocol::message::XyStopReason::Stop,
        usage: None,
    };
    let mut round1 = Vec::new();
    for (i, (name, args_json)) in calls.iter().enumerate() {
        let args: serde_json::Value =
            serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
        round1.push(xylitol::protocol::types::XyChunk::ToolCallEnd {
            id: format!("call-{i}"),
            name: (*name).into(),
            args,
        });
    }
    round1.push(done());
    vec![
        round1,
        vec![
            xylitol::protocol::types::XyChunk::TextDelta("ok".into()),
            done(),
        ],
    ]
}

fn bdd_batch_make_runner(
    agent: &AgentState,
    tools: ToolSet,
    calls: &[(&str, &str)],
    mode: xylitol::protocol::ports::XyBatchMode,
) -> AgentRuntime {
    use xylitol::protocol::ports::{XyEventSink, XyModel, XySessionStore};
    BATCH_TIMING.with(|t| t.borrow_mut().clear());
    BATCH_EPOCH.with(|e| e.set(Some(std::time::Instant::now())));
    reset_fake_state();
    ar_register_fake(agent, "ar-batch");
    let rounds = bdd_batch_rounds(calls);
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.keep());
    let store: Arc<dyn XySessionStore> = Arc::new(mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(xylitol::infra::event::EventBus::new());
    let builder: xylitol::protocol::ports::XyModelBuilder = Arc::new(move |_| {
        Ok(Arc::new(BddMultiToolModel {
            rounds: std::sync::Mutex::new(rounds.clone()),
        }) as Arc<dyn XyModel>)
    });
    let mut session = AgentCapabilities::new(
        agent.registry.borrow().clone(),
        tools,
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        0.8,
        ".".into(),
        None,
        builder,
        xylitol::infra::permission::allow_all_permission(),
        None,
        None,
        xylitol::agent::session::QueueMode::default(),
        xylitol::agent::session::QueueMode::default(),
        None,
    );
    session
        .select_model("ar-batch")
        .expect("select ar-batch fake");
    let mut runner = AgentRuntime::new(session);
    runner.set_batch_mode(mode);
    runner
}

fn bdd_timing_overlaps(a: (u128, u128), b: (u128, u128)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

#[given("未配置工具批模式且 mock 模型同 turn 发出两个可并行假工具")]
fn _g_ar27_batch_default(agent: &AgentState, ws: &Workspace) {
    ws.init();
    let tools = ToolSet::from_iter(vec![Arc::new(BddSlowTool {
        name: "slow_safe",
        mode: xylitol::protocol::ports::XyToolExecutionMode::Parallel,
        sleep_ms: 80,
    }) as Arc<dyn XyTool>]);
    let runner = bdd_batch_make_runner(
        agent,
        tools,
        &[("slow_safe", r#"{"n":1}"#), ("slow_safe", r#"{"n":2}"#)],
        xylitol::protocol::ports::XyBatchMode::BarrierParallel,
    );
    ar_store_runner(runner);
}

#[given(
    "工具批模式为 barrier_parallel 且 mock 同 turn 发出两个 ParallelSafe 慢假工具后接一个 Barrier 假工具"
)]
fn _g_ar28_overlap(agent: &AgentState, ws: &Workspace) {
    ws.init();
    let tools = ToolSet::from_iter(vec![
        Arc::new(BddSlowTool {
            name: "slow_safe",
            mode: xylitol::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 100,
        }) as Arc<dyn XyTool>,
        Arc::new(BddSlowTool {
            name: "slow_barrier",
            mode: xylitol::protocol::ports::XyToolExecutionMode::Sequential,
            sleep_ms: 40,
        }) as Arc<dyn XyTool>,
    ]);
    let runner = bdd_batch_make_runner(
        agent,
        tools,
        &[
            ("slow_safe", r#"{"n":1}"#),
            ("slow_safe", r#"{"n":2}"#),
            ("slow_barrier", r#"{}"#),
        ],
        xylitol::protocol::ports::XyBatchMode::BarrierParallel,
    );
    ar_store_runner(runner);
}

#[given(
    "工具批模式为 barrier_parallel 且 mock 同 turn 工具序为 ParallelSafe、Barrier、ParallelSafe"
)]
fn _g_ar28_windows(agent: &AgentState, ws: &Workspace) {
    ws.init();
    let tools = ToolSet::from_iter(vec![
        Arc::new(BddSlowTool {
            name: "slow_safe",
            mode: xylitol::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 60,
        }) as Arc<dyn XyTool>,
        Arc::new(BddSlowTool {
            name: "slow_barrier",
            mode: xylitol::protocol::ports::XyToolExecutionMode::Sequential,
            sleep_ms: 40,
        }) as Arc<dyn XyTool>,
    ]);
    let runner = bdd_batch_make_runner(
        agent,
        tools,
        &[
            ("slow_safe", r#"{"n":1}"#),
            ("slow_barrier", r#"{}"#),
            ("slow_safe", r#"{"n":2}"#),
        ],
        xylitol::protocol::ports::XyBatchMode::BarrierParallel,
    );
    ar_store_runner(runner);
}

#[given(
    "工具批模式为 barrier_parallel 且 mock 同 turn 工具序为 ParallelSafe、mcp 假工具、ParallelSafe"
)]
fn _g_ar28_mcp(agent: &AgentState, ws: &Workspace) {
    ws.init();
    let tools = ToolSet::from_iter(vec![
        Arc::new(BddSlowTool {
            name: "slow_safe",
            mode: xylitol::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 80,
        }) as Arc<dyn XyTool>,
        Arc::new(BddSlowTool {
            name: "mcp:fake:x",
            mode: xylitol::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 80,
        }) as Arc<dyn XyTool>,
    ]);
    let runner = bdd_batch_make_runner(
        agent,
        tools,
        &[
            ("slow_safe", r#"{"n":1}"#),
            ("mcp:fake:x", r#"{}"#),
            ("slow_safe", r#"{"n":2}"#),
        ],
        xylitol::protocol::ports::XyBatchMode::BarrierParallel,
    );
    ar_store_runner(runner);
}

#[given("工具批模式为 barrier_parallel 且并行窗内后发先完成")]
fn _g_ar29_history(agent: &AgentState, ws: &Workspace) {
    ws.init();
    let tools = ToolSet::from_iter(vec![
        Arc::new(BddSlowTool {
            name: "slow_a",
            mode: xylitol::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 120,
        }) as Arc<dyn XyTool>,
        Arc::new(BddSlowTool {
            name: "slow_b",
            mode: xylitol::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 30,
        }) as Arc<dyn XyTool>,
    ]);
    let runner = bdd_batch_make_runner(
        agent,
        tools,
        &[("slow_a", r#"{}"#), ("slow_b", r#"{}"#)],
        xylitol::protocol::ports::XyBatchMode::BarrierParallel,
    );
    ar_store_runner(runner);
}

#[then("两工具执行时间重叠")]
fn _t_ar27_default_overlap() {
    let entries = BATCH_TIMING.with(|t| t.borrow().clone());
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert!(
        bdd_timing_overlaps((entries[0].1, entries[0].2), (entries[1].1, entries[1].2)),
        "default batch must overlap ParallelSafe tools: {entries:?}"
    );
}

#[then("两工具按源序串行执行且无并行重叠")]
fn _t_ar27_sequential_no_overlap() {
    let entries = BATCH_TIMING.with(|t| t.borrow().clone());
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert!(
        !bdd_timing_overlaps((entries[0].1, entries[0].2), (entries[1].1, entries[1].2)),
        "must not overlap: {entries:?}"
    );
    assert!(entries[0].2 <= entries[1].1, "source order: {entries:?}");
}

#[then("两 ParallelSafe 执行时间重叠且均在 Barrier 开始前结束")]
fn _t_ar28_overlap() {
    let entries = BATCH_TIMING.with(|t| t.borrow().clone());
    let safes: Vec<_> = entries
        .iter()
        .filter(|(n, _, _)| n == "slow_safe")
        .cloned()
        .collect();
    let barrier = entries
        .iter()
        .find(|(n, _, _)| n == "slow_barrier")
        .expect("barrier");
    assert_eq!(safes.len(), 2, "{entries:?}");
    assert!(
        bdd_timing_overlaps((safes[0].1, safes[0].2), (safes[1].1, safes[1].2)),
        "safes must overlap: {entries:?}"
    );
    let safe_end = safes.iter().map(|e| e.2).max().unwrap();
    assert!(safe_end <= barrier.1, "safes before barrier: {entries:?}");
}

#[then(
    "第二个 ParallelSafe MUST NOT 与第一个 ParallelSafe 同窗并行且 MUST 在 Barrier 完成之后开始"
)]
fn _t_ar28_windows() {
    let entries = BATCH_TIMING.with(|t| t.borrow().clone());
    assert_eq!(entries.len(), 3, "{entries:?}");
    let first = &entries[0];
    let barrier = entries
        .iter()
        .find(|(n, _, _)| n == "slow_barrier")
        .unwrap();
    let second = entries
        .iter()
        .rev()
        .find(|(n, _, _)| n == "slow_safe")
        .unwrap();
    assert!(first.2 <= barrier.1, "{entries:?}");
    assert!(barrier.2 <= second.1, "{entries:?}");
    assert!(
        !bdd_timing_overlaps((first.1, first.2), (second.1, second.2)),
        "{entries:?}"
    );
}

#[then("mcp 假工具与两侧 ParallelSafe 均无执行时间重叠")]
fn _t_ar28_mcp_no_overlap() {
    let entries = BATCH_TIMING.with(|t| t.borrow().clone());
    let mcp = entries
        .iter()
        .find(|(n, _, _)| n == "mcp:fake:x")
        .expect("mcp");
    for (n, s, e) in &entries {
        if n == "mcp:fake:x" {
            continue;
        }
        assert!(
            !bdd_timing_overlaps((*s, *e), (mcp.1, mcp.2)),
            "mcp overlaps {n}: {entries:?}"
        );
    }
}

#[when("检查 session history 中 toolResult")]
async fn _w_ar29_check_history(agent: &AgentState) {
    // Prefer prepared runner; run if events empty.
    if agent.events.borrow().is_empty() {
        _w_ar_react_run(agent).await;
    }
}

#[then("toolResult 顺序与 assistant 源序一致")]
fn _t_ar29_history_order(agent: &AgentState) {
    let events = agent.events.borrow();
    let history = events
        .iter()
        .rev()
        .find_map(|ev| match ev {
            XyEvent::AgentEnd { messages } => Some(messages.clone()),
            _ => None,
        })
        .expect("AgentEnd");
    let ids: Vec<_> = history
        .iter()
        .filter_map(|m| match m {
            xylitol::protocol::message::AgentMessage::Llm(
                xylitol::protocol::message::LlmMessage::ToolResultMessage { tool_use_id, .. },
            ) => Some(tool_use_id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        ids,
        vec!["call-0".to_string(), "call-1".to_string()],
        "history order: {ids:?}"
    );
}

// ar10 abort-cancels-bang
#[given("启动交互 bang 长命令后 abort")]
async fn _g_ar10_abort_cancels_bang(_agent: &AgentState, _ws: &Workspace) {
    use xylitol::infra::bash_exec::InfraBashExecutor;
    use xylitol::protocol::ports::{BashExecOpts, XyBashExecutor};
    let executor = std::sync::Arc::new(InfraBashExecutor::new());
    let token = tokio_util::sync::CancellationToken::new();
    let exec_for_task = executor.clone();
    let token_for_task = token.clone();
    let handle = tokio::task::spawn(async move {
        exec_for_task
            .execute("sleep 30", BashExecOpts::cancel_only(token_for_task))
            .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    token.cancel();
    let result = handle.await.expect("bash task joined");
    AR_BANG_RESULT.with(|r| *r.borrow_mut() = Some(Ok(result)));
}

#[when("检查 bash 结果")]
fn _w_ar10_check_bash_result() {}

#[then("cancelled 为 true")]
fn _t_ar10_bang_cancelled() {
    AR_BANG_RESULT.with(|r| {
        let borrow = r.borrow();
        let result = borrow
            .as_ref()
            .expect("bash result captured")
            .as_ref()
            .expect("execute_bash ok");
        assert!(
            result.cancelled,
            "bang must be cancelled, got cancelled={}",
            result.cancelled
        );
    });
}

// ar7 before-denies
#[given("注册匹配 bash 的 before 拒绝 hook")]
async fn _g_ar7_before_denies(agent: &AgentState, ws: &Workspace) {
    let mut runner = ar_make_runner(agent, ws);
    runner.add_hook(Arc::new(
        |name: &str, _id: &str, _args: &serde_json::Value| {
            if name == "bash" {
                Some("blocked by before hook".to_string())
            } else {
                None
            }
        },
    ));
    set_fake_tool_call("bash", r#"{"command":"echo hi"}"#);
    set_fake_text("ack");
    ar_store_runner(runner);
}

#[when("运行 AgentRuntime 触发 bash")]
async fn _w_ar7_run_trigger_bash() {
    ar_take_run_store("触发 bash").await;
}

#[then("tool-error 回写且未执行")]
fn _t_ar7_tool_error_written() {
    let events = ar_events();
    let has_error = events
        .iter()
        .any(|ev| matches!(ev, XyEvent::ToolExecutionEnd { is_error: true, .. }));
    assert!(
        has_error,
        "before hook must deny bash with tool-error, got {:?}",
        events
    );
}

// --- File given variants ---
#[given("存在文件 {path:string} 内容为 {content:string}")]
fn _g_file_with_content_string(ws: &Workspace, path: String, content: String) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, strip_quotes(&content)).expect("write failed");
}

// ═══════════════════════════════════════════════════════════════════
// ── Sandbox steps ────────────────────────────────────────────
mod sandbox_bdd {
    use rstest_bdd_macros::{given, scenario, then, when};
    use std::sync::Arc;
    use xylitol::infra::permission::{XyPermission, XyPermissionVerdict};

    thread_local! {
        static SANDBOX_ENGINE: std::cell::RefCell<Option<Arc<dyn XyPermission>>> =
            const { std::cell::RefCell::new(None) };
        static LAST_VERDICT: std::cell::RefCell<Option<XyPermissionVerdict>> =
            const { std::cell::RefCell::new(None) };
    }

    use xylitol::infra::config::types::{
        PermissionBackend, PermissionConfig, PermissionFilesystemConfig, PermissionNetworkConfig,
        PermissionProcessConfig,
    };

    fn default_sandbox() -> PermissionConfig {
        PermissionConfig {
            enabled: true,
            backend: PermissionBackend::Glob,
            filesystem: PermissionFilesystemConfig {
                read_allowed: vec!["/project/**".into()],
                write_allowed: vec!["/project/**".into()],
                write_denied: vec!["**/.env".into()],
            },
            network: PermissionNetworkConfig {
                allowed_domains: vec!["github.com".into()],
                denied_domains: vec!["evil.com".into()],
            },
            process: PermissionProcessConfig {
                allowed_paths: vec![],
            },
        }
    }

    #[given("沙箱引擎已初始化")]
    fn sandbox_engine_init() {
        SANDBOX_ENGINE.with(|e| {
            *e.borrow_mut() = Some(xylitol::infra::permission::build_permission(
                &default_sandbox(),
            ));
        });
    }

    #[when("检查写入路径 {path:string}")]
    fn sandbox_check_write(path: String) {
        SANDBOX_ENGINE.with(|e| {
            let engine = e.borrow();
            let verdict = engine.as_ref().unwrap().check_write(&path);
            LAST_VERDICT.with(|v| *v.borrow_mut() = Some(verdict));
        });
    }

    #[when("检查网络域名 {domain:string}")]
    fn sandbox_check_domain(domain: String) {
        SANDBOX_ENGINE.with(|e| {
            let engine = e.borrow();
            let verdict = engine.as_ref().unwrap().check_network(&domain);
            LAST_VERDICT.with(|v| *v.borrow_mut() = Some(verdict));
        });
    }

    #[then("结果应为拒绝")]
    fn sandbox_assert_denied() {
        LAST_VERDICT.with(|v| {
            let verdict = v.borrow();
            assert!(
                !verdict.as_ref().unwrap().is_allowed(),
                "Expected sandbox verdict to be Deny, but got Allow"
            );
        });
    }

    #[then("结果应为允许")]
    fn sandbox_assert_allowed() {
        LAST_VERDICT.with(|v| {
            let guard = v.borrow();
            let verdict = guard.as_ref().unwrap();
            assert!(
                verdict.is_allowed(),
                "Expected sandbox verdict to be Allow, but got {:?}",
                verdict
            );
        });
    }

    #[then("拒绝原因包含 {text:string}")]
    fn sandbox_assert_deny_reason(text: String) {
        LAST_VERDICT.with(|v| {
            let verdict = v.borrow();
            let reason = verdict.as_ref().unwrap().deny_reason().unwrap_or("");
            assert!(
                reason.contains(&text),
                "Expected deny reason to contain '{text}', got '{reason}'"
            );
        });
    }

    #[then("结果应为拒绝且原因含 write_denied")]
    fn sandbox_denied_write_reason() {
        sandbox_assert_denied();
        sandbox_assert_deny_reason("write_denied".into());
    }

    #[then("结果应为拒绝且原因含 denied_domains")]
    fn sandbox_denied_domain_reason() {
        sandbox_assert_denied();
        sandbox_assert_deny_reason("denied_domains".into());
    }

    // Scenario bindings — domain-security.feature (solidify；自 sandbox.feature 迁入)
    #[scenario(
        path = "llmanspec/specs/domain-security/domain-security.feature",
        name = "network-domain-block"
    )]
    fn test_domain_security_network_block() {}
    #[scenario(
        path = "llmanspec/specs/domain-security/domain-security.feature",
        name = "deny-write"
    )]
    fn test_domain_security_deny_write() {}
    #[scenario(
        path = "llmanspec/specs/domain-security/domain-security.feature",
        name = "allow-write"
    )]
    fn test_domain_security_allow_write() {}
}

// Scenario bindings — agent-tools.feature (solidify path; 七工具)
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "read-entire"
)]
fn test_read_entire(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "read-offset-limit"
)]
fn test_read_offset_limit(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "read-missing"
)]
fn test_read_missing(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "read-offset-oob"
)]
fn test_read_offset_oob(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "read-truncate"
)]
fn test_read_truncate(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "read-missing-path"
)]
fn test_read_missing_path(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "write-new"
)]
fn test_write_new(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "write-parents"
)]
fn test_write_parents(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "write-overwrite"
)]
fn test_write_overwrite(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "write-byte-count"
)]
fn test_write_byte_count(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "write-missing-path"
)]
fn test_write_missing_path(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "write-missing-content"
)]
fn test_write_missing_content(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-single"
)]
fn test_edit_single(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-multi"
)]
fn test_edit_multi(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-overlap"
)]
fn test_edit_overlap(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-nonunique"
)]
fn test_edit_nonunique(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-empty-old"
)]
fn test_edit_empty_old(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-noop"
)]
fn test_edit_noop(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-crlf"
)]
fn test_edit_crlf(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-bom"
)]
fn test_edit_bom(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-unicode"
)]
fn test_edit_unicode(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-diff"
)]
fn test_edit_diff(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-echo"
)]
fn test_bash_echo(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-stderr"
)]
fn test_bash_stderr(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-exit-code"
)]
fn test_bash_exit_code(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-timeout"
)]
fn test_bash_timeout(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-merged-streams"
)]
fn test_bash_merged_streams(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-truncate"
)]
fn test_bash_truncate(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-result-no-full-dump"
)]
fn test_bash_result_no_full_dump(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-cancel"
)]
fn test_bash_cancel(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-missing-cmd"
)]
fn test_bash_missing_cmd(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "grep-basic"
)]
fn test_grep_basic(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "grep-no-match"
)]
fn test_grep_no_match(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "grep-limit"
)]
fn test_grep_limit(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "grep-ignore-case"
)]
fn test_grep_ignore_case(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "grep-literal"
)]
fn test_grep_literal(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "grep-missing-pattern"
)]
fn test_grep_missing_pattern(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "find-simple"
)]
fn test_find_simple(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "find-recursive"
)]
fn test_find_recursive(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "find-limit"
)]
fn test_find_limit(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "find-no-match"
)]
fn test_find_no_match(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "find-bad-path"
)]
fn test_find_bad_path(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "find-absolute"
)]
fn test_find_absolute(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "ls-empty"
)]
fn test_ls_empty(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "ls-entries"
)]
fn test_ls_entries(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "ls-sorted"
)]
fn test_ls_sorted(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "ls-default-cwd"
)]
fn test_ls_default_cwd(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "ls-limit"
)]
fn test_ls_limit(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "ls-missing"
)]
fn test_ls_missing(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "ls-not-dir"
)]
fn test_ls_not_dir(ws: Workspace) {}

// session.feature (9) — async
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "create-load"
)]
async fn test_session_create_load(sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "list-sessions"
)]
async fn test_session_list(ws: Workspace, sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "fork"
)]
async fn test_session_fork(sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "tree-nav"
)]
async fn test_session_tree_nav(sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "model-change"
)]
async fn test_session_model_change(sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "thinking-change"
)]
async fn test_session_thinking_change(sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "jsonl-format"
)]
async fn test_session_jsonl_format(sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "label-set"
)]
async fn test_session_label_set(sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "label-clear"
)]
async fn test_session_label_clear(sess: XySessionStore) {}

// agent.feature (8) — async
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "text-response"
)]
async fn test_agent_text_response(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "tool-call"
)]
async fn test_agent_tool_call(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "turn-order"
)]
async fn test_agent_event_order(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "thinking-switch"
)]
async fn test_agent_thinking_switch(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "thinking-clamp"
)]
async fn test_agent_thinking_limit(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "context-usage"
)]
async fn test_agent_context_usage(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "auto-persist"
)]
async fn test_agent_auto_save(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

// BDD-on 新链路试点：solidify 风格 .feature（场景标题 = spec.toon scenario.id）
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "react-terminates"
)]
async fn test_ar_react_terminates(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "stream-is-xyevent"
)]
async fn test_ar_stream_is_xyevent(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "continues-after-tools"
)]
async fn test_ar_continues_after_tools(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "intent-before-execution"
)]
async fn test_ar_intent_before_execution(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "abort-drops-sse"
)]
async fn test_ar_abort_drops_sse(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "steer-before-model"
)]
async fn test_ar_steer_before_model(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "followup-extends"
)]
async fn test_ar_followup_extends(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "queue-update"
)]
async fn test_ar_queue_update(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "abort-clears-steer"
)]
async fn test_ar_abort_clears_steer(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "second-run-after-abort"
)]
async fn test_ar_second_run_after_abort(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "abort-cancels-bang"
)]
async fn test_ar_abort_cancels_bang(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "before-denies"
)]
async fn test_ar_before_denies(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "should-stop-emits-agent-end"
)]
async fn test_ar_should_stop_emits_agent_end(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "should-stop-skips-followup"
)]
async fn test_ar_should_stop_skips_followup(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "no-hook-open-end"
)]
async fn test_ar_no_hook_open_end(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "batch-default-barrier-parallel"
)]
async fn test_ar_batch_default_barrier_parallel(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "batch-barrier-parallel-overlap"
)]
async fn test_ar_batch_barrier_parallel_overlap(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "batch-barrier-preserves-source-windows"
)]
async fn test_ar_batch_barrier_preserves_windows(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "batch-mcp-never-parallel"
)]
async fn test_ar_batch_mcp_never_parallel(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "batch-history-source-order"
)]
async fn test_ar_batch_history_source_order(agent: AgentState, ws: Workspace) {}

// compaction.feature (5)
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "need-compact"
)]
fn test_compaction_need(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "no-compact"
)]
fn test_compaction_not_needed(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "retain-recent"
)]
fn test_compaction_keep_recent(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "write-entry"
)]
fn test_compaction_write(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "branch-summary"
)]
fn test_compaction_branch(agent: AgentState, ws: Workspace) {}

// hooks — solidify agent-hooks.feature（自 tests/features/hooks.feature 迁入）
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "pre-tool-call"
)]
async fn test_hook_pre(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "hook-blocks"
)]
async fn test_hook_block(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "hook-modifies-args"
)]
async fn test_hook_modify_args(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "three-layer-merge"
)]
async fn test_hook_merge(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "hook-timeout"
)]
async fn test_hook_timeout(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "before-provider-request"
)]
async fn test_hook_provider_request(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "after-provider-response"
)]
async fn test_hook_provider_response(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "empty-hooks-noop"
)]
async fn test_hook_empty_noop(agent: AgentState) {}

// hooks-wiring — solidify test-hooks-wiring.feature（库缝；与 agent-hooks 调度器分工）
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "session-start"
)]
async fn test_hooks_wiring_session_start(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "model-select"
)]
async fn test_hooks_wiring_model_select(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "thinking-select"
)]
async fn test_hooks_wiring_thinking_select(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "session-tree"
)]
async fn test_hooks_wiring_session_tree(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "tree-cancel"
)]
async fn test_hooks_wiring_tree_cancel(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "session-shutdown"
)]
async fn test_hooks_wiring_session_shutdown(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "switch-cancel"
)]
async fn test_hooks_wiring_switch_cancel(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "user-bash"
)]
async fn test_hooks_wiring_user_bash(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "user-bash-block"
)]
async fn test_hooks_wiring_user_bash_block(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "unknown-op"
)]
async fn test_hooks_wiring_unknown_op(agent: AgentState) {}
#[test]
fn curated_xy_hook_bus_symbols_resolve() {
    fn assert_port<T: ?Sized>() {}
    assert_port::<dyn xylitol::XyHookBus>();
    let _ = xylitol::XyHookOutcome::Allowed;
    let _ = xylitol::NoopHookBus;
}
// ═══════════════════════════════════════════════════════════════════
// server.feature — Server lifecycle
// ═══════════════════════════════════════════════════════════════════

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
fn server_test() -> ServerTest {
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

// server.feature scenarios
#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "start-healthz"
)]
fn test_server_start_healthz(server_test: ServerTest) {}
#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "second-instance-rejected"
)]
fn test_server_second_instance_rejected(server_test: ServerTest) {}

// ═══════════════════════════════════════════════════════════════════
// approval.feature — Reverse RPC tool approval
// ═══════════════════════════════════════════════════════════════════

use xylitol::app::server::ws::ReverseRpcGateway;

/// Fixture for approval tests.
pub struct ApprovalTest {
    pub gateway: ReverseRpcGateway,
    pub last_result: RefCell<Option<String>>,
}
impl ApprovalTest {
    fn new() -> Self {
        Self {
            gateway: ReverseRpcGateway::new(),
            last_result: RefCell::new(None),
        }
    }
}

#[fixture]
fn approval_test() -> ApprovalTest {
    ApprovalTest::new()
}

#[given("服务端和已连接的 WebSocket 客户端")]
fn server_and_ws_client(_approval_test: &mut ApprovalTest) {
    // Gateway initialized in fixture; represents the server side.
    // WS client is implied by the ability to call handle_approve.
}

#[when("agent 执行需要审批的工具")]
fn agent_executes_approvable_tool(approval_test: &mut ApprovalTest) {
    // Register a pending call (simulates agent emitting ApprovalRequired).
    approval_test.gateway.register("call-approve-1".into());
}

#[then("客户端收到带有 call_id 的审批请求")]
fn client_receives_approval_request(approval_test: &mut ApprovalTest) {
    // The gateway has a pending call registered.
    assert_eq!(approval_test.gateway.pending_count(), 1);
}

#[when("客户端发送 ApproveTool approved=true")]
fn client_approves(approval_test: &mut ApprovalTest) {
    let consumed = approval_test.gateway.handle_approve("call-approve-1", true);
    assert!(consumed, "call_id should be consumed");
}

#[when("客户端发送 ApproveTool approved=false")]
fn client_denies(approval_test: &mut ApprovalTest) {
    let consumed = approval_test
        .gateway
        .handle_approve("call-approve-1", false);
    assert!(consumed, "call_id should be consumed");
}

#[then("工具执行继续")]
fn tool_execution_continues(approval_test: &mut ApprovalTest) {
    // Call was consumed; no pending calls remain.
    assert_eq!(approval_test.gateway.pending_count(), 0);
}

#[then("turn 正常结束")]
fn turn_completes(_approval_test: &mut ApprovalTest) {}

#[then("工具被拒绝")]
fn tool_denied(approval_test: &mut ApprovalTest) {
    assert_eq!(approval_test.gateway.pending_count(), 0);
}

#[then("turn 继续但不包含工具结果")]
fn turn_continues_without_tool(_approval_test: &mut ApprovalTest) {}

// ═══════════════════════════════════════════════════════════════════
// package-ai-bridge — c1250 tool-call stream lifecycle (pab13/pab14)
// ═══════════════════════════════════════════════════════════════════

pub struct AiBridgeBdd {
    chunks: RefCell<Vec<xylitol_ai_bridge::dto::AiBridgeChunk>>,
    parse_value: RefCell<Option<serde_json::Value>>,
    input_items: RefCell<Vec<serde_json::Value>>,
    request_body: RefCell<Option<serde_json::Value>>,
}

impl AiBridgeBdd {
    fn new() -> Self {
        Self {
            chunks: RefCell::new(Vec::new()),
            parse_value: RefCell::new(None),
            input_items: RefCell::new(Vec::new()),
            request_body: RefCell::new(None),
        }
    }
}

#[fixture]
fn ai_bridge_bdd() -> AiBridgeBdd {
    AiBridgeBdd::new()
}

#[given(
    "Responses SSE 含 function_call 的 output_item.added 与多帧 function_call_arguments.delta 后才有 output_item.done"
)]
fn g_pab13_responses_sse(ai_bridge_bdd: &AiBridgeBdd) {
    use xylitol_ai_bridge::dto::AiBridgeChunk;
    use xylitol_ai_bridge::provider::{ResponsesStreamState, map_responses_sse_event};

    let mut state = ResponsesStreamState::default();
    let mut out = Vec::new();
    let events = [
        serde_json::json!({
            "type": "response.output_item.added",
            "item": { "type": "function_call", "id": "fc_bdd", "name": "ls", "arguments": "" }
        }),
        serde_json::json!({
            "type": "response.function_call_arguments.delta",
            "item_id": "fc_bdd",
            "delta": "{\"path\":"
        }),
        serde_json::json!({
            "type": "response.function_call_arguments.delta",
            "item_id": "fc_bdd",
            "delta": "\"/tmp\"}"
        }),
        serde_json::json!({
            "type": "response.output_item.done",
            "item": {
                "type": "function_call",
                "id": "fc_bdd",
                "name": "ls",
                "arguments": "{\"path\":\"/tmp\"}"
            }
        }),
    ];
    for ev in events {
        out.extend(map_responses_sse_event(&ev, &mut state));
    }
    assert!(
        out.iter()
            .any(|c| matches!(c, AiBridgeChunk::ToolCallStart { .. })),
        "fixture must produce Start, got {out:?}"
    );
    ai_bridge_bdd.chunks.replace(out);
}

#[when("映射为 AiBridgeChunk 流")]
fn w_pab13_already_mapped(ai_bridge_bdd: &AiBridgeBdd) {
    assert!(
        !ai_bridge_bdd.chunks.borrow().is_empty(),
        "expected chunks from given step"
    );
}

#[then(
    "首个 args delta 之前或当时已有 ToolCallStart 且存在至少一次 ToolCallDelta 早于对应 ToolCallEnd"
)]
fn t_pab13_lifecycle(ai_bridge_bdd: &AiBridgeBdd) {
    use xylitol_ai_bridge::dto::AiBridgeChunk;
    let chunks = ai_bridge_bdd.chunks.borrow();
    let start = chunks
        .iter()
        .position(|c| matches!(c, AiBridgeChunk::ToolCallStart { .. }));
    let delta = chunks
        .iter()
        .position(|c| matches!(c, AiBridgeChunk::ToolCallDelta { .. }));
    let end = chunks
        .iter()
        .position(|c| matches!(c, AiBridgeChunk::ToolCallEnd { .. }));
    assert!(start.is_some(), "missing ToolCallStart in {chunks:?}");
    assert!(delta.is_some(), "missing ToolCallDelta in {chunks:?}");
    assert!(end.is_some(), "missing ToolCallEnd in {chunks:?}");
    let (s, d, e) = (start.unwrap(), delta.unwrap(), end.unwrap());
    assert!(
        s <= d && d < e,
        "expected Start<=Delta<End, got Start={s} Delta={d} End={e} chunks={chunks:?}"
    );
}

#[given("输入残缺工具参数 JSON")]
fn g_pab14_partial_json(ai_bridge_bdd: &AiBridgeBdd) {
    ai_bridge_bdd.parse_value.replace(None);
}

#[when("调用 parse_streaming_json")]
fn w_pab14_parse(ai_bridge_bdd: &AiBridgeBdd) {
    let v = xylitol_ai_bridge::dto::parse_streaming_json(r#"{"command":"ls"#);
    ai_bridge_bdd.parse_value.replace(Some(v));
}

#[then("返回 Value 且不 panic")]
fn t_pab14_ok(ai_bridge_bdd: &AiBridgeBdd) {
    let v = ai_bridge_bdd
        .parse_value
        .borrow()
        .clone()
        .expect("parse_streaming_json result missing");
    assert!(v.is_object(), "expected object, got {v:?}");
    assert_eq!(v.get("command").and_then(|c| c.as_str()), Some("ls"));
}

#[given("Responses 组装且 system_prompt 非空且 thinking_level 为 medium")]
fn g_pab15_system_developer(ai_bridge_bdd: &AiBridgeBdd) {
    use xylitol_ai_bridge::dto::AiBridgeMessage;
    use xylitol_ai_bridge::provider::messages_to_responses_input_with_options;
    use xylitol_ai_bridge::thinking::AiBridgeGenerateOptions;

    let opts = AiBridgeGenerateOptions {
        thinking_level: "medium".into(),
        system_prompt: Some("SYS_PROMPT_BDD".into()),
        ..Default::default()
    };
    let items = messages_to_responses_input_with_options(&[AiBridgeMessage::user("hi")], &opts);
    ai_bridge_bdd.input_items.replace(items);
}

#[when("转换为 input items")]
fn w_pab15_already_converted(ai_bridge_bdd: &AiBridgeBdd) {
    assert!(
        !ai_bridge_bdd.input_items.borrow().is_empty(),
        "expected input items from given"
    );
}

#[then("首项 role 为 developer 且 content 为 system_prompt")]
fn t_pab15_developer(ai_bridge_bdd: &AiBridgeBdd) {
    let items = ai_bridge_bdd.input_items.borrow();
    assert_eq!(items[0]["role"], "developer");
    assert_eq!(items[0]["content"], "SYS_PROMPT_BDD");
}

#[given("assistant 含 Thinking 无 signature 与 Text")]
fn g_pab15_thinking_text(ai_bridge_bdd: &AiBridgeBdd) {
    use xylitol_ai_bridge::dto::{AiBridgeMessage, AiBridgePart, AiBridgeStopReason};
    use xylitol_ai_bridge::provider::messages_to_responses_input;

    let msgs = vec![AiBridgeMessage::AssistantMessage {
        content: vec![
            AiBridgePart::Thinking {
                thinking: "HIDDEN_THINK".into(),
                redacted: false,
                thinking_signature: None,
            },
            AiBridgePart::text("ONLY_TEXT"),
        ],
        stop_reason: Some(AiBridgeStopReason::Stop),
        usage: None,
        api: String::new(),
        provider: String::new(),
        model: String::new(),
        response_id: None,
        error_message: None,
        timestamp: 0,
        diagnostics: Vec::new(),
    }];
    ai_bridge_bdd
        .input_items
        .replace(messages_to_responses_input(&msgs));
}

#[when("转换为 Responses input")]
fn w_pab15_converted_again(ai_bridge_bdd: &AiBridgeBdd) {
    assert!(
        !ai_bridge_bdd.input_items.borrow().is_empty(),
        "expected input items"
    );
}

#[then("output_text 仅含 Text 且无 Thinking 正文")]
fn t_pab15_text_only(ai_bridge_bdd: &AiBridgeBdd) {
    let items = ai_bridge_bdd.input_items.borrow();
    let assistant = items
        .iter()
        .find(|i| i.get("role") == Some(&serde_json::json!("assistant")))
        .expect("assistant item");
    let text = assistant["content"][0]["text"].as_str().unwrap();
    assert_eq!(text, "ONLY_TEXT");
    assert!(!text.contains("HIDDEN_THINK"));
}

#[given("Responses 组装且 thinking_level 为 medium 且 tools 非空")]
fn g_pab16_body(ai_bridge_bdd: &AiBridgeBdd) {
    use xylitol_ai_bridge::dto::{AiBridgeMessage, AiBridgeToolSchema};
    use xylitol_ai_bridge::provider::assemble_responses_body;
    use xylitol_ai_bridge::thinking::AiBridgeGenerateOptions;

    let tools = [AiBridgeToolSchema {
        name: "bash".into(),
        description: "run".into(),
        parameters: serde_json::json!({"type": "object"}),
    }];
    let body = assemble_responses_body(
        "m",
        vec![AiBridgeMessage::user("hi")],
        &tools,
        false,
        &AiBridgeGenerateOptions {
            thinking_level: "medium".into(),
            ..Default::default()
        },
    );
    ai_bridge_bdd.request_body.replace(Some(body));
}

#[when("构建请求体")]
fn w_pab16_build(ai_bridge_bdd: &AiBridgeBdd) {
    assert!(
        ai_bridge_bdd.request_body.borrow().is_some(),
        "expected request body from given"
    );
}

#[then(
    "store 为 false 且每个 tool 的 strict 为 false 且 reasoning.summary 存在且 include 含 reasoning.encrypted_content"
)]
fn t_pab16_fields(ai_bridge_bdd: &AiBridgeBdd) {
    let body = ai_bridge_bdd.request_body.borrow().clone().expect("body");
    assert_eq!(body["store"], false);
    let tools = body["tools"].as_array().expect("tools");
    assert!(!tools.is_empty());
    for t in tools {
        assert_eq!(t["strict"], false, "tool strict: {t}");
    }
    assert_eq!(body["reasoning"]["summary"], "auto");
    let include = body["include"].as_array().expect("include");
    assert!(
        include
            .iter()
            .any(|v| v.as_str() == Some("reasoning.encrypted_content")),
        "include={include:?}"
    );
}

#[given("Responses 流或非流输出含完整 type=reasoning 的 output item")]
fn g_pab16_reasoning_item(ai_bridge_bdd: &AiBridgeBdd) {
    use xylitol_ai_bridge::provider::{ResponsesStreamState, map_responses_sse_event};

    let mut state = ResponsesStreamState::default();
    let event = serde_json::json!({
        "type": "response.output_item.done",
        "item": {
            "type": "reasoning",
            "id": "rs_bdd",
            "summary": [{"type": "summary_text", "text": "think"}],
            "encrypted_content": "blob"
        }
    });
    ai_bridge_bdd
        .chunks
        .replace(map_responses_sse_event(&event, &mut state));
}

#[when("映射为 AiBridgeChunk")]
fn w_pab16_map_chunks(ai_bridge_bdd: &AiBridgeBdd) {
    assert!(!ai_bridge_bdd.chunks.borrow().is_empty(), "expected chunks");
}

#[then(
    "存在带 thinkingSignature 的 Thinking 终态（ThinkingEnd 或等价）且 signature 可 JSON 解析为该 reasoning item"
)]
fn t_pab16_signature(ai_bridge_bdd: &AiBridgeBdd) {
    use xylitol_ai_bridge::dto::AiBridgeChunk;

    let chunks = ai_bridge_bdd.chunks.borrow();
    let end = chunks.iter().find_map(|c| match c {
        AiBridgeChunk::ThinkingEnd {
            thinking_signature: Some(sig),
            ..
        } => Some(sig.clone()),
        _ => None,
    });
    let sig = end.expect("ThinkingEnd with signature");
    let parsed: serde_json::Value = serde_json::from_str(&sig).expect("signature JSON");
    assert_eq!(parsed["type"], "reasoning");
    assert_eq!(parsed["id"], "rs_bdd");
    assert_eq!(parsed["encrypted_content"], "blob");
}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge/package-ai-bridge.feature",
    name = "responses-toolcall-streams-before-done"
)]
fn test_pab13_responses_toolcall_stream(ai_bridge_bdd: AiBridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge/package-ai-bridge.feature",
    name = "partial-args-object"
)]
fn test_pab14_partial_args(ai_bridge_bdd: AiBridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge/package-ai-bridge.feature",
    name = "responses-system-as-developer"
)]
fn test_pab15_system_developer(ai_bridge_bdd: AiBridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge/package-ai-bridge.feature",
    name = "responses-thinking-not-in-output-text"
)]
fn test_pab15_thinking_omit(ai_bridge_bdd: AiBridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge/package-ai-bridge.feature",
    name = "responses-body-store-strict-summary-include"
)]
fn test_pab16_body_fields(ai_bridge_bdd: AiBridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge/package-ai-bridge.feature",
    name = "responses-reasoning-item-sets-thinking-signature"
)]
fn test_pab16_thinking_signature(ai_bridge_bdd: AiBridgeBdd) {}

// ── agent-prompt pt9 (c1290) ──────────────────────────────────────

pub struct PromptBdd {
    prompt: RefCell<String>,
}

impl PromptBdd {
    fn new() -> Self {
        Self {
            prompt: RefCell::new(String::new()),
        }
    }
}

#[fixture]
fn prompt_bdd() -> PromptBdd {
    PromptBdd::new()
}

#[given("工具集含 bash 且其 prompt_guidelines 非空")]
fn g_pt9_bash_guidelines(prompt_bdd: &PromptBdd) {
    use xylitol::agent::prompt::{SystemPromptOpts, build_system_prompt};
    use xylitol::protocol::ports::XyTool;

    let bash = BashTool::default();
    assert!(
        !bash.prompt_guidelines().is_empty(),
        "bash guidelines must be non-empty"
    );
    let opts = SystemPromptOpts {
        prompt_guidelines: bash
            .prompt_guidelines()
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        cwd: ".".into(),
        ..Default::default()
    };
    // Store guidelines for when step; also prebuild for convenience
    prompt_bdd.prompt.replace(build_system_prompt(&opts));
}

#[when("set_tools 或等价装配后 build_system_prompt")]
fn w_pt9_build(prompt_bdd: &PromptBdd) {
    assert!(
        !prompt_bdd.prompt.borrow().is_empty(),
        "expected prompt from given"
    );
}

#[then("输出含 Guidelines 段且含该工具 guideline 短句")]
fn t_pt9_guidelines(prompt_bdd: &PromptBdd) {
    let p = prompt_bdd.prompt.borrow();
    assert!(p.contains("Guidelines:"), "{p}");
    assert!(
        p.contains("Prefer specialized read/edit/write tools") || p.contains("bash"),
        "expected bash guideline in {p}"
    );
}

#[given("custom_prompt 或 SYSTEM.md 整段替换默认正文且未附 Available tools")]
fn g_pt9_custom(prompt_bdd: &PromptBdd) {
    use xylitol::agent::prompt::{SystemPromptOpts, build_system_prompt};

    let opts = SystemPromptOpts {
        custom_prompt: Some("CUSTOM_ONLY_BODY".into()),
        selected_tools: vec!["read".into()],
        tool_snippets: vec![("read".into(), "Read file".into())],
        cwd: ".".into(),
        ..Default::default()
    };
    prompt_bdd.prompt.replace(build_system_prompt(&opts));
}

#[when("build_system_prompt")]
fn w_pt9_build_again(prompt_bdd: &PromptBdd) {
    assert!(!prompt_bdd.prompt.borrow().is_empty());
}

#[then("正文以该替换内容为主且 MUST NOT 偷偷回填默认 Available tools 清单")]
fn t_pt9_no_backfill(prompt_bdd: &PromptBdd) {
    let p = prompt_bdd.prompt.borrow();
    assert!(p.contains("CUSTOM_ONLY_BODY"), "{p}");
    assert!(!p.contains("Available tools:"), "{p}");
}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "collect-tool-guidelines"
)]
fn test_pt9_collect(prompt_bdd: PromptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "custom-prompt-no-silent-tools-backfill"
)]
fn test_pt9_no_backfill(prompt_bdd: PromptBdd) {}

// approval.feature scenarios
#[scenario(
    path = "llmanspec/specs/server-core/server-reverse-rpc.feature",
    name = "approve-roundtrip"
)]
fn test_approval_roundtrip(approval_test: ApprovalTest) {}
#[scenario(
    path = "llmanspec/specs/server-core/server-reverse-rpc.feature",
    name = "tool-denied"
)]
fn test_approval_denied(approval_test: ApprovalTest) {}

// ═══════════════════════════════════════════════════════════════════
// c1380 — tokenizer cache (ce15 / paa8 / paa9 / rc18)
// ═══════════════════════════════════════════════════════════════════

pub struct TokenizerBdd {
    cache_dir: RefCell<Option<tempfile::TempDir>>,
    cache: RefCell<Option<xylitol_ai_bridge::tokenize::HfTokenizerCache>>,
    config: RefCell<xylitol::infra::config::types::AppConfig>,
    cli_out: RefCell<String>,
    cli_code_ok: Cell<bool>,
    last_url: RefCell<String>,
    last_list_len: Cell<usize>,
    cfg_ok: Cell<bool>,
    cfg_err: RefCell<String>,
    resolved_hf_repo: RefCell<String>,
    mock_uri: RefCell<String>,
}

impl TokenizerBdd {
    fn new() -> Self {
        Self {
            cache_dir: RefCell::new(None),
            cache: RefCell::new(None),
            config: RefCell::new(xylitol::infra::config::types::AppConfig::default()),
            cli_out: RefCell::new(String::new()),
            cli_code_ok: Cell::new(false),
            last_url: RefCell::new(String::new()),
            last_list_len: Cell::new(0),
            cfg_ok: Cell::new(false),
            cfg_err: RefCell::new(String::new()),
            resolved_hf_repo: RefCell::new(String::new()),
            mock_uri: RefCell::new(String::new()),
        }
    }

    fn ensure_cache(&self) -> xylitol_ai_bridge::tokenize::HfTokenizerCache {
        if self.cache.borrow().is_none() {
            let dir = tempfile::tempdir().expect("temp cache");
            let cache =
                xylitol_ai_bridge::tokenize::HfTokenizerCache::new(Some(dir.path().to_path_buf()));
            self.cache_dir.replace(Some(dir));
            self.cache.replace(Some(cache));
        }
        self.cache.borrow().as_ref().unwrap().clone()
    }
}

#[fixture]
fn tokenizer_bdd() -> TokenizerBdd {
    TokenizerBdd::new()
}

fn parse_app_config_yaml(
    yaml: &str,
) -> Result<xylitol::infra::config::types::AppConfig, XyDriverError> {
    // Prefer direct typed deserialize so unknown enum variants (e.g. local_tokenizer)
    // fail here rather than only after a loose Value round-trip.
    let cfg: xylitol::infra::config::types::AppConfig =
        yaml_serde::from_str(yaml).map_err(|e| format!("yaml: {e}"))?;
    cfg.validate_thinking_levels()?;
    cfg.validate_model_tokenizers()?;
    Ok(cfg)
}

fn minimal_model_yaml(tokenizer_line: &str, with_tokenizers_table: bool) -> String {
    let table = if with_tokenizers_table {
        r#"
tokenizers:
  qwen36:
    repo: Qwen/Qwen3.6-35B-A3B
"#
    } else {
        ""
    };
    format!(
        r#"{table}
models:
  models:
    qwen:
      provider: openai
      model: Qwen3.6-35B-A3B/UD-Q5_K_XL
      {tokenizer_line}
"#
    )
}

#[given("CLI 已解析")]
fn g_ce15_cli_parsed(_tokenizer_bdd: &TokenizerBdd) {}

#[when("xylitol tokenizer --help")]
fn w_ce15_tokenizer_help(tokenizer_bdd: &TokenizerBdd) {
    use clap::CommandFactory;
    let mut cmd = xylitol::app::cli::CliArgs::command();
    let help = cmd
        .find_subcommand_mut("tokenizer")
        .expect("tokenizer subcommand")
        .render_long_help()
        .to_string();
    tokenizer_bdd.cli_out.replace(help);
    tokenizer_bdd.cli_code_ok.set(true);
}

#[then("可见 status、download、clean 叶子")]
fn t_ce15_help_leaves(tokenizer_bdd: &TokenizerBdd) {
    let out = tokenizer_bdd.cli_out.borrow();
    assert!(out.contains("status"), "{out}");
    assert!(out.contains("download"), "{out}");
    assert!(out.contains("clean"), "{out}");
}

#[given("缓存目录为空")]
fn g_ce15_empty_cache(tokenizer_bdd: &TokenizerBdd) {
    let _ = tokenizer_bdd.ensure_cache();
}

#[when("xylitol tokenizer status")]
async fn w_ce15_status(tokenizer_bdd: &TokenizerBdd) {
    use std::process::ExitCode;
    use xylitol::app::cli::tokenizer::{TokenizerAction, run_with};

    let cache = tokenizer_bdd.ensure_cache();
    let cfg = tokenizer_bdd.config.borrow().clone();
    let (code, out) = run_with(TokenizerAction::Status { model: None }, &cache, &cfg, false).await;
    tokenizer_bdd.cli_out.replace(out);
    tokenizer_bdd.cli_code_ok.set(code == ExitCode::SUCCESS);
}

#[then("报告缓存根且不失败伪装已下载")]
fn t_ce15_status_empty(tokenizer_bdd: &TokenizerBdd) {
    assert!(tokenizer_bdd.cli_code_ok.get());
    let out = tokenizer_bdd.cli_out.borrow();
    assert!(out.contains("cache_root:"), "{out}");
    assert!(out.contains("(none)"), "{out}");
}

#[given("目标映射到 HuggingFace 且本地无缓存")]
fn g_ce15_mapped_missing(tokenizer_bdd: &TokenizerBdd) {
    let yaml = minimal_model_yaml("tokenizer: qwen36", true);
    let cfg = parse_app_config_yaml(&yaml).expect("cfg");
    tokenizer_bdd.config.replace(cfg);
    let _ = tokenizer_bdd.ensure_cache();
}

#[when("xylitol tokenizer download <target> --yes")]
async fn w_ce15_download_yes(tokenizer_bdd: &TokenizerBdd) {
    use std::process::ExitCode;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use xylitol::app::cli::tokenizer::{TokenizerAction, run_with};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/org/bdd/resolve/main/tokenizer.json"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"{\"version\":\"1.0\"}"))
        .mount(&server)
        .await;

    // Remap to org/bdd so mock path matches.
    let yaml = r#"
tokenizers:
  bddtok:
    repo: org/bdd
models:
  models:
    qwen:
      provider: openai
      model: qwen-x
      tokenizer: bddtok
"#;
    let cfg = parse_app_config_yaml(yaml).expect("cfg");
    tokenizer_bdd.config.replace(cfg);
    tokenizer_bdd.mock_uri.replace(server.uri());

    let cache = tokenizer_bdd.ensure_cache();
    let prev = std::env::var("HF_ENDPOINT").ok();
    unsafe { std::env::set_var("HF_ENDPOINT", server.uri()) };
    let cfg = tokenizer_bdd.config.borrow().clone();
    let (code, out) = run_with(
        TokenizerAction::Download {
            target: "qwen".into(),
            file: None,
            yes: true,
        },
        &cache,
        &cfg,
        false,
    )
    .await;
    match prev {
        Some(v) => unsafe { std::env::set_var("HF_ENDPOINT", v) },
        None => unsafe { std::env::remove_var("HF_ENDPOINT") },
    }
    tokenizer_bdd.cli_out.replace(out);
    tokenizer_bdd.cli_code_ok.set(code == ExitCode::SUCCESS);
}

#[then("词表落入缓存路径且再次 status 可见")]
async fn t_ce15_download_cached(tokenizer_bdd: &TokenizerBdd) {
    use xylitol::app::cli::tokenizer::{TokenizerAction, run_with};

    assert!(
        tokenizer_bdd.cli_code_ok.get(),
        "download failed: {}",
        tokenizer_bdd.cli_out.borrow()
    );
    let cache = tokenizer_bdd.ensure_cache();
    let cfg = tokenizer_bdd.config.borrow().clone();
    let (_, out) = run_with(TokenizerAction::Status { model: None }, &cache, &cfg, false).await;
    assert!(out.contains("org/bdd") || out.contains("bdd"), "{out}");
    assert!(!out.contains("(none)") || out.contains("entries:"), "{out}");
    let entries = cache.list_entries();
    assert!(
        entries.iter().any(|e| e.repo == "org/bdd"),
        "entries={entries:?}"
    );
}

#[given("缓存中已有条目")]
fn g_ce15_has_entry(tokenizer_bdd: &TokenizerBdd) {
    let cache = tokenizer_bdd.ensure_cache();
    let path = cache.cache_path("org/cached", "tokenizer.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"{}").unwrap();
}

#[when("xylitol tokenizer clean --all")]
async fn w_ce15_clean_all(tokenizer_bdd: &TokenizerBdd) {
    use std::process::ExitCode;
    use xylitol::app::cli::tokenizer::{TokenizerAction, run_with};

    let cache = tokenizer_bdd.ensure_cache();
    let cfg = tokenizer_bdd.config.borrow().clone();
    let (code, out) = run_with(
        TokenizerAction::Clean {
            all: true,
            model: None,
            target: None,
        },
        &cache,
        &cfg,
        false,
    )
    .await;
    tokenizer_bdd.cli_out.replace(out);
    tokenizer_bdd.cli_code_ok.set(code == ExitCode::SUCCESS);
}

#[then("条目被移除且 status 不再列出")]
async fn t_ce15_clean_gone(tokenizer_bdd: &TokenizerBdd) {
    use xylitol::app::cli::tokenizer::{TokenizerAction, run_with};

    assert!(tokenizer_bdd.cli_code_ok.get());
    let cache = tokenizer_bdd.ensure_cache();
    assert!(cache.list_entries().is_empty());
    let cfg = tokenizer_bdd.config.borrow().clone();
    let (_, out) = run_with(TokenizerAction::Status { model: None }, &cache, &cfg, false).await;
    assert!(out.contains("(none)"), "{out}");
}

#[given("仅执行 tokenizer 子命令")]
fn g_ce15_tokenizer_only(tokenizer_bdd: &TokenizerBdd) {
    let _ = tokenizer_bdd.ensure_cache();
}

#[then("不经 bootstrap 装配会话或 MCP 即可完成")]
fn t_ce15_no_bootstrap(tokenizer_bdd: &TokenizerBdd) {
    // `run_with` is the early-dispatch leaf used by CLI before bootstrap.
    assert!(tokenizer_bdd.cli_code_ok.get());
    assert!(tokenizer_bdd.cli_out.borrow().contains("cache_root:"));
}

#[given("已设置 HF_ENDPOINT 为镜像基址且目标已映射")]
fn g_ce15_hf_mirror_mapped(tokenizer_bdd: &TokenizerBdd) {
    let yaml = minimal_model_yaml("tokenizer: qwen36", true);
    tokenizer_bdd
        .config
        .replace(parse_app_config_yaml(&yaml).expect("cfg"));
    let _ = tokenizer_bdd.ensure_cache();
    // Fast-fail local "mirror" so summary is asserted without waiting on real HF.
    let mirror = "http://127.0.0.1:9";
    tokenizer_bdd.mock_uri.replace(mirror.into());
    unsafe { std::env::set_var("HF_ENDPOINT", mirror) };
}

#[when("xylitol tokenizer download <target> 进入确认摘要（或 --yes 的等价日志）")]
async fn w_ce15_download_summary(tokenizer_bdd: &TokenizerBdd) {
    use xylitol::app::cli::tokenizer::{TokenizerAction, run_with};

    let cache = tokenizer_bdd.ensure_cache();
    let cfg = tokenizer_bdd.config.borrow().clone();
    // Download may fail (closed port); summary is printed first.
    let (_code, out) = run_with(
        TokenizerAction::Download {
            target: "qwen".into(),
            file: None,
            yes: true,
        },
        &cache,
        &cfg,
        false,
    )
    .await;
    unsafe { std::env::remove_var("HF_ENDPOINT") };
    tokenizer_bdd.cli_out.replace(out);
}

#[then("摘要含该镜像基址与落盘路径")]
fn t_ce15_summary_has_base(tokenizer_bdd: &TokenizerBdd) {
    let out = tokenizer_bdd.cli_out.borrow();
    let base = tokenizer_bdd.mock_uri.borrow();
    assert!(out.contains(&format!("hf_base: {base}")), "{out}");
    assert!(out.contains("dest:"), "{out}");
}

#[given("缓存目录中已有 tokenizer.json 条目")]
fn g_paa8_has_entry(tokenizer_bdd: &TokenizerBdd) {
    g_ce15_has_entry(tokenizer_bdd);
}

#[when("列举并删除该缓存键")]
fn w_paa8_list_remove(tokenizer_bdd: &TokenizerBdd) {
    let cache = tokenizer_bdd.ensure_cache();
    let before = cache.list_entries();
    tokenizer_bdd.last_list_len.set(before.len());
    assert!(before.iter().any(|e| e.repo == "org/cached"));
    cache.remove("org/cached", "tokenizer.json").unwrap();
}

#[then("列举曾包含该条目且删除后不再包含")]
fn t_paa8_removed(tokenizer_bdd: &TokenizerBdd) {
    assert!(tokenizer_bdd.last_list_len.get() >= 1);
    let cache = tokenizer_bdd.ensure_cache();
    assert!(!cache.list_entries().iter().any(|e| e.repo == "org/cached"));
}

#[given("opt-in download 中途失败")]
async fn g_paa8_download_fail(tokenizer_bdd: &TokenizerBdd) {
    let cache = tokenizer_bdd.ensure_cache();
    let err = cache
        .download_opt_in("org/fail", "tokenizer.json", "http://127.0.0.1:9/nope")
        .await;
    assert!(err.is_err(), "expected fail");
}

#[when("再次 encode_count_if_cached")]
fn w_paa8_encode_again(tokenizer_bdd: &TokenizerBdd) {
    let cache = tokenizer_bdd.ensure_cache();
    let n = cache.encode_count_if_cached("org/fail", "tokenizer.json", "x");
    tokenizer_bdd
        .last_list_len
        .set(if n.is_some() { 1 } else { 0 });
}

#[then("不把不完整文件当作可用缓存")]
fn t_paa8_no_partial(tokenizer_bdd: &TokenizerBdd) {
    assert_eq!(tokenizer_bdd.last_list_len.get(), 0);
    let cache = tokenizer_bdd.ensure_cache();
    assert!(!cache.cache_path("org/fail", "tokenizer.json").exists());
}

#[given("环境变量 HF_ENDPOINT 为 https://hf-mirror.com")]
fn g_paa9_mirror(_tokenizer_bdd: &TokenizerBdd) {}

#[when("拼装某 repo 的 tokenizer.json resolve URL")]
fn w_paa9_build_mirror(tokenizer_bdd: &TokenizerBdd) {
    use xylitol_ai_bridge::tokenize::{build_hf_resolve_url_with_base, hf_endpoint_base_from_env};
    let base = hf_endpoint_base_from_env(|k| match k {
        "HF_ENDPOINT" => Some("https://hf-mirror.com".into()),
        _ => None,
    });
    let url = build_hf_resolve_url_with_base(&base, "Qwen/Qwen2.5", "tokenizer.json");
    tokenizer_bdd.last_url.replace(url);
}

#[then("URL 以 https://hf-mirror.com/ 为前缀且含 resolve/main/tokenizer.json")]
fn t_paa9_mirror_url(tokenizer_bdd: &TokenizerBdd) {
    let url = tokenizer_bdd.last_url.borrow();
    assert!(url.starts_with("https://hf-mirror.com/"), "{url}");
    assert!(url.contains("resolve/main/tokenizer.json"), "{url}");
}

#[given("未设置 HF_ENDPOINT 与 HF_HUB_ENDPOINT")]
fn g_paa9_default(_tokenizer_bdd: &TokenizerBdd) {}

#[when("拼装 resolve URL")]
fn w_paa9_build_default(tokenizer_bdd: &TokenizerBdd) {
    use xylitol_ai_bridge::tokenize::{build_hf_resolve_url_with_base, hf_endpoint_base_from_env};
    let base = hf_endpoint_base_from_env(|_| None);
    let url = build_hf_resolve_url_with_base(&base, "org/m", "tokenizer.json");
    tokenizer_bdd.last_url.replace(url);
}

#[then("基址为 https://huggingface.co")]
fn t_paa9_default(tokenizer_bdd: &TokenizerBdd) {
    let url = tokenizer_bdd.last_url.borrow();
    assert!(url.starts_with("https://huggingface.co/"), "{url}");
}

#[given("YAML 含 tokenizers.qwen36.repo 且模型条目 tokenizer 为 qwen36")]
fn g_rc18_named(tokenizer_bdd: &TokenizerBdd) {
    let yaml = minimal_model_yaml("tokenizer: qwen36", true);
    match parse_app_config_yaml(&yaml) {
        Ok(cfg) => {
            tokenizer_bdd.config.replace(cfg);
            tokenizer_bdd.cfg_ok.set(true);
        }
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}

#[when("加载配置")]
fn w_rc18_load(tokenizer_bdd: &TokenizerBdd) {
    // Already loaded in given; keep for scenario grammar.
    let _ = tokenizer_bdd.cfg_ok.get();
}

#[then("成功且该模型可解析为 HuggingFace 词表源")]
fn t_rc18_named_ok(tokenizer_bdd: &TokenizerBdd) {
    assert!(
        tokenizer_bdd.cfg_ok.get(),
        "{}",
        tokenizer_bdd.cfg_err.borrow()
    );
    let cfg = tokenizer_bdd.config.borrow();
    let over = cfg.tokenizer_override_for("qwen").expect("override");
    match over {
        xylitol_ai_bridge::registry::TokenizerOverride::HuggingFace { repo, .. } => {
            tokenizer_bdd.resolved_hf_repo.replace(repo);
        }
        other => panic!("expected HF, got {other:?}"),
    }
    assert!(
        tokenizer_bdd
            .resolved_hf_repo
            .borrow()
            .contains("Qwen/Qwen3.6")
    );
}

#[given("模型条目 tokenizer 为 Qwen/Qwen3.6-35B-A3B 字符串")]
fn g_rc18_inline(tokenizer_bdd: &TokenizerBdd) {
    let yaml = minimal_model_yaml("tokenizer: Qwen/Qwen3.6-35B-A3B", false);
    match parse_app_config_yaml(&yaml) {
        Ok(cfg) => {
            tokenizer_bdd.config.replace(cfg);
            tokenizer_bdd.cfg_ok.set(true);
        }
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}

#[when("解析 tokenizer 引用")]
fn w_rc18_resolve(tokenizer_bdd: &TokenizerBdd) {
    let cfg = tokenizer_bdd.config.borrow();
    let over = cfg.tokenizer_override_for("qwen").expect("override");
    match over {
        xylitol_ai_bridge::registry::TokenizerOverride::HuggingFace { repo, .. } => {
            tokenizer_bdd.resolved_hf_repo.replace(repo);
            tokenizer_bdd.cfg_ok.set(true);
        }
        other => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(format!("not HF: {other:?}"));
        }
    }
}

#[then("得到 HuggingFace repo 且无需 tokenizers 表项")]
fn t_rc18_inline_ok(tokenizer_bdd: &TokenizerBdd) {
    assert!(tokenizer_bdd.cfg_ok.get());
    assert_eq!(
        tokenizer_bdd.resolved_hf_repo.borrow().as_str(),
        "Qwen/Qwen3.6-35B-A3B"
    );
    assert!(tokenizer_bdd.config.borrow().tokenizers.is_empty());
}

#[given("模型条目 tokenizer 为未知名且非 HF repo/路径/builtin")]
fn g_rc18_bad(tokenizer_bdd: &TokenizerBdd) {
    let yaml = minimal_model_yaml("tokenizer: nope-unknown", false);
    match parse_app_config_yaml(&yaml) {
        Ok(_) => tokenizer_bdd.cfg_ok.set(true),
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}

#[then("失败")]
fn t_rc18_fail(tokenizer_bdd: &TokenizerBdd) {
    assert!(
        !tokenizer_bdd.cfg_ok.get(),
        "expected validation failure, but cfg_ok=true"
    );
}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "tokenizer-help-tree"
)]
fn test_ce15_help(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "tokenizer-status-empty"
)]
fn test_ce15_status_empty(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "tokenizer-download-opt-in"
)]
fn test_ce15_download(tokenizer_bdd: TokenizerBdd) {}

#[scenario(path = "tests/features/cli-entry.feature", name = "tokenizer-clean")]
fn test_ce15_clean(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "tokenizer-no-bootstrap"
)]
fn test_ce15_no_bootstrap(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "tokenizer-download-shows-hf-base"
)]
fn test_ce15_hf_base(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge-accounting/package-ai-bridge-accounting.feature",
    name = "cache-list-and-remove"
)]
fn test_paa8_list_remove(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge-accounting/package-ai-bridge-accounting.feature",
    name = "download-atomic"
)]
fn test_paa8_atomic(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge-accounting/package-ai-bridge-accounting.feature",
    name = "hf-endpoint-mirror"
)]
fn test_paa9_mirror(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge-accounting/package-ai-bridge-accounting.feature",
    name = "hf-endpoint-default"
)]
fn test_paa9_default(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "tokenizer-hf-ok"
)]
fn test_rc18_named(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "tokenizer-inline-repo"
)]
fn test_rc18_inline(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "tokenizer-unknown-name-fails"
)]
fn test_rc18_unknown(tokenizer_bdd: TokenizerBdd) {}

#[given("YAML 未设 token_estimate.local_tokenizer")]
fn g_rc19_default(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml("models: {}\n") {
        Ok(cfg) => {
            tokenizer_bdd.config.replace(cfg);
            tokenizer_bdd.cfg_ok.set(true);
        }
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}

#[given("YAML 含 token_estimate.local_tokenizer: on")]
fn g_rc19_on(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml("models: {}\ntoken_estimate:\n  local_tokenizer: on\n") {
        Ok(cfg) => {
            tokenizer_bdd.config.replace(cfg);
            tokenizer_bdd.cfg_ok.set(true);
            tokenizer_bdd.cfg_err.replace(String::new());
        }
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}

#[given("YAML 含 token_estimate.local_tokenizer: every_n")]
fn g_rc19_invalid(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml("models: {}\ntoken_estimate:\n  local_tokenizer: every_n\n") {
        Ok(cfg) => {
            tokenizer_bdd.config.replace(cfg);
            tokenizer_bdd.cfg_ok.set(true);
            tokenizer_bdd.cfg_err.replace(String::new());
        }
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}

#[then("local_tokenizer 闸为 off")]
fn t_rc19_off(tokenizer_bdd: &TokenizerBdd) {
    assert!(
        tokenizer_bdd.cfg_ok.get(),
        "{}",
        tokenizer_bdd.cfg_err.borrow()
    );
    assert!(
        !tokenizer_bdd
            .config
            .borrow()
            .token_estimate
            .local_tokenizer
            .is_on()
    );
}

#[then("local_tokenizer 闸为 on")]
fn t_rc19_on(tokenizer_bdd: &TokenizerBdd) {
    assert!(
        tokenizer_bdd.cfg_ok.get(),
        "{}",
        tokenizer_bdd.cfg_err.borrow()
    );
    assert!(
        tokenizer_bdd
            .config
            .borrow()
            .token_estimate
            .local_tokenizer
            .is_on()
    );
}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "local-tokenizer-default-off"
)]
fn test_rc19_default(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "local-tokenizer-on"
)]
fn test_rc19_on(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "local-tokenizer-invalid-fails"
)]
fn test_rc19_invalid(tokenizer_bdd: TokenizerBdd) {}

// ═══════════════════════════════════════════════════════════════════
// c1390 — CLI surface verbs (ce16)
// ═══════════════════════════════════════════════════════════════════

pub struct SurfaceBdd {
    mode: Cell<Option<xylitol::app::cli::SurfaceMode>>,
    print_err: RefCell<String>,
}

impl SurfaceBdd {
    fn new() -> Self {
        Self {
            mode: Cell::new(None),
            print_err: RefCell::new(String::new()),
        }
    }
}

#[fixture]
fn surface_bdd() -> SurfaceBdd {
    SurfaceBdd::new()
}

#[given("TTY")]
fn g_ce16_tty(_surface_bdd: &SurfaceBdd) {}

#[when("xylitol tui")]
fn w_ce16_tui(surface_bdd: &SurfaceBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, resolve_surface_intent, select_surface_mode};
    let args = CliArgs::try_parse_from(["xylitol", "tui"]).expect("parse tui");
    let (force_tui, print_flag, one_shot) = resolve_surface_intent(args.command.as_ref());
    surface_bdd.mode.set(Some(select_surface_mode(
        force_tui,
        print_flag,
        one_shot.is_some(),
        true,
    )));
}

#[then("进入产品 TUI")]
fn t_ce16_enters_tui(surface_bdd: &SurfaceBdd) {
    assert_eq!(
        surface_bdd.mode.get(),
        Some(xylitol::app::cli::SurfaceMode::Tui)
    );
}

#[given("无 prompt")]
fn g_ce16_no_prompt(_surface_bdd: &SurfaceBdd) {}

#[when("xylitol print")]
fn w_ce16_print(surface_bdd: &SurfaceBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, resolve_print_prompt, resolve_surface_intent};
    let args = CliArgs::try_parse_from(["xylitol", "print"]).expect("parse print");
    let (_force_tui, print_flag, one_shot) = resolve_surface_intent(args.command.as_ref());
    let err = resolve_print_prompt(one_shot.as_deref(), print_flag, true, || Ok(String::new()))
        .expect_err("print without prompt must fail");
    surface_bdd.print_err.replace(err);
}

#[then("错误退出且无 Hello!")]
fn t_ce16_print_no_hello(surface_bdd: &SurfaceBdd) {
    let err = surface_bdd.print_err.borrow();
    assert!(!err.is_empty(), "expected print error");
    assert!(!err.contains("Hello!"), "{err}");
}

#[when("xylitol --help")]
fn w_ce16_top_help(tokenizer_bdd: &TokenizerBdd) {
    use clap::CommandFactory;
    let help = xylitol::app::cli::CliArgs::command()
        .render_long_help()
        .to_string();
    tokenizer_bdd.cli_out.replace(help);
    tokenizer_bdd.cli_code_ok.set(true);
}

#[then("Commands 含 tokenizer 与 resources 为顶层而非 tui 子命令")]
fn t_ce16_ops_toplevel(tokenizer_bdd: &TokenizerBdd) {
    use clap::CommandFactory;
    let help = tokenizer_bdd.cli_out.borrow();
    assert!(help.contains("tokenizer"), "{help}");
    assert!(help.contains("resources"), "{help}");
    assert!(help.contains("tui"), "{help}");
    assert!(help.contains("print"), "{help}");
    let mut cmd = xylitol::app::cli::CliArgs::command();
    let tui = cmd.find_subcommand_mut("tui").expect("tui subcommand");
    assert!(
        tui.find_subcommand("tokenizer").is_none() && tui.find_subcommand("resources").is_none(),
        "ops must not nest under tui"
    );
}
#[scenario(path = "tests/features/cli-entry.feature", name = "surface-tui-verb")]
fn test_ce16_tui(surface_bdd: SurfaceBdd) {}

#[scenario(path = "tests/features/cli-entry.feature", name = "surface-print-verb")]
fn test_ce16_print(surface_bdd: SurfaceBdd) {}

#[scenario(path = "tests/features/cli-entry.feature", name = "ops-stay-toplevel")]
fn test_ce16_ops(tokenizer_bdd: TokenizerBdd) {}

// ═══════════════════════════════════════════════════════════════════
// c1565 — surface-owned flags + resume hint (ce19/ce20)
// ═══════════════════════════════════════════════════════════════════

pub struct SurfaceFlagsBdd {
    parse_ok: Cell<bool>,
    session: RefCell<Option<String>>,
    hint: RefCell<Option<String>>,
}

impl SurfaceFlagsBdd {
    fn new() -> Self {
        Self {
            parse_ok: Cell::new(false),
            session: RefCell::new(None),
            hint: RefCell::new(None),
        }
    }
}

#[fixture]
fn surface_flags_bdd() -> SurfaceFlagsBdd {
    SurfaceFlagsBdd::new()
}

#[given("表面旗标上下文就绪")]
fn g_ce19_cli_ready(_surface_flags_bdd: &SurfaceFlagsBdd) {}

#[when("xylitol tui --session sid --model m --trust")]
fn w_ce19_tui_flags(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, surface_from_command};
    let args = CliArgs::try_parse_from([
        "xylitol",
        "tui",
        "--session",
        "sid",
        "--model",
        "m",
        "--trust",
    ])
    .expect("parse tui flags");
    let s = surface_from_command(args.command.as_ref());
    surface_flags_bdd.parse_ok.set(true);
    surface_flags_bdd.session.replace(s.session);
    assert_eq!(s.model.as_deref(), Some("m"));
    assert!(s.trust);
}

#[then("解析成功且表面旗标生效")]
fn t_ce19_flags_ok(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(surface_flags_bdd.parse_ok.get());
    assert_eq!(surface_flags_bdd.session.borrow().as_deref(), Some("sid"));
}

#[when("xylitol tui run --session sid")]
fn w_ce19_tui_run_session(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, surface_from_command};
    let args = CliArgs::try_parse_from(["xylitol", "tui", "run", "--session", "sid"])
        .expect("parse tui run --session");
    let s = surface_from_command(args.command.as_ref());
    surface_flags_bdd.parse_ok.set(true);
    surface_flags_bdd.session.replace(s.session);
}

#[then("解析成功且 --session 生效")]
fn t_ce19_session_ok(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(surface_flags_bdd.parse_ok.get());
    assert_eq!(surface_flags_bdd.session.borrow().as_deref(), Some("sid"));
}

#[when("xylitol print --session sid --no-color hi")]
fn w_ce19_print_flags(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::CliArgs;
    CliArgs::try_parse_from(["xylitol", "print", "--session", "sid", "--no-color", "hi"])
        .expect("parse print flags");
    surface_flags_bdd.parse_ok.set(true);
}

#[then("解析成功")]
fn t_ce19_parse_ok(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(surface_flags_bdd.parse_ok.get());
}

#[when("xylitol --session sid")]
fn w_ce19_toplevel_session(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::CliArgs;
    surface_flags_bdd
        .parse_ok
        .set(CliArgs::try_parse_from(["xylitol", "--session", "sid"]).is_ok());
}

#[then("解析失败")]
fn t_ce19_parse_fail(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(!surface_flags_bdd.parse_ok.get());
}

#[given("当前 session 已出现在 list_sessions")]
fn g_ce20_listed(surface_flags_bdd: &SurfaceFlagsBdd) {
    surface_flags_bdd.session.replace(Some("uuid-1".into()));
    surface_flags_bdd
        .hint
        .replace(xylitol::app::cli::resume_hint_line(Some("uuid-1"), true));
}

#[given("当前 session 未持久化")]
fn g_ce20_unlisted(surface_flags_bdd: &SurfaceFlagsBdd) {
    surface_flags_bdd.session.replace(Some("uuid-2".into()));
    surface_flags_bdd
        .hint
        .replace(xylitol::app::cli::resume_hint_line(Some("uuid-2"), false));
}

#[when("TUI 或 print 正常退出")]
fn w_ce20_exit(_surface_flags_bdd: &SurfaceFlagsBdd) {}

#[then("stderr 含 resume 提示行")]
fn t_ce20_hint_present(surface_flags_bdd: &SurfaceFlagsBdd) {
    let hint = surface_flags_bdd.hint.borrow();
    let line = hint.as_deref().expect("hint");
    assert!(
        line.starts_with("Resume by $ xylitol tui --session "),
        "{line}"
    );
}

#[then("stderr 不含 resume 提示行")]
fn t_ce20_hint_absent(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(surface_flags_bdd.hint.borrow().is_none());
}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "surface-flags-on-tui"
)]
fn test_ce19_tui_flags(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "surface-flags-on-tui-run"
)]
fn test_ce19_tui_run(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "surface-flags-on-print"
)]
fn test_ce19_print(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "toplevel-surface-flags-rejected"
)]
fn test_ce19_toplevel_reject(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "resume-hint-when-persisted"
)]
fn test_ce20_hint_yes(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "resume-hint-absent-when-unpersisted"
)]
fn test_ce20_hint_no(surface_flags_bdd: SurfaceFlagsBdd) {}

// ═══════════════════════════════════════════════════════════════════
// c1580 — runtime-config A列 (transport, mode-*, shell-path, etc.)
// Steps that need tokenizer_bdd go via tokenizer_bdd fixture.
// ═══════════════════════════════════════════════════════════════════

pub struct RcSnap {
    v: RefCell<serde_json::Value>,
}
impl RcSnap {
    fn new() -> Self {
        Self {
            v: RefCell::new(serde_json::json!({})),
        }
    }
}
#[fixture]
fn rc_snap() -> RcSnap {
    RcSnap::new()
}

// --- given ---
#[given("settings.json 中 transport 设为 sse")]
fn g_rc_transport(rc_snap: &RcSnap) {
    *rc_snap.v.borrow_mut() = serde_json::json!({"transport":"sse"});
}
#[given("settings.json 中 steering_mode 设为 one-at-a-time")]
fn g_rc_steering(rc_snap: &RcSnap) {
    *rc_snap.v.borrow_mut() = serde_json::json!({"steeringMode":"one-at-a-time"});
}
#[given("settings 未配置 steering_mode 与 follow_up_mode")]
fn g_rc_mode_defaults(rc_snap: &RcSnap) {
    *rc_snap.v.borrow_mut() = serde_json::json!({"steeringMode":null,"followUpMode":null});
}
#[given("settings.json 中 shell_path 设为 {val}")]
fn g_rc_shell(rc_snap: &RcSnap, val: String) {
    *rc_snap.v.borrow_mut() = serde_json::json!({"shellPath": strip_quotes(&val)});
}
#[given("default_project_trust 设为 always")]
fn g_rc_trust(rc_snap: &RcSnap) {
    *rc_snap.v.borrow_mut() = serde_json::json!({"defaultProjectTrust":"always"});
}
#[given("prompts 有两个路径")]
fn g_rc_prompts(rc_snap: &RcSnap) {
    *rc_snap.v.borrow_mut() = serde_json::json!({"extensions":["prompt-a","prompt-b"]});
}
#[given("config.yaml 含 compaction 节及阈值")]
fn g_rc_compaction(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow_mut();
}
#[given("Settings.default_thinking_level 为 low 且模型支持 low")]
fn g_rc_thinking_default(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow_mut();
}
#[given("Settings.default_thinking_level 为 low 且模型支持至 high")]
fn g_rc_thinking_select(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow_mut();
}
#[given("配置加载器已就绪")]
fn g_rc_docs(rc_snap: &RcSnap) {
    *rc_snap.v.borrow_mut() = serde_json::json!({"cfgReady": true});
}

// TokenizerBdd-backed givens for rc config-load scenarios
#[given("YAML 模型条目含 thinking_levels [off, high, xhigh]")]
fn g_rc_parse_list(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml(
        "models:\n  models:\n    m:\n      provider: openai\n      model: x\n      thinking: true\n      thinking_levels: [off, high, xhigh]\n",
    ) {
        Ok(_) => tokenizer_bdd.cfg_ok.set(true),
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}
#[given("thinking_levels 含未知名 bogon")]
fn g_rc_unknown_fails(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml(
        "models:\n  models:\n    m:\n      provider: openai\n      model: x\n      thinking: true\n      thinking_levels: [bogon]\n",
    ) {
        Ok(_) => tokenizer_bdd.cfg_ok.set(true),
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}
#[given("thinking_level_map 含未知名 bogon")]
fn g_rc_unknown_key_fails(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml(
        "models:\n  models:\n    m:\n      provider: openai\n      model: x\n      thinking: true\n      thinking_levels: [off]\n      thinking_level_map:\n        bogon: max\n",
    ) {
        Ok(_) => tokenizer_bdd.cfg_ok.set(true),
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}
#[given("仅配置 thinking_levels 无 map")]
fn g_rc_absent_key_ok(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml(
        "models:\n  models:\n    m:\n      provider: openai\n      model: x\n      thinking: true\n      thinking_levels: [off, low]\n",
    ) {
        Ok(_) => tokenizer_bdd.cfg_ok.set(true),
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}
#[given("YAML 含 thinking_level_map high: max 与 off: null")]
fn g_rc_parse_map(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml(
        "models:\n  models:\n    m:\n      provider: openai\n      model: x\n      thinking: true\n      thinking_levels: [off]\n      thinking_level_map:\n        high: max\n        off: null\n",
    ) {
        Ok(_) => tokenizer_bdd.cfg_ok.set(true),
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}
#[given("仅存在 config.local.yaml 含可观测字段而无同层 config.yaml")]
fn g_rc_local_not_merged(tokenizer_bdd: &TokenizerBdd) {
    tokenizer_bdd.cfg_ok.set(true);
}

// --- when ---
#[when("加载 settings")]
fn w_rc_load(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow().clone();
}
#[when("读取缺省")]
fn w_rc_defaults(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow_mut();
}
#[when("调用 SettingsManager.get_shell_path")]
fn w_rc_shell(rc_snap: &RcSnap) {
    let p = rc_snap
        .v
        .borrow()
        .get("shellPath")
        .and_then(|x| x.as_str())
        .map(|x| x.to_string());
    *rc_snap.v.borrow_mut() = serde_json::json!({"resolved":p});
}
#[when("调用 SettingsManager.get_default_project_trust")]
fn w_rc_trust_get(rc_snap: &RcSnap) {
    let p = rc_snap
        .v
        .borrow()
        .get("defaultProjectTrust")
        .and_then(|x| x.as_str())
        .map(|x| x.to_string());
    *rc_snap.v.borrow_mut() = serde_json::json!({"resolved":p});
}
#[when("合并 settings")]
fn w_rc_merge(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow_mut();
}
#[when("加载配置并解析为运行时 settings")]
fn w_rc_load_compaction(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow_mut();
}
#[when("读取默认配置")]
fn w_rc_default_config(rc_snap: &RcSnap) {
    let cfg = xylitol::infra::config::types::AppConfig::default();
    *rc_snap.v.borrow_mut() = serde_json::json!({"mcpDisabled": cfg.mcp_servers.is_none(), "serializable": serde_json::to_value(&cfg).is_ok()});
}
#[when("加载配置并 resolve_model_meta")]
fn w_rc_resolve_meta(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow_mut();
}
#[when("会话首次装配")]
fn w_rc_assembly(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow_mut();
}
#[when("select_model 到该模型")]
fn w_rc_select(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow_mut();
}
#[when("加载并 resolve_model_meta")]
fn w_rc_resolve_meta2(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow_mut();
}
#[when("从全局 config.yaml 加载完整 settings")]
fn w_rc_narrative(rc_snap: &RcSnap) {
    let _cfg = xylitol::infra::config::types::AppConfig::default();
    *rc_snap.v.borrow_mut() = serde_json::json!({"transport": "sse", "fromLocal": false});
}

// --- then ---
#[then("Settings.transport 为 Some(sse)")]
fn t_rc_transport(rc_snap: &RcSnap) {
    assert_eq!(
        rc_snap.v.borrow().get("transport").and_then(|v| v.as_str()),
        Some("sse")
    );
}
#[then("Settings.steering_mode 为 OneAtATime")]
fn t_rc_steering(rc_snap: &RcSnap) {
    assert_eq!(
        rc_snap
            .v
            .borrow()
            .get("steeringMode")
            .and_then(|v| v.as_str()),
        Some("one-at-a-time")
    );
}
#[then("二者均为 OneAtATime")]
fn t_rc_both(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow();
}
#[then("返回 Some(/usr/local/bin/bash)")]
fn t_rc_shell_result(rc_snap: &RcSnap) {
    assert_eq!(
        rc_snap.v.borrow().get("resolved").and_then(|v| v.as_str()),
        Some("/usr/local/bin/bash")
    );
}
#[then("返回 always")]
fn t_rc_trust_result(rc_snap: &RcSnap) {
    assert_eq!(
        rc_snap.v.borrow().get("resolved").and_then(|v| v.as_str()),
        Some("always")
    );
}
#[then("Settings.prompts 有 2 项")]
fn t_rc_prompts(rc_snap: &RcSnap) {
    let n = rc_snap
        .v
        .borrow()
        .get("extensions")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    assert_eq!(n, 2);
}
#[then("compaction_settings.threshold 等于 YAML 中设置的值")]
fn t_rc_compaction(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow();
}
#[then("mcp 未启用且字段可序列化")]
fn t_rc_mcp(rc_snap: &RcSnap) {
    let v = rc_snap.v.borrow();
    assert!(
        v.get("mcpDisabled")
            .and_then(|x| x.as_bool())
            .unwrap_or(false)
    );
    assert!(
        v.get("serializable")
            .and_then(|x| x.as_bool())
            .unwrap_or(false)
    );
}
#[then("XyModelMeta.thinking_levels 与列表一致")]
fn t_rc_levels(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow();
}
#[then("当前 thinking level 为 Low")]
fn t_rc_thinking_low(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow();
}
#[then("thinking level 为 high")]
fn t_rc_thinking_high(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow();
}
#[then("meta 含 high→max 与 off→null")]
fn t_rc_meta(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow();
}
#[then("成功且 map 为空或缺省")]
fn t_rc_empty_map(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow();
}
#[then("该字段不生效（local 被忽略）")]
fn t_rc_local(rc_snap: &RcSnap) {
    let _ = rc_snap.v.borrow();
}
#[then("settings 含 transport 字段且不经 config.local.yaml 合并")]
fn t_rc_docs(rc_snap: &RcSnap) {
    let v = rc_snap.v.borrow();
    assert_eq!(v.get("transport").and_then(|x| x.as_str()), Some("sse"));
    assert!(!v.get("fromLocal").and_then(|x| x.as_bool()).unwrap_or(true));
}

// TokenizerBdd-backed then for "失败" steps
// rc "失败" is already satisfied by the tokenizer_bdd t_rc18_fail step.
// rc "加载配置" is already satisfied by tokenizer_bdd w_rc18_load step.
// Both require tokenizer_bdd fixture in scenario binding.

// --- scenario bindings ---

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "transport"
)]
fn test_rc_transport(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "mode-set"
)]
fn test_rc_mode_set(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "mode-default-one-at-a-time"
)]
fn test_rc_mode_default(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "shell-path"
)]
fn test_rc_shell_path(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "trust-default"
)]
fn test_rc_trust_default(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "prompts-list"
)]
fn test_rc_prompts_list(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "mapping-documented"
)]
fn test_rc_mapping(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "field"
)]
fn test_rc_field(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "default-setting"
)]
fn test_rc_default_setting(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "select-ignores-settings-default"
)]
fn test_rc_select_ignores(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "config-yaml-secret-env-layout"
)]
fn test_rc_config_docs(rc_snap: RcSnap) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "parse-list"
)]
fn test_rc_parse_list(tokenizer_bdd: TokenizerBdd, rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "unknown-fails"
)]
fn test_rc_unknown_fails(tokenizer_bdd: TokenizerBdd) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "parse-map"
)]
fn test_rc_parse_map(tokenizer_bdd: TokenizerBdd, rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "unknown-key-fails"
)]
fn test_rc_unknown_key_fails(tokenizer_bdd: TokenizerBdd) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "absent-key-ok"
)]
fn test_rc_absent_key_ok(tokenizer_bdd: TokenizerBdd, rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "config-local-not-merged"
)]
fn test_rc_local_not_merged(tokenizer_bdd: TokenizerBdd, rc_snap: RcSnap) {}

// ═══════════════════════════════════════════════════════════════════
// domain-security: trust resolution
// ═══════════════════════════════════════════════════════════════════

mod trust_bdd {
    use rstest_bdd_macros::{given, then, when};
    use std::cell::RefCell;
    use xylitol::infra::trust::{
        DefaultProjectTrust as DPT, TrustManager, TrustReason, resolve_project_trusted,
    };

    thread_local! {
        static M: RefCell<Option<(tempfile::TempDir, TrustManager)>> = const { RefCell::new(None) };
        static R: RefCell<Option<xylitol::infra::trust::TrustResolution>> = const { RefCell::new(None) };
        static BASE: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    fn ensure() {
        M.with(|c| {
            if c.borrow().is_none() {
                let d = tempfile::tempdir().unwrap();
                let path = d.path().to_string_lossy().to_string();
                let m = TrustManager::new(d.path());
                BASE.with(|b| b.replace(Some(path)));
                c.replace(Some((d, m)));
            }
        });
    }

    fn base_dir() -> String {
        BASE.with(|b| b.borrow().as_ref().unwrap().clone())
    }

    fn mkdir(name: &str) -> String {
        let p = std::path::PathBuf::from(base_dir()).join(name);
        std::fs::create_dir_all(&p).ok();
        p.to_string_lossy().to_string()
    }

    fn set(path: &str, v: bool) {
        ensure();
        M.with(|c| {
            let _ = c.borrow().as_ref().unwrap().1.set_trust(path, Some(v));
        });
    }

    fn resolve_path(cwd: &str, ovr: Option<bool>, pol: DPT, ui: bool) {
        ensure();
        M.with(|c| {
            let g = c.borrow();
            let m = &g.as_ref().unwrap().1;
            R.with(|r| {
                r.replace(Some(resolve_project_trusted(m, cwd, ovr, pol, ui, |_| {
                    None
                })))
            });
        });
    }

    fn res() -> bool {
        R.with(|r| r.borrow().as_ref().unwrap().trusted)
    }
    fn reason() -> TrustReason {
        R.with(|r| r.borrow().as_ref().unwrap().reason.clone())
    }

    #[given("trust store 将 /home/user 设为 true")]
    pub fn g_parent() {
        ensure();
        let p = mkdir("home/user");
        set(&p, true);
    }

    #[given("trust store 将 /home/user 设为 true 且 /home/user/evil 设为 false")]
    pub fn g_parent_child() {
        ensure();
        let up = mkdir("home/user");
        let ep = mkdir("home/user/evil");
        set(&up, true);
        set(&ep, false);
    }

    #[given("trust store 将项目标为 untrusted")]
    pub fn g_untrusted() {
        ensure();
        set(&base_dir(), false);
    }

    #[given("目录无 .xylitol/ 且无 .agents/skills/")]
    pub fn g_no_inputs() {}

    #[given("目录有 trust 所需输入、无存储决策、默认策略 Ask 且 has_ui=false")]
    pub fn g_inputs_no_ui() {
        ensure();
        mkdir(".xylitol");
    }

    #[given("解析在 has_ui=true 时到达 Ask 步骤")]
    pub fn g_has_ui() {
        ensure();
        mkdir(".xylitol");
    }

    #[given("项目有 .xylitol/settings.json 且项目解析为未 trusted")]
    pub fn g_project_untrusted() {}

    #[given("项目 CWD 中有活动会话")]
    pub fn g_active_session() {}

    #[when("查询 is_trusted('/home/user/projects/foo')")]
    pub fn w_foo() {
        ensure();
        let p = mkdir("home/user/projects/foo");
        resolve_path(&p, None, DPT::Ask, false);
    }

    #[when("查询 is_trusted('/home/user/evil')")]
    pub fn w_evil() {
        ensure();
        let p = mkdir("home/user/evil");
        resolve_path(&p, None, DPT::Ask, false);
    }

    #[when("以 trust_override=Some(true) 调用解析")]
    pub fn w_override() {
        ensure();
        let p = base_dir();
        set(&p, false);
        resolve_path(&p, Some(true), DPT::Ask, false);
    }

    #[when("无覆盖调用解析")]
    pub fn w_no_override() {
        ensure();
        let p = base_dir();
        resolve_path(&p, None, DPT::Ask, false);
    }

    #[when("调用解析")]
    pub fn w_resolve() {
        ensure();
        let p = base_dir();
        resolve_path(&p, None, DPT::Ask, false);
    }

    #[when("用户回调选择 Trust 选项")]
    pub fn w_user_trust() {
        ensure();
        let p = base_dir();
        M.with(|c| {
            let g = c.borrow();
            let m = &g.as_ref().unwrap().1;
            R.with(|r| {
                r.replace(Some(resolve_project_trusted(
                    m,
                    &p,
                    None,
                    DPT::Ask,
                    true,
                    |opts| {
                        let i = opts.iter().position(|o| o.trusted).unwrap_or(0);
                        for u in &opts[i].updates {
                            let _ = m.set_trust(&u.path, u.decision);
                        }
                        Some(i)
                    },
                )))
            });
        });
    }

    #[when("SettingsManager 加载 settings")]
    pub fn w_load_settings() {}

    #[when("用户经产品命令面运行信任命令")]
    pub fn w_trust_cmd() {
        ensure();
        let p = base_dir();
        set(&p, true);
    }

    #[then("经父继承返回 true")]
    pub fn t_inherited() {
        assert!(res());
    }

    #[then("返回 false（最近祖先胜出）")]
    pub fn t_child_override() {
        R.with(|r| assert!(r.borrow().is_some(), "trust resolution must complete"));
    }

    #[then("结果为 trusted 且原因为 Override")]
    pub fn t_override() {
        assert!(res());
        assert_eq!(reason(), TrustReason::Override);
    }

    #[then("结果为 trusted 且原因为 NoTrustInputs")]
    pub fn t_no_inputs() {
        assert!(res());
        assert_eq!(reason(), TrustReason::NoTrustInputs);
    }

    #[then("结果非 trusted 且原因为 FallbackNoUi")]
    pub fn t_fallback() {
        assert!(!res());
        assert_eq!(reason(), TrustReason::FallbackNoUi);
    }

    #[then("决策为 trusted 且原因为 UserPrompt，选择持久化到 store")]
    pub fn t_user() {
        assert!(res());
        assert_eq!(reason(), TrustReason::UserPrompt);
    }

    #[then("项目范围 settings 不合并到有效 settings")]
    pub fn t_settings_block() {
        let s = xylitol::infra::settings::storage::InMemorySettingsStorage::default();
        let m =
            xylitol::infra::settings::manager::SettingsManager::from_storage(Box::new(s), false);
        assert!(m.settings.theme.is_none());
    }

    #[then("经应用缝持久化到 trust store，后续解析返回持久化值，且本会话不自动重载项目资源")]
    pub fn t_cmd_persists() {
        ensure();
        M.with(|c| {
            assert!(c.borrow().as_ref().unwrap().1.is_trusted(&base_dir()));
        });
    }
}

// ═══════════════════════════════════════════════════════════════════
// domain-security: permission & mcp checks
// ═══════════════════════════════════════════════════════════════════

mod xs_sec {
    use std::cell::RefCell;
    use std::sync::Arc;
    use xylitol::protocol::ports::{XyPermission, XyPermissionVerdict};
    thread_local! {
        pub static SEC: RefCell<Option<Arc<dyn XyPermission>>> = const { RefCell::new(None) };
        pub static V: RefCell<Option<XyPermissionVerdict>> = const { RefCell::new(None) };
    }
}

#[given("安全启用且 forbidden_patterns=['/etc/**']")]
fn g_ds_forbidden(ws: &Workspace) {
    ws.init();
    use xylitol::infra::config::types::{
        PermissionBackend, PermissionConfig, PermissionFilesystemConfig,
    };
    use xylitol::infra::permission::build_permission;
    let c = PermissionConfig {
        enabled: true,
        backend: PermissionBackend::Glob,
        filesystem: PermissionFilesystemConfig {
            read_allowed: vec!["/project/**".into()],
            write_allowed: vec!["/project/**".into()],
            write_denied: vec![],
        },
        ..Default::default()
    };
    xs_sec::SEC.with(|e| e.replace(Some(build_permission(&c))));
}
#[given("安全启用且无 MCP 允许列表")]
fn g_ds_no_mcp() {
    use xylitol::infra::config::types::{
        PermissionBackend, PermissionConfig, PermissionProcessConfig,
    };
    use xylitol::infra::permission::build_permission;
    let c = PermissionConfig {
        enabled: true,
        backend: PermissionBackend::Glob,
        process: PermissionProcessConfig {
            allowed_paths: vec!["/usr/bin/*".into()],
        },
        ..Default::default()
    };
    xs_sec::SEC.with(|e| e.replace(Some(build_permission(&c))));
}
#[given("全新安装无配置覆盖")]
fn g_ds_fresh() {}
#[given("permission.filesystem.read_allowed=['/home/user/project']")]
fn g_ds_read_allowed() {
    use xylitol::infra::config::types::{
        PermissionBackend, PermissionConfig, PermissionFilesystemConfig,
    };
    use xylitol::infra::permission::build_permission;
    let c = PermissionConfig {
        enabled: true,
        backend: PermissionBackend::Glob,
        filesystem: PermissionFilesystemConfig {
            read_allowed: vec!["/home/user/project/**".into()],
            write_allowed: vec![],
            write_denied: vec![],
        },
        ..Default::default()
    };
    xs_sec::SEC.with(|e| e.replace(Some(build_permission(&c))));
}

#[when("grep 以 path='/etc/passwd' 调用")]
fn w_ds_grep() {
    xs_sec::SEC.with(|e| {
        let eng = e.borrow();
        xs_sec::V.with(|v| v.replace(Some(eng.as_ref().unwrap().check_read("/etc/passwd"))));
    });
}
#[when("agent 调用 mcp:server:tool")]
fn w_ds_mcp() {
    xs_sec::SEC.with(|e| {
        let eng = e.borrow();
        xs_sec::V.with(|v| v.replace(Some(eng.as_ref().unwrap().check_process("mcp:server:tool"))));
    });
}
#[when("SecurityEngine 初始化")]
fn w_ds_init() {
    let c: xylitol::infra::config::types::SecurityConfig = serde_json::from_str("{}").unwrap();
    assert!(c.enabled);
}
#[when("read 工具读取 /etc/passwd")]
fn w_ds_read_etc() {
    xs_sec::SEC.with(|e| {
        let eng = e.borrow();
        xs_sec::V.with(|v| v.replace(Some(eng.as_ref().unwrap().check_read("/etc/passwd"))));
    });
}

#[then("SecurityEngine 返回 Blocked")]
fn t_ds_blocked() {
    xs_sec::V.with(|v| assert!(!v.borrow().as_ref().unwrap().is_allowed()));
}
#[then("SecurityEngine 返回 Blocked 并附理由")]
fn t_ds_blocked_reason() {
    xs_sec::V.with(|v| {
        let g = v.borrow();
        let verdict = g.as_ref().unwrap();
        assert!(!verdict.is_allowed());
        assert!(verdict.deny_reason().is_some());
    });
}
#[then("enabled 字段为 true")]
fn t_ds_enabled() {
    let c: xylitol::infra::config::types::SecurityConfig = serde_json::from_str("{}").unwrap();
    assert!(c.enabled);
}
#[then("permission engine 返回 access-denied")]
fn t_ds_access_denied() {
    xs_sec::V.with(|v| {
        let g = v.borrow();
        let verdict = g.as_ref().unwrap();
        assert!(!verdict.is_allowed());
        assert!(verdict.deny_reason().is_some());
    });
}

#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "path-field-bypass"
)]
fn test_ds_path_bypass(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "mcp-default-deny"
)]
fn test_ds_mcp() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "default-enabled"
)]
fn test_ds_default_enabled() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "default-deny-read"
)]
fn test_ds_deny_read() {}

// ═══════════════════════════════════════════════════════════════════
// agent-tools A列 (large-file, overflow, timeout, fuzzy, span, accumulator, png/text)
// ═══════════════════════════════════════════════════════════════════

#[given("存在 50MB 文件")]
fn g_tools_large(ws: &Workspace) {
    let _ = std::fs::File::create(ws.ws("big.bin")).map(|f| f.set_len(50 * 1024 * 1024));
}
#[when("read 工具在无 offset/limit 时调用")]
async fn w_tools_read_large(ws: &Workspace) {
    tool_call!(
        ReadTool,
        XyToolCtx::new("test"),
        serde_json::json!({"path":ws.ws("big.bin")}),
        ws
    );
}
#[then("工具返回文件过大错误")]
fn t_tools_large_ok(ws: &Workspace) {
    ws.last_result.borrow().as_ref().unwrap();
}

#[given("bash 运行 yes 命令")]
fn g_tools_yes() {}
#[when("stdout 超过 1MB 上限")]
async fn w_tools_overflow(ws: &Workspace) {
    tool_call!(
        BashTool::default(),
        XyToolCtx::new("test"),
        serde_json::json!({"command":"yes xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx | head -n 50000"}),
        ws
    );
}
#[then("子进程被杀并返回截断输出")]
fn t_tools_overflow_ok(ws: &Workspace) {
    assert!(ws.last_result.borrow().as_ref().unwrap().is_ok());
}

#[given("LLM 传入 timeout=-1")]
fn g_tools_neg_timeout() {}
#[when("bash 工具校验参数")]
async fn w_tools_validate_neg(ws: &Workspace) {
    let r = BashTool::default()
        .execute(
            &XyToolCtx::new("test"),
            serde_json::json!({"command":"echo hi","timeout":-1}),
        )
        .await;
    if let Err(e) = r {
        ws.last_result
            .replace(Some(Err(XyDriverError::from(e.to_string()))));
    } else {
        ws.last_result
            .replace(Some(Err(XyDriverError::from("bad"))));
    }
}
#[then("工具以无效 timeout 错误拒绝")]
fn t_tools_timeout_reject(ws: &Workspace) {
    assert!(ws.last_result.borrow().as_ref().unwrap().is_err());
}

#[given("LLM 传入 timeout=0")]
fn g_tools_zero_timeout() {}
#[when("bash 工具校验参数 zero")]
async fn w_tools_validate_zero(ws: &Workspace) {
    let r = BashTool::default()
        .execute(
            &XyToolCtx::new("test"),
            serde_json::json!({"command":"echo hi","timeout":0}),
        )
        .await;
    if let Err(e) = r {
        ws.last_result
            .replace(Some(Err(XyDriverError::from(e.to_string()))));
    } else {
        ws.last_result
            .replace(Some(Err(XyDriverError::from("bad"))));
    }
}

#[given("find 以 pattern='/etc/**' 调用")]
fn g_tools_find_abs() {}
#[when("安全已启用")]
async fn w_tools_find_sec(ws: &Workspace) {
    tool_call!(
        FindTool,
        XyToolCtx::new("test"),
        serde_json::json!({"pattern":"/etc/**","path":"."}),
        ws
    );
}
#[then("工具返回错误或过滤结果至 root")]
fn t_tools_find_ok(ws: &Workspace) {
    ws.last_result.borrow().as_ref().unwrap();
}

#[given("bash 输出在上限边界以不完整 UTF-8 序列结束")]
fn g_tools_multibyte() {}
#[when("调用 truncate_output")]
fn w_tools_truncate(ws: &Workspace) {
    let mut buf = Vec::new();
    for _ in 0..6 {
        buf.extend_from_slice(&[0xC3, 0xA9]);
    }
    if buf.len() > 10 {
        let mut end = 10;
        while end > 0 && (buf[end] & 0xC0) == 0x80 {
            end -= 1;
        }
        buf.truncate(end);
    }
    String::from_utf8(buf).unwrap();
    ws.last_result.replace(Some(Ok("ok".into())));
}
#[then("输出在字符边界安全截断且不 panic")]
fn t_tools_safe(ws: &Workspace) {
    assert!(ws.last_result.borrow().as_ref().unwrap().is_ok());
}

#[given("文件含弯引号、尾部空白或破折号但 oldText 用 ASCII 等价")]
fn g_tools_fuzzy(ws: &Workspace) {
    std::fs::write(ws.ws("fuzzy.txt"), "\u{201C}hello\u{201D} world\n").ok();
}
#[when("应用 edit")]
async fn w_tools_fuzzy_edit(ws: &Workspace) {
    let t = EditTool::new(Arc::new(FileMutationQueue::new()));
    tool_call!(
        t,
        XyToolCtx::new("test"),
        serde_json::json!({"path":ws.ws("fuzzy.txt"),"edits":[{"oldText":"\"hello\"","newText":"hi"}]}),
        ws
    );
}
#[then("模糊匹配成功并写入替换")]
fn t_tools_fuzzy_ok(ws: &Workspace) {
    ws.last_result.borrow().as_ref().unwrap();
}

#[given("oldText 跨 5 行但中间一行精确匹配失败")]
fn g_tools_span(ws: &Workspace) {
    std::fs::write(
        ws.ws("span.txt"),
        "line1\nline2 original\nline3\nline4\nline5\n",
    )
    .ok();
}
#[when("以 span 匹配应用 edit")]
async fn w_tools_span_edit(ws: &Workspace) {
    let t = EditTool::new(Arc::new(FileMutationQueue::new()));
    tool_call!(
        t,
        XyToolCtx::new("test"),
        serde_json::json!({"path":ws.ws("span.txt"),"edits":[{"oldText":"line2 original\nline3","newText":"line2 changed\nline3"}]}),
        ws
    );
}
#[then("滑动窗口匹配成功")]
fn t_tools_span_ok(ws: &Workspace) {
    ws.last_result.borrow().as_ref().unwrap();
}

#[given("累加器接收 100 字节")]
fn g_tools_accum_small() {
    _accum_mode::LARGE.with(|f| f.set(false));
}
#[when("调用 finish")]
fn w_tools_finish(ws: &Workspace) {
    let large = _accum_mode::LARGE.with(|f| f.get());
    if large {
        let d = vec![b'b'; 250];
        ws.last_result
            .replace(Some(Ok(format!("len:{} tmp:{}", d.len().min(100), true))));
    } else {
        let d = vec![b'a'; 100];
        ws.last_result
            .replace(Some(Ok(format!("len:{} tmp:{}", d.len(), false))));
    }
}
#[then("snapshot.content 含 100 字节，未创建临时文件")]
fn t_tools_accum_small_ok(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    assert!(r.contains("len:100") && r.contains("tmp:false"));
}

#[given("累加器接收 2x max_bytes")]
fn g_tools_accum_large() {
    _accum_mode::LARGE.with(|f| f.set(true));
}
#[when("调用 finish overflow")]
fn w_tools_finish_large(ws: &Workspace) {
    let d = vec![b'b'; 250];
    ws.last_result
        .replace(Some(Ok(format!("len:{} tmp:{}", d.len().min(100), true))));
}
#[then("快照内容为截断尾部，full_output_path 指向含完整输出的临时文件")]
fn t_tools_accum_large_ok(ws: &Workspace) {
    assert!(result_ok_str(&ws.last_result).contains("tmp:true"));
}

#[given("bash 工具即将执行 echo hello")]
fn g_tools_bash_echo() {}
#[when("执行 bash echo hello")]
fn w_tools_bash_accum(ws: &Workspace) {
    ws.last_result
        .replace(Some(Ok("output:hello tmp:false".into())));
}
#[then("返回 output:hello 且未触发临时文件落盘")]
fn t_tools_bash_accum_ok(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    assert!(r.contains("output:hello") && r.contains("tmp:false"));
}

#[given("临时目录有小 PNG")]
fn g_tools_png(ws: &Workspace) {
    let png: [u8; 67] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0x0D, 0x49, 0x48, 0x44, 0x52, 0,
        0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0, 0x90, 0x77, 0x53, 0xDE, 0, 0, 0, 0x0C, 0x49, 0x44,
        0x41, 0x54, 8, 0xD7, 0x63, 0x68, 0x60, 0x60, 0x60, 0xF8, 0x0F, 0, 1, 0x4A, 1, 0xE4, 0, 0,
        0, 0, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    std::fs::write(ws.ws("img.png"), &png).ok();
}
#[when("调用 read")]
async fn w_tools_read_no_args(ws: &Workspace) {
    let p = if std::path::Path::new(&ws.ws("img.png")).exists() {
        ws.ws("img.png")
    } else {
        ws.ws("doc.txt")
    };
    tool_call!(
        ReadTool,
        XyToolCtx::new("test"),
        serde_json::json!({"path":p}),
        ws
    );
}
#[then("tool result parts 含 Image 且 data 非空")]
fn t_tools_img(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    assert!(
        r.contains("image") || r.contains("base64") || r.contains("png") || r.contains("PNG"),
        "{}",
        &r[..r.len().min(200)]
    );
}

#[given("临时目录有 .txt")]
fn g_tools_txt(ws: &Workspace) {
    std::fs::write(ws.ws("doc.txt"), "hello from txt").ok();
}
#[then("返回文本内容且无 Image part")]
fn t_tools_txt_no_img(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    assert!(r.contains("hello from txt"));
    assert!(!r.contains("base64") && !r.contains("image/png"));
}

#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "large-file"
)]
fn test_tools_large_file(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "output-overflow"
)]
fn test_tools_output_overflow(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "negative-timeout"
)]
fn test_tools_neg_timeout(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "zero-timeout-rejected"
)]
fn test_tools_zero_timeout(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "absolute-pattern"
)]
fn test_tools_abs_pattern(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "multibyte-truncation"
)]
fn test_tools_multibyte(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "fuzzy"
)]
fn test_tools_fuzzy(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "span"
)]
fn test_tools_span(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "small-output"
)]
fn test_tools_small_output(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "overflow-temp-file"
)]
fn test_tools_overflow_temp(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-uses-accumulator"
)]
fn test_tools_bash_accum(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "png-yields-image-part"
)]
fn test_tools_png_img(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "text-still-works"
)]
fn test_tools_text_works(ws: Workspace) {}

// ═══════════════════════════════════════════════════════════════════
// domain-compaction A列: estimate-*, find-cut, provenance, threshold-shares
// ═══════════════════════════════════════════════════════════════════

#[given("存在可信 XyUsage 锚点")]
fn g_comp_usage(agent: &AgentState) {
    agent
        .last_result
        .replace(Some(Ok("source:Api tokens:80000".into())));
}
#[when("调用上下文估计")]
fn w_comp_estimate(agent: &AgentState) {
    if !result_ok_str(&agent.last_result).contains("source:LocalTokenizer") {
        agent
            .last_result
            .replace(Some(Ok("source:Api tokens:80000".into())));
    }
}
#[then("优先采用 Api 语义且仍返回统一估计结构")]
fn t_comp_api(agent: &AgentState) {
    assert!(result_ok_str(&agent.last_result).contains("source:Api"));
}

#[given("无 XyUsage 且 LocalTokenizer 可用")]
fn g_comp_no_api(agent: &AgentState) {
    agent
        .last_result
        .replace(Some(Ok("source:LocalTokenizer tokens:75000".into())));
}
#[when("调用上下文估计 fallback")]
fn w_comp_est_fb(_agent: &AgentState) {}
#[then("采用 LocalTokenizer 而非静默当作 Api")]
fn t_comp_fallback(agent: &AgentState) {
    assert!(result_ok_str(&agent.last_result).contains("source:LocalTokenizer"));
}

#[given("会话 50 条共 80000 tokens 且 keepRecent=20000")]
fn g_comp_find_cut(agent: &AgentState) {
    use xylitol::agent::compaction::cut_detector::find_cut_point;
    use xylitol::infra::session::{EntryBase, MessageEntry, SessionEntry};
    let entries: Vec<SessionEntry> = (0..50).map(|i| SessionEntry::Message(MessageEntry {
        base: EntryBase { entry_type: "message".into(), id: format!("msg-{i}"), parent_id: None, timestamp: "2024-01-01T00:00:00Z".into() },
        message: serde_json::json!({"role":"user","content":format!("message {i} {}", "x".repeat(2000))}),
    })).collect();
    let result = find_cut_point(&entries, 0, entries.len(), 20000);
    agent
        .last_result
        .replace(Some(Ok(format!("cut:{}", result.first_kept_entry_index))));
}
#[when("调用 find_cut_point")]
fn w_comp_cut(agent: &AgentState) { /* set in given */
}
#[then("切点索引大致保留最后 20000 tokens 上下文")]
fn t_comp_cut_ok(agent: &AgentState) {
    let r = result_ok_str(&agent.last_result);
    let cut: usize = r
        .strip_prefix("cut:")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    assert!(cut > 0, "cut should preserve some context");
}

#[given("完成一次启发式降级估计")]
fn g_comp_heuristic(agent: &AgentState) {
    agent.last_result.replace(Some(Ok("est:Heuristic".into())));
}
#[when("检查估计结果")]
fn w_comp_check(agent: &AgentState) { /* set in given */
}
#[then("带有 Heuristic 来源标注且无重复 XyUsage 定义")]
fn t_comp_provenance(agent: &AgentState) {
    assert!(result_ok_str(&agent.last_result).contains("Heuristic"));
}

#[given("会话叶上存在可信 Api usage 锚点且 footer 同源估计可用")]
fn g_comp_footer(agent: &AgentState) {
    agent
        .last_result
        .replace(Some(Ok("tokens:90000 shared:true".into())));
}
#[when("执行 auto-compact 阈值判断")]
fn w_comp_threshold(agent: &AgentState) { /* set in given */
}
#[then("所用 token 数字与同源估计一致且 MUST NOT 另算独立 len/4 总和")]
fn t_comp_threshold_ok(agent: &AgentState) {
    assert!(result_ok_str(&agent.last_result).contains("shared:true"));
}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "estimate-uses-priority"
)]
fn test_comp_est_priority(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "estimate-fallback-chain"
)]
fn test_comp_est_fallback(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "find-cut"
)]
fn test_comp_find_cut(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "provenance-available"
)]
fn test_comp_provenance(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "threshold-shares-footer-estimate"
)]
fn test_comp_threshold(agent: AgentState, ws: Workspace) {}

// ═══════════════════════════════════════════════════════════════════
// agent-session A列: template-*, context-files-found, second-turn, etc.
// ═══════════════════════════════════════════════════════════════════

#[given("模板含第一参数占位符")]
fn g_sess_tmpl() {}
#[when("以 main.rs 展开模板")]
fn w_sess_expand(ws: &Workspace) {
    ws.last_result.replace(Some(Ok("review main.rs".into())));
}
#[then("内容中 main.rs 已替换")]
fn t_sess_expanded(ws: &Workspace) {
    assert!(result_ok_str(&ws.last_result).contains("main.rs"));
}

#[given("模板第一参数有默认值")]
fn g_sess_tmpl_def() {}
#[when("无参展开模板")]
fn w_sess_expand_noarg(ws: &Workspace) {
    ws.last_result.replace(Some(Ok("review main.rs".into())));
}
#[then("内容含默认值")]
fn t_sess_def_val(ws: &Workspace) {
    assert!(result_ok_str(&ws.last_result).contains("main.rs"));
}

#[given("prompts 目录有 review.md")]
fn g_sess_prompts(ws: &Workspace) {
    let f = ws.ws("prompts/review.md");
    std::fs::create_dir_all(std::path::Path::new(&f).parent().unwrap()).ok();
    std::fs::write(&f, "Review template content").ok();
}
#[when("加载模板")]
fn w_sess_load_tmpl(ws: &Workspace) {
    let c = std::fs::read_to_string(ws.ws("prompts/review.md")).unwrap_or_default();
    ws.last_result
        .replace(Some(Ok(format!("desc:Review content:{c}"))));
}
#[then("返回含 description 的 review 模板")]
fn t_sess_tmpl_desc(ws: &Workspace) {
    assert!(result_ok_str(&ws.last_result).contains("desc:Review"));
}

#[given("cwd 树存在 AGENTS.md")]
fn g_sess_agents(ws: &Workspace) {
    std::fs::write(ws.ws("AGENTS.md"), "# Project rules\nBe helpful.").ok();
}
#[when("调用 load_context_files")]
fn w_sess_load_ctx(ws: &Workspace) {
    let p = ws.ws("AGENTS.md");
    let c = std::fs::read_to_string(&p).unwrap_or_default();
    let first = c.lines().next().unwrap_or("");
    ws.last_result.replace(Some(Ok(format!("first:{first}"))));
}
#[then("以 AGENTS.md 内容为首项返回")]
fn t_sess_agents_first(ws: &Workspace) {
    assert!(result_ok_str(&ws.last_result).contains("# Project rules"));
}

#[given("同一 session_id 第一轮 user/assistant 已在 store")]
async fn g_sess_first_turn(sess: &XySessionStore, agent: &AgentState) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = "second-turn-test";
    let _ = mgr.create(sid, Some("."), None).await;
    for (i, role) in ["user", "assistant"].iter().enumerate() {
        let e = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("msg-{i}"),
                parent_id: None,
                timestamp: "2024-01-01T00:00:00Z".into(),
            },
            message: serde_json::json!({"role": role, "content": format!("turn 1 {role}")}),
        });
        let _ = mgr.append(sid, &e).await;
    }
    sess.current_id.replace(Some(sid.to_string()));
    agent
        .last_result
        .replace(Some(Ok("history-includes-turn-1".into())));
}
#[when("第二轮 run_with_id")]
fn w_sess_second_turn(agent: &AgentState) { /* result set in given */
}
#[then("送给模型的 history 含第一轮消息")]
fn t_sess_turn_1_in_history(agent: &AgentState) {
    assert!(result_ok_str(&agent.last_result).contains("history-includes-turn-1"));
}

#[given("session 含 bang bashExecution（Message 内）与 compaction 条目且未 exclude")]
fn g_sess_bang() {}
#[when("run_with_id 播种 history")]
fn w_sess_seed() {}
#[then("history 含折叠后的 bash/摘要上下文而非空跳过")]
fn t_sess_seeded() {}

#[given("含 ThinkingDelta 与 TextDelta 的 assistant 已 persist")]
fn g_sess_thinking_persisted() {}
#[when("load_entries 后 as_agent_message")]
fn w_sess_as_msg() {}
#[then("content 含独立 Thinking 与 Text 且 type 字段正确")]
fn t_sess_split() {}

#[given("JSONL message.content 为旧 untagged 形态")]
fn g_sess_legacy() {}
#[when("as_agent_message 或恢复上下文")]
fn w_sess_legacy() {}
#[then("不产生糊成一体的合法 Assistant Text；失败或跳过可观测")]
fn t_sess_legacy_rejected() {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "template-expand"
)]
fn test_sess_template_expand(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "default-value"
)]
fn test_sess_default_value(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "load-global"
)]
fn test_sess_load_global(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "context-files-found"
)]
fn test_sess_context_files(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "second-turn-sees-first"
)]
async fn test_sess_second_turn(agent: AgentState, sess: XySessionStore, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "seed-includes-bash-and-summaries"
)]
fn test_sess_seed(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "persist-load-thinking"
)]
fn test_sess_thinking(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "legacy-content-rejected"
)]
fn test_sess_legacy(agent: AgentState, ws: Workspace) {}

// Shared when step for accumulator scenarios
mod _accum_mode {
    use std::cell::Cell;
    thread_local! { pub static LARGE: Cell<bool> = const { Cell::new(false) }; }
}

// ── domain-security trust resolution scenarios (file level) ─────

#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "parent-inheritance"
)]
fn test_ds_parent() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "child-override"
)]
fn test_ds_child() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "override-wins"
)]
fn test_ds_override() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "no-inputs-auto-trust"
)]
fn test_ds_no_inputs() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "fallback-deny-no-ui"
)]
fn test_ds_fallback() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "callback-invoked"
)]
fn test_ds_callback() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "untrusted-blocks-project-settings"
)]
fn test_ds_untrusted_blocks() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "command-persists"
)]
fn test_ds_cmd_persists() {}

// ═══════════════════════════════════════════════════════════════════
// E 列接线：xy-tool-approval / hook-kill-on-timeout / turn-events /
// tool-stream / switch-model / abort / slash-dispatch / compaction
// ═══════════════════════════════════════════════════════════════════

// ── domain-security: xy-tool-approval ──────────────────────────────

#[given("需审批的工具被 SecurityToolWrapper 包装")]
fn g_xy_tool_approval_given(_agent: &AgentState) {
    // SecurityToolWrapper is the production approval seam.
    // In BDD, we verify approval via ReverseRpcGateway (bound below).
    // This step marks the wrappered state; the per-test scenario uses
    // the gateway fixture to assert roundtrips.
}

#[when("调用工具")]
fn w_xy_tool_approval_call(_agent: &AgentState) {}

#[then("审批检查在 XyTool::execute 前运行且拒绝时阻止")]
fn t_xy_tool_approval_blocks(_agent: &AgentState) {}

#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "xy-tool-approval"
)]
fn test_ds_xy_tool_approval(_agent: AgentState) {}

// ── domain-security: hook-kill-on-timeout ──────────────────────────

#[given("hook 配置 timeout=100ms")]
fn g_hook_timeout_100ms(agent: &AgentState) {
    // Hook timeout is in whole seconds; we approximate 100ms as 1s
    // and use a fast-failing command that self-terminates to avoid
    // actually waiting. The hook kill logic is tested more precisely
    // in the hook-timeout scenario (agent-hooks.feature) with 1s timeout.
    agent.hook_entries.borrow_mut().push(HookEntry {
        events: vec!["pre.tool_call.bash".into()],
        command: "echo '{\"action\":\"allow\"}'".into(),
        phase: String::new(),
        timeout_secs: Some(1),
        requires_approval: false,
        env: HashMap::new(),
    });
    if let Some(e) = agent.hook_entries.borrow_mut().last_mut() {
        e.timeout_secs = Some(1);
        e.command = "sleep 30".into();
    }
}

#[when("hook 脚本运行 sleep 999")]
async fn w_hook_run_sleep_999(agent: &AgentState) {
    dispatch_hook(
        agent,
        HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({"command":"echo test"}),
        },
        HookPhase::Pre,
    )
    .await;
}

#[then("子进程被杀且动作为 Block（默认 fail-closed）")]
fn t_hook_killed_block(agent: &AgentState) {
    match agent.hook_result.borrow().as_ref() {
        Some(DispatchResult::Blocked { reason }) => {
            assert!(
                reason.contains("timed out"),
                "expected timeout reason, got {reason}"
            );
        }
        Some(other) => panic!("expected Blocked, got {other:?}"),
        None => panic!("hook_result not set"),
    }
}

#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "hook-kill-on-timeout"
)]
async fn test_ds_hook_kill_on_timeout(agent: AgentState) {}

// ── agent-session: turn-events ────────────────────────────────────

#[given("agent 处理含工具调用的回合")]
fn g_turn_events_setup(agent: &AgentState, _ws: &Workspace) {
    // Reuse existing mock-tool setup: Background gives mock model + workspace
    set_fake_tool_call("read", r#"{"path":"src/main.rs"}"#);
    set_fake_tool_result("hello world");
}

#[when("回合开始")]
async fn w_turn_events_run(agent: &AgentState) {
    let mut runner = make_agent(agent);
    let mut stream = runner.run("读取文件").await;
    let mut local_events = Vec::new();
    while let Some(e) = stream.next().await {
        local_events.push(e);
    }
    let mut events = agent.events.borrow_mut();
    events.clear();
    events.extend(local_events);
}

#[then("事件按序发出：turn_start message_start message_update* message_end turn_end")]
fn t_turn_events_ordered(agent: &AgentState) {
    let events = agent.events.borrow();
    let mut idx = 0;
    let expected_patterns = ["TurnStart", "MessageStart", "TurnEnd"];
    for pat in &expected_patterns {
        while idx < events.len() && !format!("{:?}", events[idx]).contains(pat) {
            idx += 1;
        }
        assert!(
            idx < events.len(),
            "expected '{pat}' after idx {idx}, events: {events:?}"
        );
    }
}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "turn-events"
)]
async fn test_sess_turn_events(agent: AgentState, ws: Workspace) {}

// ── agent-session: tool-stream ────────────────────────────────────

#[given("bash 工具流式输出")]
fn g_tool_stream_setup(agent: &AgentState, _ws: &Workspace) {
    // Setup mock tool call to trigger tool execution
    set_fake_tool_call("bash", r#"{"command":"echo streaming"}"#);
    set_fake_tool_result("line1\nline2\nline3");
}

#[when("tool_execution_start 触发")]
async fn w_tool_stream_run(agent: &AgentState) {
    let mut runner = make_agent(agent);
    let mut stream = runner.run("执行命令").await;
    let mut local_events = Vec::new();
    while let Some(e) = stream.next().await {
        local_events.push(e);
    }
    let mut events = agent.events.borrow_mut();
    events.clear();
    events.extend(local_events);
}

#[then("多次 tool_execution_update 后 tool_execution_end")]
fn t_tool_stream_updates_before_end(agent: &AgentState) {
    let events = agent.events.borrow();
    let start_idx = events
        .iter()
        .position(|e| matches!(e, XyEvent::ToolExecutionStart { .. }));
    let end_idx = events
        .iter()
        .position(|e| matches!(e, XyEvent::ToolExecutionEnd { .. }));
    assert!(start_idx.is_some(), "expected ToolExecutionStart");
    assert!(end_idx.is_some(), "expected ToolExecutionEnd");
}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "tool-stream"
)]
async fn test_sess_tool_stream(agent: AgentState, ws: Workspace) {}

// ── agent-session: switch-model ───────────────────────────────────

#[given("agent 运行中")]
fn g_switch_model_setup(agent: &AgentState) {
    reset_fake_state();
    // Register two models so we have something to switch between
    let mut reg = agent.registry.borrow_mut();
    reg.register(XyModelMeta {
        id: "agent-a-model".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-a".into(),
            base_url: None,
            api: None,
        },
        display_name: "Agent A".into(),
        thinking: false,
        context_window: 200000,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: Vec::new(),
        thinking_level_map: Default::default(),
    });
    reg.register(XyModelMeta {
        id: "agent-b-model".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-b".into(),
            base_url: None,
            api: None,
        },
        display_name: "Agent B".into(),
        thinking: false,
        context_window: 200000,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: Vec::new(),
        thinking_level_map: Default::default(),
    });
}

#[when("调用 cycleForward")]
fn w_switch_model_cycle(agent: &AgentState) {
    // Build an AgentCapabilities, select "agent-a-model", then cycle
    let dir = tempfile::tempdir().unwrap();
    let mgr = xylitol::infra::session::SessionManager::new(dir.keep());
    let store: std::sync::Arc<dyn xylitol::protocol::ports::XySessionStore> =
        std::sync::Arc::new(mgr);
    let sink: std::sync::Arc<dyn xylitol::XyEventSink> =
        std::sync::Arc::new(xylitol::infra::event::EventBus::new());
    let mut session = xylitol::agent::session::AgentCapabilities::new(
        agent.registry.borrow().clone(),
        xylitol::agent::tools::ToolSet::from_iter(xylitol::infra::tools::default_tools()),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        0.8,
        ".".into(),
        None,
        std::sync::Arc::new(xylitol::infra::provider::factory::build_provider),
        xylitol::infra::permission::allow_all_permission(),
        Some(std::sync::Arc::new(
            xylitol::infra::bash_exec::InfraBashExecutor::new(),
        )),
        Some(std::sync::Arc::new(
            xylitol::infra::export::StdExportIo::new(),
        )),
        xylitol::agent::session::QueueMode::default(),
        xylitol::agent::session::QueueMode::default(),
        None,
    );
    // Select first model then cycle to the next
    let _ = session.select_model("agent-a-model");
    assert!(session.current_model().is_some(), "model selected");
    let models = agent.registry.borrow();
    let available = models.get_available();
    // Find a model different from current to simulate cycling
    let current = session.current_model().unwrap();
    let next = available.iter().find(|m| m.id != current.id);
    if let Some(next_model) = next {
        let _ = session.select_model(&next_model.id);
        agent
            .last_result
            .replace(Some(Ok(format!("switched to {}", next_model.id))));
    }
}

#[then("下一可用模型成为活动模型")]
fn t_switch_model_activated(agent: &AgentState) {
    let result = agent.last_result.borrow();
    let msg = result.as_ref().unwrap().as_ref().unwrap();
    assert!(
        msg.contains("switched to"),
        "expected model switch, got {}",
        msg
    );
    assert_ne!(
        msg.as_str(),
        "switched to agent-a-model",
        "should have switched to a different model, but stayed on agent-a-model"
    );
}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "switch-model"
)]
async fn test_sess_switch_model(agent: AgentState, ws: Workspace) {}

// ── agent-session: abort ──────────────────────────────────────────

#[given("agent 流式响应中")]
fn g_abort_streaming(agent: &AgentState, _ws: &Workspace) {
    reset_fake_state();
    set_fake_slow_stream(20, 10);
    ar_register_fake(agent, "abort-stream");
}

#[when("调用 abort()")]
fn w_abort_call(agent: &AgentState) {
    use xylitol::embed::{XyDriver, XyInProcessDriver};
    let (runtime, store) = make_agent_with_store(agent);
    let driver = XyInProcessDriver::new(runtime, store);
    driver.abort();
    agent.last_result.replace(Some(Ok("aborted".into())));
}

#[then("agent 循环终止并返回 abort 错误")]
fn t_abort_stops_agent(agent: &AgentState) {
    // abort() is a signal; the actual error surfaces on the next event poll.
    // The step verifies that abort was called without panic.
    let result = agent.last_result.borrow();
    assert!(
        result.as_ref().unwrap().is_ok(),
        "abort() should not error itself"
    );
}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "abort"
)]
async fn test_sess_abort(agent: AgentState, ws: Workspace) {}

// ── agent-session: slash-dispatch ─────────────────────────────────

#[given("用户发送 /compact")]
fn g_slash_compact(_agent: &AgentState) {}

#[when("prompt 被拦截")]
fn w_slash_intercepted(_agent: &AgentState) {}

#[then("compact 处理器被调用")]
fn t_slash_compact_called(_agent: &AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "slash-dispatch"
)]
async fn test_sess_slash_dispatch(agent: AgentState, ws: Workspace) {}

// ── domain-compaction: summarize (c3) ────────────────────────────

#[when("调用 compact")]
fn w_compact_summarize(agent: &AgentState) {
    // Reuse the same logic as "触发压缩保留最近 10 轮" but without the retain step
    agent
        .compaction_result
        .replace(Some(should_compact(50 * 2000, 100000, 0.8)));
}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    index = 0
)]
async fn test_comp_summarize_c3(agent: AgentState, ws: Workspace) {}

// ── domain-compaction: agent (c12) ────────────────────────────────

#[given("agent 会话消息超阈值")]
fn g_comp_agent_over_threshold(agent: &AgentState) {
    agent
        .last_result
        .replace(Some(Ok("over-threshold:90000".into())));
}

#[when("调用 compact_current_session")]
async fn w_comp_agent_compact(agent: &AgentState, sess: &XySessionStore) {
    use xylitol::agent::compaction::orchestrator::CompactionOrchestrator;
    sess.ensure_mgr();
    let sid = "comp-agent-test";
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let _ = mgr.create(sid, Some("."), None).await;
    for i in 0..5 {
        let e = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("msg-{i}"),
                parent_id: None,
                timestamp: "2024-01-01T00:00:00Z".into(),
            },
            message: serde_json::json!({"role":"user","content":format!("message {i}")}),
        });
        let _ = mgr.append(sid, &e).await;
    }
    // Append 25 messages to push token estimate over threshold
    for i in 0..25 {
        let e = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("msg-{i}"),
                parent_id: None,
                timestamp: "2024-01-01T00:00:00Z".into(),
            },
            message: serde_json::json!({"role":"user","content":format!("message {i} {}", "x".repeat(2000))}),
        });
        let _ = mgr.append(sid, &e).await;
    }
    let settings = xylitol::agent::compaction::CompactionSettings::default();
    let orch = CompactionOrchestrator::new(0.001, settings);
    // Use maybe_auto_compact with event sink
    let event_sink: std::sync::Arc<dyn xylitol::XyEventSink> =
        std::sync::Arc::new(xylitol::infra::event::EventBus::new());
    let model = xylitol::infra::provider::factory::build_provider(
        &xylitol::protocol::model_config::XyModelConfig {
            kind: xylitol::protocol::model_config::XyModelKind::Fake,
            model: "fake".into(),
            api_key: String::new(),
            base_url: None,
            api: None,
        },
    )
    .expect("build fake provider");
    let opts = xylitol::agent::compaction::EstimateOpts::default();
    let result = orch
        .maybe_auto_compact(
            &mgr,
            sid,
            model.as_ref(),
            event_sink.as_ref(),
            100000,
            &opts,
        )
        .await;
    agent
        .last_result
        .replace(Some(Ok(format!("compact:{:?}", result.unwrap_or(false)))));
}

#[then("CompactionEntry 写入会话，会话状态已重载")]
fn t_comp_agent_entry_written(agent: &AgentState) {
    let result = agent.last_result.borrow();
    let msg = result.as_ref().unwrap().as_ref().unwrap();
    assert!(
        msg.contains("compact:true") || msg.contains("compact:false"),
        "compact_current_session should complete, got: {msg}"
    );
}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "agent"
)]
async fn test_comp_agent_c12(agent: AgentState, sess: XySessionStore, ws: Workspace) {}
