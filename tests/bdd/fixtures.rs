use rstest::fixture;
use std::cell::{Cell, RefCell};
use std::sync::Arc;

use xylitol::XyDriverError;
use xylitol::agent::capabilities::{ContextUsage, ModelRegistry};
use xylitol::agent::runtime::XyEvent;
use xylitol::infra::config::types::HookEntry;
use xylitol::infra::hooks::DispatchResult;
use xylitol::infra::session::{SessionEntry, SessionManager};

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
    pub(crate) fn ws(&self, path: &str) -> String {
        self.dir
            .borrow()
            .as_ref()
            .expect("workspace not initialized")
            .path()
            .join(path)
            .to_string_lossy()
            .to_string()
    }

    /// Workspace root as a string (session-workspace scenarios).
    pub(crate) fn root(&self) -> String {
        self.dir
            .borrow()
            .as_ref()
            .expect("workspace not initialized")
            .path()
            .to_string_lossy()
            .to_string()
    }
    pub(crate) fn init(&self) {
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
    /// s12/s21/s22 原始文件断言用的会话目录。
    pub sessions_dir: RefCell<Option<std::path::PathBuf>>,
    /// ex3：导入侧的全新存储与返回 id。
    pub second_mgr: RefCell<Option<SessionManager>>,
}
impl XySessionStore {
    fn new() -> Self {
        Self {
            mgr: RefCell::new(None),
            entries: RefCell::new(Vec::new()),
            current_id: RefCell::new(None),
            last_result: RefCell::new(None),
            sessions_dir: RefCell::new(None),
            second_mgr: RefCell::new(None),
        }
    }
    /// Auto-initialize session manager if not yet set.
    pub(crate) fn ensure_mgr(&self) {
        if self.mgr.borrow().is_none() {
            let dir = tempfile::tempdir().unwrap();
            let d = dir.path().join("sessions");
            std::fs::create_dir_all(&d).ok();
            self.sessions_dir.replace(Some(d.clone()));
            // Leak the TempDir to keep it alive for the test duration.
            // This is only for tests.
            std::mem::forget(dir);
            self.mgr.replace(Some(SessionManager::new(d)));
        }
    }
}

/// Library-seam hook recorder implementing crate-root [`xylitol::XyHookBus`] (c990).
pub(crate) struct WiringHookLog {
    pub(crate) calls: std::sync::Mutex<Vec<(String, String, serde_json::Value)>>,
    pub(crate) force: std::sync::Mutex<Option<xylitol::XyHookOutcome>>,
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
    pub context_window: Cell<u64>,
    pub compaction_reserve_tokens: Cell<u64>,
    pub compaction_enabled: Cell<bool>,
    pub hook_result: RefCell<Option<DispatchResult>>,
    pub hook_entries: RefCell<Vec<HookEntry>>,
    /// Last JSON context that would be piped to a hook script stdin.
    pub(crate) last_hook_stdin: RefCell<Option<serde_json::Value>>,
    /// When set, injected as `XyHookBus` for library-seam wiring BDD (c990).
    pub(crate) wiring_hook_log: RefCell<Option<Arc<WiringHookLog>>>,
    pub(crate) last_op_error: RefCell<Option<String>>,
}
impl AgentState {
    fn new() -> Self {
        Self {
            registry: RefCell::new(ModelRegistry::new()),
            events: RefCell::new(Vec::new()),
            last_result: RefCell::new(None),
            context_usage: RefCell::new(None),
            compaction_result: RefCell::new(None),
            context_window: Cell::new(100_000),
            compaction_reserve_tokens: Cell::new(16_384),
            compaction_enabled: Cell::new(true),
            hook_result: RefCell::new(None),
            hook_entries: RefCell::new(Vec::new()),
            last_hook_stdin: RefCell::new(None),
            wiring_hook_log: RefCell::new(None),
            last_op_error: RefCell::new(None),
        }
    }

    pub(crate) fn ensure_wiring_hook_log(&self) -> Arc<WiringHookLog> {
        if self.wiring_hook_log.borrow().is_none() {
            self.wiring_hook_log.replace(Some(WiringHookLog::new()));
        }
        self.wiring_hook_log.borrow().as_ref().unwrap().clone()
    }
}

#[fixture]
pub fn ws() -> Workspace {
    Workspace::new()
}

#[fixture]
pub fn sess() -> XySessionStore {
    XySessionStore::new()
}

#[fixture]
pub fn agent() -> AgentState {
    AgentState::new()
}
