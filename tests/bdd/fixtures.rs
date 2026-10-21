use rstest::fixture;
use std::cell::{Cell, RefCell};
use std::sync::Arc;

use xylitol::XyDriverError;
use xylitol_kernel::config::types::HookEntry;
use xylitol_official_plugin_hooks::DispatchResult;
use xylitol_official_plugin_session::{SessionEntry, SessionManager};
use xylitol_runtime::agent::capabilities::{ContextUsage, ModelRegistry};
use xylitol_runtime::agent::runtime::XyEvent;

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
    pub sessions_dir: RefCell<Option<std::path::PathBuf>>,
    pub second_mgr: RefCell<Option<SessionManager>>,
    pub lock_probe: Cell<Option<i32>>,
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
            lock_probe: Cell::new(None),
        }
    }
    pub(crate) fn ensure_mgr(&self) {
        if self.mgr.borrow().is_none() {
            let dir = tempfile::tempdir().unwrap();
            let d = dir.path().join("sessions");
            std::fs::create_dir_all(&d).ok();
            self.sessions_dir.replace(Some(d.clone()));
            std::mem::forget(dir);
            self.mgr.replace(Some(SessionManager::new(d)));
        }
    }
}

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
    pub compaction_keep_tokens: Cell<u64>,
    pub compaction_fixed_overhead: Cell<u64>,
    pub compaction_enabled: Cell<bool>,
    pub hook_result: RefCell<Option<DispatchResult>>,
    pub hook_entries: RefCell<Vec<HookEntry>>,
    pub(crate) last_hook_stdin: RefCell<Option<serde_json::Value>>,
    pub(crate) wiring_hook_log: RefCell<Option<Arc<WiringHookLog>>>,
    pub(crate) last_op_error: RefCell<Option<String>>,
    pub compaction_task_model: RefCell<Option<xylitol_plugin_sdk::model_entry::XyModelEntryConfig>>,
    pub compaction_thinking_level: RefCell<Option<String>>,
    pub compaction_binding:
        RefCell<Option<xylitol_runtime::agent::model::task_model::CompactionSummaryBinding>>,
    pub compaction_notice_count: Cell<u32>,
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
            compaction_keep_tokens: Cell::new(20_000),
            compaction_fixed_overhead: Cell::new(0),
            compaction_enabled: Cell::new(true),
            hook_result: RefCell::new(None),
            hook_entries: RefCell::new(Vec::new()),
            last_hook_stdin: RefCell::new(None),
            wiring_hook_log: RefCell::new(None),
            last_op_error: RefCell::new(None),
            compaction_task_model: RefCell::new(None),
            compaction_thinking_level: RefCell::new(None),
            compaction_binding: RefCell::new(None),
            compaction_notice_count: Cell::new(0),
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
