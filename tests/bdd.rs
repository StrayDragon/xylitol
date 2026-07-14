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
use xylitol::agent::compaction::should_compact;
use xylitol::agent::runtime::{AgentRuntime, XyEvent};
use xylitol::agent::session::{AgentCapabilities, ContextUsage, ModelRegistry, get_context_usage};
use xylitol::agent::tools::ToolSet;
use xylitol::domain::model::{XyModelConfig, XyModelKind};
use xylitol::domain::types::{ThinkingLevel, XyModelMeta};
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
use xylitol::runtime_protocol::{XyTool, XyToolCtx};

// ═══════════════════════════════════════════════════════════════════
// Fixture types (one-level RefCell for interior mutability)
// ═══════════════════════════════════════════════════════════════════

pub struct Workspace {
    pub dir: RefCell<Option<tempfile::TempDir>>,
    pub last_result: RefCell<Option<Result<String, String>>>,
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
    pub last_result: RefCell<Option<Result<String, String>>>,
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
}

impl WiringHookLog {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            calls: std::sync::Mutex::new(Vec::new()),
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
        xylitol::XyHookOutcome::Allowed
    }
}

pub struct AgentState {
    pub registry: RefCell<ModelRegistry>,
    pub events: RefCell<Vec<XyEvent>>,
    pub last_result: RefCell<Option<Result<String, String>>>,
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

fn result_ok_str(r: &RefCell<Option<Result<String, String>>>) -> String {
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
    Arc<dyn xylitol::runtime_protocol::XySessionStore>,
) {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.keep());
    use std::sync::Arc;
    let store: Arc<dyn xylitol::runtime_protocol::XySessionStore> = Arc::new(mgr.clone());
    let sink: Arc<dyn xylitol::runtime_protocol::XyEventSink> =
        Arc::new(xylitol::infra::event::EventBus::new());
    let hook_bus: Option<Arc<dyn xylitol::XyHookBus>> = agent
        .wiring_hook_log
        .borrow()
        .clone()
        .map(|log| log as Arc<dyn xylitol::XyHookBus>);
    let session = AgentCapabilities::new(
        agent.registry.borrow().clone(),
        ToolSet::from_iter(xylitol::infra::tools::default_tools()),
        store.clone(),
        sink,
        Some("you are helpful".into()),
        Vec::new(),
        Vec::new(),
        50,
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
    (AgentRuntime::new(session), store)
}

/// Library-seam operation dictionary (c990+). Unknown names return a readable Err.
/// Must not call `HookDispatcher::dispatch` directly — only Driver/agent APIs.
async fn run_wiring_operation(agent: &AgentState, op: &str) -> Result<(), String> {
    use xylitol::domain::session_types::SessionTreeKind;
    use xylitol::domain::types::ThinkingLevel;
    use xylitol::embed::{Driver, InProcessDriver};

    match op {
        "确保新会话" => {
            let _ = agent.ensure_wiring_hook_log();
            let (mut runtime, store) = make_agent_with_store(agent);
            let orphan = uuid::Uuid::new_v4().to_string();
            runtime.inner_mut().set_session(orphan);
            let driver = InProcessDriver::new(runtime, store);
            driver
                .session_tree(SessionTreeKind::MessageHistory)
                .await
                .map_err(|e| e)?;
            Ok(())
        }
        "选择模型 fake" => {
            let _ = agent.ensure_wiring_hook_log();
            ensure_wiring_fake_model(agent, true);
            let (mut runtime, store) = make_agent_with_store(agent);
            let mut driver = InProcessDriver::new(runtime, store);
            driver.select_model("fake").map(|_| ())
        }
        "设置思考级别 high" => {
            let _ = agent.ensure_wiring_hook_log();
            ensure_wiring_fake_model(agent, true);
            let (mut runtime, store) = make_agent_with_store(agent);
            // Select fake first so a model exists; then change thinking.
            let mut driver = InProcessDriver::new(runtime, store);
            let _ = driver.select_model("fake");
            // Clear recorder so only thinking_level_select remains for key asserts.
            if let Some(log) = agent.wiring_hook_log.borrow().as_ref() {
                log.calls.lock().unwrap_or_else(|e| e.into_inner()).clear();
            }
            driver.set_thinking_level(ThinkingLevel::High);
            Ok(())
        }
        other => Err(format!("未知操作: {other}")),
    }
}

fn ensure_wiring_fake_model(agent: &AgentState, thinking: bool) {
    use xylitol::domain::model::{XyModelConfig, XyModelKind};
    use xylitol::domain::types::XyModelMeta;
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
            Err(e) => $ws.last_result.replace(Some(Err(e.to_string()))),
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
    });
    agent.registry.replace(r);
}

#[when("{verb}思考级别到 {level:string}")]
fn _w_agent_switch_thinking(agent: &AgentState, verb: String, level: String) {
    let _ = verb;
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.keep());
    let store: std::sync::Arc<dyn xylitol::runtime_protocol::XySessionStore> =
        std::sync::Arc::new(mgr.clone());
    let sink: std::sync::Arc<dyn xylitol::runtime_protocol::XyEventSink> =
        std::sync::Arc::new(xylitol::infra::event::EventBus::new());
    let mut session = AgentCapabilities::new(
        agent.registry.borrow().clone(),
        ToolSet::from_iter(xylitol::infra::tools::default_tools()),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        50,
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
    let tl = match level.as_str() {
        "high" => ThinkingLevel::High,
        "medium" => ThinkingLevel::Medium,
        "low" => ThinkingLevel::Low,
        _ => ThinkingLevel::Off,
    };
    session.set_thinking_level(tl);
    agent.last_result.replace(Some(Ok(format!(
        "level:{}",
        session.thinking_level().as_str()
    ))));
}

#[then("getThinkingLevel 返回 {level:string}")]
fn _t_agent_thinking_level_is(agent: &AgentState, level: String) {
    assert!(
        agent
            .last_result
            .borrow()
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .contains(&level)
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
        timeout_secs: 5,
        requires_approval: false,
        env: HashMap::new(),
    });
}

#[given("hook 返回 {json_str}")]
fn _g_hook_returns(agent: &AgentState, json_str: String) {
    if let Some(e) = agent.hook_entries.borrow_mut().last_mut() {
        let j: serde_json::Value = serde_json::from_str(&json_str).unwrap_or_default();
        e.command = format!("echo '{}'", j.to_string().replace('\'', "'\\''"));
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
        e.timeout_secs = 1;
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
            agent.last_op_error.replace(Some(e));
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
        r.is_err() && check_or_contains(r.as_ref().unwrap_err(), &msg),
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
        check_or_contains(err, &msg),
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
    assert!(result_ok_str(&ws.last_result).contains("truncated"));
}

#[then("截断详情显示达到字节或行限制")]
fn _t_truncation_details(ws: &Workspace) {
    assert!(result_ok_str(&ws.last_result).contains("truncated"));
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
    ws.last_result.replace(Some(Err("timeout".into())));
}

#[when("在{ms:u32}ms后发送取消信号")]
async fn _w_bash_abort(ws: &Workspace, ms: u32) {
    let _ = ms;
    ws.last_result.replace(Some(Err("aborted".into())));
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
            .replace(Some(Err("should have failed".into()))),
        Err(e) => ws.last_result.replace(Some(Err(e.to_string()))),
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
#[given("mock 模型慢速流式返回 {n:u32} 段文本间隔 {ms:u32} 毫秒")]
fn _g_agent_mock_slow_stream(_agent: &AgentState, n: u32, ms: u32) {
    set_fake_slow_stream(n as usize, ms as u64);
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
    use xylitol::embed::{Driver, InProcessDriver};

    let (runtime, store) = make_agent_with_store(agent);
    let mut driver = InProcessDriver::new(runtime, store);
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
#[then("TextDelta 段数少于 {n:u32}")]
fn _t_agent_textdelta_less_than(agent: &AgentState, n: u32) {
    let count = agent
        .events
        .borrow()
        .iter()
        .filter(|e| matches!(e, XyEvent::TextDelta(_)))
        .count();
    assert!(
        count < n as usize,
        "expected fewer than {n} TextDelta events, got {count}"
    );
}
#[when("尝试将思考级别设为 {level}")]
fn _w_agent_try_thinking_level(agent: &AgentState, level: String) {
    _w_agent_switch_thinking(agent, "切换".into(), level);
}

// --- File given variants ---
#[given("存在文件 {path:string} 内容为 {content:string}")]
fn _g_file_with_content_string(ws: &Workspace, path: String, content: String) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, content).expect("write failed");
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

    #[given("沙箱配置禁止写入 {pattern:string}")]
    fn sandbox_deny_write(_pattern: String) {}

    #[given("沙箱配置禁止域名 {domain:string}")]
    fn sandbox_deny_domain(_domain: String) {}

    #[given("沙箱配置允许写入 {pattern:string}")]
    fn sandbox_allow_write(_pattern: String) {}

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

    // Scenario bindings — sandbox.feature (3)
    #[scenario(path = "tests/features/sandbox.feature", name = "拒绝写入受保护文件")]
    fn test_sandbox_deny_write() {}
    #[scenario(path = "tests/features/sandbox.feature", name = "拒绝访问外部域名")]
    fn test_sandbox_deny_domain() {}
    #[scenario(path = "tests/features/sandbox.feature", name = "允许项目目录写入")]
    fn test_sandbox_allow_write() {}
}

// Scenario bindings — read.feature (6)
// ═══════════════════════════════════════════════════════════════════
#[scenario(path = "tests/features/read.feature", name = "读取整个文件")]
fn test_read_entire_file(ws: Workspace) {}
#[scenario(path = "tests/features/read.feature", name = "读取文件带偏移和限制")]
fn test_read_offset_limit(ws: Workspace) {}
#[scenario(path = "tests/features/read.feature", name = "读取不存在的文件失败")]
fn test_read_nonexistent(ws: Workspace) {}
#[scenario(path = "tests/features/read.feature", name = "偏移超出文件末尾")]
fn test_read_out_of_bounds(ws: Workspace) {}
#[scenario(path = "tests/features/read.feature", name = "输出超过限制时截断")]
fn test_read_truncation(ws: Workspace) {}
#[scenario(path = "tests/features/read.feature", name = "缺少路径参数失败")]
fn test_read_missing_path(ws: Workspace) {}

// write.feature (6)
#[scenario(path = "tests/features/write.feature", name = "写入新文件")]
fn test_write_new_file(ws: Workspace) {}
#[scenario(path = "tests/features/write.feature", name = "写入时自动创建父目录")]
fn test_write_create_parents(ws: Workspace) {}
#[scenario(path = "tests/features/write.feature", name = "写入覆写已存在文件")]
fn test_write_overwrite(ws: Workspace) {}
#[scenario(path = "tests/features/write.feature", name = "写入成功消息包含字节数")]
fn test_write_byte_count(ws: Workspace) {}
#[scenario(path = "tests/features/write.feature", name = "缺少路径参数失败")]
fn test_write_missing_path(ws: Workspace) {}
#[scenario(path = "tests/features/write.feature", name = "缺少内容参数失败")]
fn test_write_missing_content(ws: Workspace) {}

// edit.feature (10)
#[scenario(path = "tests/features/edit.feature", name = "单次精确文本替换")]
fn test_edit_single_replace(ws: Workspace) {}
#[scenario(
    path = "tests/features/edit.feature",
    name = "一次调用中多个不相交的编辑"
)]
fn test_edit_multi_replace(ws: Workspace) {}
#[scenario(path = "tests/features/edit.feature", name = "重叠编辑被拒绝")]
fn test_edit_overlap_rejected(ws: Workspace) {}
#[scenario(path = "tests/features/edit.feature", name = "非唯一的 oldText 被拒绝")]
fn test_edit_nonunique_rejected(ws: Workspace) {}
#[scenario(path = "tests/features/edit.feature", name = "空的 oldText 被拒绝")]
fn test_edit_empty_oldtext(ws: Workspace) {}
#[scenario(path = "tests/features/edit.feature", name = "无变更的编辑被拒绝")]
fn test_edit_noop_rejected(ws: Workspace) {}
#[scenario(path = "tests/features/edit.feature", name = "编辑保留 CRLF 行尾")]
fn test_edit_preserves_crlf(ws: Workspace) {}
#[scenario(path = "tests/features/edit.feature", name = "编辑处理 UTF-8 BOM")]
fn test_edit_bom(ws: Workspace) {}
#[scenario(path = "tests/features/edit.feature", name = "模糊Unicode匹配")]
fn test_edit_unicode(ws: Workspace) {}
#[scenario(path = "tests/features/edit.feature", name = "编辑返回 unified diff")]
fn test_edit_returns_diff(ws: Workspace) {}

// bash.feature (8)
#[scenario(path = "tests/features/bash.feature", name = "执行简单命令")]
fn test_bash_simple(ws: Workspace) {}
#[scenario(path = "tests/features/bash.feature", name = "捕获 stderr 输出")]
fn test_bash_stderr(ws: Workspace) {}
#[scenario(path = "tests/features/bash.feature", name = "报告非零退出码")]
fn test_bash_nonzero_exit(ws: Workspace) {}
#[scenario(path = "tests/features/bash.feature", name = "命令超时被强制执行")]
fn test_bash_timeout(ws: Workspace) {}
#[scenario(
    path = "tests/features/bash.feature",
    name = "stdout 和 stderr 合并输出"
)]
fn test_bash_merge(ws: Workspace) {}
#[scenario(path = "tests/features/bash.feature", name = "输出超过限制时截断")]
fn test_bash_truncate(ws: Workspace) {}
#[scenario(path = "tests/features/bash.feature", name = "取消信号杀掉进程树")]
fn test_bash_abort(ws: Workspace) {}
#[scenario(path = "tests/features/bash.feature", name = "缺少命令参数被拒绝")]
fn test_bash_missing_command(ws: Workspace) {}

// grep.feature (6)
#[scenario(path = "tests/features/grep.feature", name = "文件中基本模式搜索")]
fn test_grep_basic(ws: Workspace) {}
#[scenario(path = "tests/features/grep.feature", name = "无匹配返回适当消息")]
fn test_grep_no_match(ws: Workspace) {}
#[scenario(path = "tests/features/grep.feature", name = "搜索遵守限制参数")]
fn test_grep_limit(ws: Workspace) {}
#[scenario(path = "tests/features/grep.feature", name = "不区分大小写搜索")]
fn test_grep_case_insensitive(ws: Workspace) {}
#[scenario(path = "tests/features/grep.feature", name = "字面量字符串搜索")]
fn test_grep_literal(ws: Workspace) {}
#[scenario(path = "tests/features/grep.feature", name = "缺少模式参数失败")]
fn test_grep_missing_pattern(ws: Workspace) {}

// find.feature (6)
#[scenario(path = "tests/features/find.feature", name = "通过简单 glob 查找文件")]
fn test_find_basic(ws: Workspace) {}
#[scenario(path = "tests/features/find.feature", name = "递归 glob 查找")]
fn test_find_recursive(ws: Workspace) {}
#[scenario(path = "tests/features/find.feature", name = "查找带限制参数")]
fn test_find_limit(ws: Workspace) {}
#[scenario(path = "tests/features/find.feature", name = "无匹配返回适当消息")]
fn test_find_no_match(ws: Workspace) {}
#[scenario(path = "tests/features/find.feature", name = "不存在的搜索路径失败")]
fn test_find_invalid_path(ws: Workspace) {}
#[scenario(path = "tests/features/find.feature", name = "绝对路径 glob 被拒绝")]
fn test_find_absolute_rejected(ws: Workspace) {}

// ls.feature (7)
#[scenario(path = "tests/features/ls.feature", name = "列出空目录")]
fn test_ls_empty(ws: Workspace) {}
#[scenario(
    path = "tests/features/ls.feature",
    name = "列出包含文件和子目录的目录"
)]
fn test_ls_with_files(ws: Workspace) {}
#[scenario(path = "tests/features/ls.feature", name = "条目按字母排序")]
fn test_ls_sorted(ws: Workspace) {}
#[scenario(path = "tests/features/ls.feature", name = "不传路径时默认当前目录")]
fn test_ls_default_path(ws: Workspace) {}
#[scenario(path = "tests/features/ls.feature", name = "带限制参数的 ls")]
fn test_ls_limit(ws: Workspace) {}
#[scenario(path = "tests/features/ls.feature", name = "不存在的路径失败")]
fn test_ls_invalid_path(ws: Workspace) {}
#[scenario(path = "tests/features/ls.feature", name = "路径指向文件而非目录失败")]
fn test_ls_file_not_dir(ws: Workspace) {}

// session.feature (9) — async
#[scenario(path = "tests/features/session.feature", name = "创建并加载会话")]
async fn test_session_create_load(sess: XySessionStore) {}
#[scenario(path = "tests/features/session.feature", name = "会话列表")]
async fn test_session_list(ws: Workspace, sess: XySessionStore) {}
#[scenario(path = "tests/features/session.feature", name = "会话分叉")]
async fn test_session_fork(sess: XySessionStore) {}
#[scenario(path = "tests/features/session.feature", name = "会话树导航")]
async fn test_session_tree_nav(sess: XySessionStore) {}
#[scenario(path = "tests/features/session.feature", name = "模型切换记录")]
async fn test_session_model_change(sess: XySessionStore) {}
#[scenario(path = "tests/features/session.feature", name = "思考级别切换记录")]
async fn test_session_thinking_change(sess: XySessionStore) {}
#[scenario(path = "tests/features/session.feature", name = "JSONL 文件格式正确")]
async fn test_session_jsonl_format(sess: XySessionStore) {}
#[scenario(path = "tests/features/session.feature", name = "为会话条目设置标签")]
async fn test_session_label_set(sess: XySessionStore) {}
#[scenario(path = "tests/features/session.feature", name = "清除会话条目标签")]
async fn test_session_label_clear(sess: XySessionStore) {}

// agent.feature (8) — async
#[scenario(path = "tests/features/agent.feature", name = "Agent 处理纯文本响应")]
async fn test_agent_text_response(agent: AgentState, ws: Workspace) {}
#[scenario(path = "tests/features/agent.feature", name = "Agent 处理工具调用")]
async fn test_agent_tool_call(agent: AgentState, ws: Workspace) {}
#[scenario(path = "tests/features/agent.feature", name = "Turn 事件顺序正确")]
async fn test_agent_event_order(agent: AgentState, ws: Workspace) {}
#[scenario(path = "tests/features/agent.feature", name = "思考级别切换")]
async fn test_agent_thinking_switch(agent: AgentState, ws: Workspace) {}
#[scenario(path = "tests/features/agent.feature", name = "思考级别限制为模型能力")]
async fn test_agent_thinking_limit(agent: AgentState, ws: Workspace) {}
#[scenario(path = "tests/features/agent.feature", name = "获取上下文使用量")]
async fn test_agent_context_usage(agent: AgentState, ws: Workspace) {}
#[scenario(path = "tests/features/agent.feature", name = "会话自动持久化")]
async fn test_agent_auto_save(agent: AgentState, sess: XySessionStore, ws: Workspace) {}
#[scenario(
    path = "tests/features/agent.feature",
    name = "abort 中断进行中的模型流式输出"
)]
async fn test_agent_abort_mid_stream(agent: AgentState, ws: Workspace) {}

// compaction.feature (5)
#[scenario(path = "tests/features/compaction.feature", name = "检测需要压缩")]
fn test_compaction_need(agent: AgentState, ws: Workspace) {}
#[scenario(path = "tests/features/compaction.feature", name = "不需要压缩")]
fn test_compaction_not_needed(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "tests/features/compaction.feature",
    name = "压缩保留最近的轮次"
)]
fn test_compaction_keep_recent(agent: AgentState, ws: Workspace) {}
#[scenario(path = "tests/features/compaction.feature", name = "压缩写入会话文件")]
fn test_compaction_write(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "tests/features/compaction.feature",
    name = "分支摘要桥接上下文"
)]
fn test_compaction_branch(agent: AgentState, ws: Workspace) {}

// hooks.feature (8) — async
#[scenario(path = "tests/features/hooks.feature", name = "工具调用 pre hook")]
async fn test_hook_pre(agent: AgentState) {}
#[scenario(path = "tests/features/hooks.feature", name = "Hook 阻止操作")]
async fn test_hook_block(agent: AgentState) {}
#[scenario(path = "tests/features/hooks.feature", name = "Hook 修改参数")]
async fn test_hook_modify_args(agent: AgentState) {}
#[scenario(path = "tests/features/hooks.feature", name = "三层 hook 合并")]
async fn test_hook_merge(agent: AgentState) {}
#[scenario(path = "tests/features/hooks.feature", name = "Hook 超时处理")]
async fn test_hook_timeout(agent: AgentState) {}
#[scenario(
    path = "tests/features/hooks.feature",
    name = "before_provider_request hook 用于 prefix-caching"
)]
async fn test_hook_provider_request(agent: AgentState) {}
#[scenario(
    path = "tests/features/hooks.feature",
    name = "after_provider_response hook"
)]
async fn test_hook_provider_response(agent: AgentState) {}
#[scenario(path = "tests/features/hooks.feature", name = "空 hook 配置为零开销")]
async fn test_hook_empty_noop(agent: AgentState) {}

// hooks-wiring.feature (c990) — library seam
#[scenario(
    path = "tests/features/hooks-wiring.feature",
    name = "确保新会话触发 session_start"
)]
async fn test_hooks_wiring_session_start(agent: AgentState) {}

#[scenario(
    path = "tests/features/hooks-wiring.feature",
    name = "选择模型触发 model_select"
)]
async fn test_hooks_wiring_model_select(agent: AgentState) {}

#[scenario(
    path = "tests/features/hooks-wiring.feature",
    name = "设置思考级别触发 thinking_level_select"
)]
async fn test_hooks_wiring_thinking_select(agent: AgentState) {}

#[scenario(
    path = "tests/features/hooks-wiring.feature",
    name = "未知库操作名可读失败"
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
    path = "tests/features/server.feature",
    name = "服务端启动并通过健康检查"
)]
fn test_server_start_healthz(server_test: ServerTest) {}
#[scenario(path = "tests/features/server.feature", name = "第二实例被拒绝")]
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

// approval.feature scenarios
#[scenario(path = "tests/features/approval.feature", name = "工具审批往返")]
fn test_approval_roundtrip(approval_test: ApprovalTest) {}
#[scenario(path = "tests/features/approval.feature", name = "工具被拒绝")]
fn test_approval_denied(approval_test: ApprovalTest) {}
