use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use chrono::Local;
use futures::StreamExt;
use serde_json::{Value, json};
use tokio::sync::{Mutex, RwLock, broadcast, mpsc};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use crate::occupancy::engine::{OccupancyEngine, OccupancyFactory};
use crate::occupancy::envelope;
use crate::occupancy::idempotency::{Admission, IdempotencyLedger, conflict_result};
use crate::occupancy::oauth::HostOAuthGateway;
use crate::occupancy::rpc_module::describe_value;
use crate::occupancy::ws::{
    EventJournal, ReverseRpcGateway, ReverseRpcResult, downlink_notification,
};
use crate::wire::Reply;
use crate::{MUX_CHAN_CAP, MuxSink, OccupiedHost};
use xylitol_plugin_sdk::ask::{AskArgs, AskModeArg, AskOptionArg, AskQuestionArg, AskUserGateway};
use xylitol_plugin_sdk::error::{XySessionStoreError, XyToolError};
use xylitol_plugin_sdk::lifecycle::XyEvent;
use xylitol_plugin_sdk::mcp::McpResourceContext;
use xylitol_plugin_sdk::mcp::{
    McpElicitationAction, McpElicitationGateway, McpElicitationRequest, McpElicitationResponse,
};
use xylitol_plugin_sdk::session::{RunOutcome, SessionEntry};
use xylitol_protocol::method::{
    APPROVAL_REQUESTED, BASH_OUTPUT, QUESTION_REQUESTED, SESSION_EVENT, SESSION_MODEL,
    SESSION_RESOURCES, SESSION_RESYNC_REQUIRED, SESSION_SUBSCRIBED, is_unary_method,
};
use xylitol_protocol::registry::parse_command;
use xylitol_protocol::registry::{
    METHOD_ABORT, METHOD_ARM_TOOL_FREEZE, METHOD_EXPORT_JSONL, METHOD_GET_AVAILABLE_MODELS,
    METHOD_HOST_DESCRIBE, METHOD_IMPORT_JSONL, METHOD_LOADED_RESOURCES, METHOD_MCP_CATALOG,
    METHOD_MCP_OAUTH_CANCEL, METHOD_MCP_OAUTH_START, METHOD_MCP_OAUTH_STATUS,
    METHOD_MCP_PROMPT_GET, METHOD_MCP_RESOURCE_READ, METHOD_PERSIST_TRUST, METHOD_PROMPT,
    METHOD_RELOAD, METHOD_SESSION_DETACH, METHOD_SESSION_TREE, METHOD_SUBSCRIBE,
};
use xylitol_protocol::{
    ApprovalRequestedPayload, BashOutputPayload, Notification, QuestionRequestedPayload, RpcResult,
    SessionEventPayload, SessionModelPayload, SessionResourcesPayload,
    SessionResyncRequiredPayload, SessionSubscribedPayload,
};
use xylitol_runtime::XyDriverError;
use xylitol_runtime::agent::RuntimeBaseline;
use xylitol_runtime::agent::automation::{AutomationAction, AutomationEngine};
use xylitol_runtime::driver::{
    LoadedResourcesSnapshot, ModelInfo, ProjectTrustMode, RuntimeReloadReport,
};

pub struct HostState {
    pub ports: RuntimeBaseline,
    pub reload: ReloadBaseline,
    pub factory: Arc<dyn OccupancyFactory>,
    pub sessions: RwLock<HashMap<String, Arc<SessionSlot>>>,
    pub unbound_mux: Mutex<Vec<(u64, MuxSink)>>,
    next_mux_id: AtomicU64,
    pub resource_driver: Mutex<Option<Box<dyn OccupancyEngine>>>,
    pub in_process_downlink: broadcast::Sender<Notification>,
    pub reload_cancel: Mutex<Option<CancellationToken>>,
    pub shutting_down: AtomicBool,
    pub fallback_session: String,
    pub write_lock_linger: std::sync::Mutex<Duration>,
    pub sessions_dir: std::sync::Mutex<PathBuf>,
    seq_watermarks: std::sync::Mutex<HashMap<String, u64>>,
}

#[derive(Clone)]
pub struct ReloadBaseline {
    pub cwd: PathBuf,
    pub agent_dir: PathBuf,
    pub project_trusted: bool,
    pub default_model_id: Option<String>,
    pub trust_audit: xylitol_runtime::TrustAuditConfig,
}

pub struct SessionSlot {
    pub session_id: String,
    pub journal: Mutex<EventJournal>,
    pub driver: Arc<Mutex<Option<Box<dyn OccupancyEngine>>>>,
    pub subscribers: Mutex<HashMap<u64, MuxSink>>,
    observing: Mutex<HashSet<u64>>,
    linger_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    eviction_gate: Mutex<()>,
    pub gateway: Arc<ReverseRpcGateway>,
    pub in_process_downlink: broadcast::Sender<Notification>,
    next_conn: AtomicU64,
    resync_offered: AtomicBool,
    turn_inflight: AtomicBool,
    pending_model: std::sync::Mutex<Option<String>>,
    pending_thinking: std::sync::Mutex<Option<String>>,
    pub latest_tape: std::sync::Mutex<Option<xylitol_plugin_sdk::message::AgentMessage>>,
    mcp_watch_started: AtomicBool,
    pub bash_run_cancel: std::sync::Mutex<Option<CancellationToken>>,
    pub idempotency: IdempotencyLedger,
    pub loop_registry: Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>>,
    pub workspaces: std::sync::Mutex<WorkspaceSet>,
    automation: Mutex<Option<Arc<Mutex<AutomationEngine>>>>,
    automation_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

#[derive(Clone, Debug, Default)]
pub struct WorkspaceSet {
    pub primary: PathBuf,
    pub additional: Vec<PathBuf>,
}

impl HostState {
    pub fn new(
        ports: RuntimeBaseline,
        reload: ReloadBaseline,
        factory: Arc<dyn OccupancyFactory>,
        fallback_session: String,
    ) -> Arc<Self> {
        let (in_process_downlink, _) = broadcast::channel(MUX_CHAN_CAP);
        Arc::new(Self {
            ports,
            reload,
            factory,
            sessions: RwLock::new(HashMap::new()),
            unbound_mux: Mutex::new(Vec::new()),
            next_mux_id: AtomicU64::new(1),
            resource_driver: Mutex::new(None),
            in_process_downlink,
            reload_cancel: Mutex::new(None),
            shutting_down: AtomicBool::new(false),
            fallback_session,
            write_lock_linger: std::sync::Mutex::new(Duration::from_secs(60)),
            sessions_dir: std::sync::Mutex::new(PathBuf::new()),
            seq_watermarks: std::sync::Mutex::new(HashMap::new()),
        })
    }

    pub fn remember_sessions_dir(self: Arc<Self>, sessions_dir: PathBuf) -> Arc<Self> {
        if let Ok(mut g) = self.sessions_dir.lock() {
            *g = sessions_dir;
        }
        self
    }

    pub fn from_bootstrap(
        ports: RuntimeBaseline,
        reload: ReloadBaseline,
        factory: Arc<dyn OccupancyFactory>,
        fallback_session: String,
    ) -> Arc<Self> {
        Self::new(ports, reload, factory, fallback_session)
    }

    pub fn arm_test_linger(self: Arc<Self>) -> Arc<Self> {
        if let Ok(mut g) = self.write_lock_linger.lock() {
            *g = Duration::from_millis(80);
        }
        self
    }

    pub async fn slot(&self, session_id: &str) -> Arc<SessionSlot> {
        {
            let map = self.sessions.read().await;
            if let Some(s) = map.get(session_id) {
                return s.clone();
            }
        }
        let watermark = self.seq_watermark(session_id);
        let mut map = self.sessions.write().await;
        map.entry(session_id.to_string())
            .or_insert_with(|| {
                SessionSlot::with_seq(
                    session_id,
                    self.in_process_downlink.clone(),
                    self.factory.loop_registry(),
                    watermark,
                )
            })
            .clone()
    }

    async fn existing_slot(&self, session_id: &str) -> Option<Arc<SessionSlot>> {
        self.sessions.read().await.get(session_id).cloned()
    }

    fn seq_watermark(&self, session_id: &str) -> u64 {
        self.seq_watermarks
            .lock()
            .ok()
            .and_then(|g| g.get(session_id).copied())
            .unwrap_or(1)
    }

    fn remember_seq_watermark(&self, session_id: &str, next_seq: u64) {
        if let Ok(mut g) = self.seq_watermarks.lock() {
            g.insert(session_id.to_string(), next_seq.max(1));
        }
    }

    pub async fn evict_slot(&self, slot: &Arc<SessionSlot>) {
        let next_seq = slot.journal.lock().await.next_seq();
        self.remember_seq_watermark(&slot.session_id, next_seq);
        if let Some(task) = slot.automation_task.lock().await.take() {
            task.abort();
        }
        *slot.automation.lock().await = None;
        self.factory.release_session(&slot.session_id);
        *slot.driver.lock().await = None;
        self.sessions.write().await.remove(&slot.session_id);
    }

    pub fn session_id_from_payload(&self, payload: &Value) -> String {
        payload
            .get("session_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| self.fallback_session.clone())
    }

    pub async fn register_unbound_mux(&self, tx: MuxSink) -> u64 {
        let id = self.next_mux_id.fetch_add(1, Ordering::Relaxed);
        self.unbound_mux.lock().await.push((id, tx));
        id
    }

    pub async fn bind_mux_to_session(
        &self,
        slot: &SessionSlot,
        last_seq: u64,
        mux_id: Option<u64>,
    ) -> RpcResult {
        let bound = match mux_id {
            Some(id) => self.bind_single_mux(slot, id).await,
            None => self.bind_all_unbound_muxes(slot).await,
        };
        Self::cancel_write_linger(slot).await;
        let inflight = slot.turn_inflight_active();
        let observe = inflight && last_seq == 0;
        if observe {
            slot.observe(bound.iter().copied()).await;
        }
        let result = slot
            .replay_or_resync(&bound, observe, inflight, last_seq)
            .await;
        slot.sync_model_downlink().await;
        result
    }

    async fn bind_single_mux(&self, slot: &SessionSlot, id: u64) -> Vec<u64> {
        let tx = {
            let mut unbound = self.unbound_mux.lock().await;
            unbound
                .iter()
                .position(|(other, _)| *other == id)
                .map(|pos| unbound.swap_remove(pos).1)
        };
        if let Some(tx) = tx {
            slot.subscribers.lock().await.entry(id).or_insert(tx);
        }
        vec![id]
    }

    async fn bind_all_unbound_muxes(&self, slot: &SessionSlot) -> Vec<u64> {
        let pending = {
            let mut unbound = self.unbound_mux.lock().await;
            std::mem::take(&mut *unbound)
        };
        let mut bound = Vec::new();
        for (id, tx) in pending {
            slot.subscribers.lock().await.insert(id, tx);
            bound.push(id);
        }
        bound
    }

    async fn cancel_write_linger(slot: &SessionSlot) {
        if let Some(task) = slot.linger_task.lock().await.take() {
            task.abort();
        }
    }

    fn linger_duration(&self) -> Duration {
        *self
            .write_lock_linger
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    pub fn occupancy_lock_path(&self, session_id: &str) -> PathBuf {
        self.sessions_dir
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .join(session_id)
            .join("session.lock")
    }

    async fn live_subscriber_count(slot: &SessionSlot) -> usize {
        let mut subs = slot.subscribers.lock().await;
        let before = subs.len();
        subs.retain(|_, tx| !tx.is_closed());
        if subs.len() != before {
            slot.observing
                .lock()
                .await
                .retain(|id| subs.contains_key(id));
        }
        subs.len()
    }

    pub async fn respond(&self, rpc_id: &str, payload: Value) -> bool {
        let map = self.sessions.read().await;
        for slot in map.values() {
            if slot.gateway.handle_respond(rpc_id, &payload) {
                return true;
            }
        }
        false
    }

    async fn ensure_resource_driver(
        &self,
    ) -> tokio::sync::MutexGuard<'_, Option<Box<dyn OccupancyEngine>>> {
        let mut guard = self.resource_driver.lock().await;
        if guard.is_none() {
            *guard = Some(self.factory.new_driver());
        }
        guard
    }

    pub async fn reload_resources(&self) -> Result<RuntimeReloadReport, XyDriverError> {
        let cancel = CancellationToken::new();
        *self.reload_cancel.lock().await = Some(cancel.clone());
        let slots: Vec<Arc<SessionSlot>> = {
            let sessions = self.sessions.read().await;
            sessions.values().cloned().collect()
        };
        let mut report = None;
        for slot in slots {
            let mut g = slot.driver.lock().await;
            if let Some(driver) = g.as_mut() {
                report = Some(driver.reload_runtime(&cancel).await);
                if matches!(&report, Some(Err(_))) {
                    break;
                }
            }
        }
        let result = match report {
            Some(r) => {
                if r.is_ok() {
                    let mut guard = self.resource_driver.lock().await;
                    if let Some(driver) = guard.as_mut() {
                        let _ = driver.reload_runtime(&cancel).await;
                    }
                }
                r
            }
            None => {
                let mut guard = self.ensure_resource_driver().await;
                guard
                    .as_mut()
                    .expect("resource driver initialized")
                    .reload_runtime(&cancel)
                    .await
            }
        };
        *self.reload_cancel.lock().await = None;
        result
    }

    pub async fn abort_reload(&self) -> bool {
        let guard = self.reload_cancel.lock().await;
        if let Some(cancel) = guard.as_ref() {
            cancel.cancel();
            true
        } else {
            false
        }
    }

    pub async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
        self.loaded_resources_snapshot_for("").await
    }

    pub async fn loaded_resources_snapshot_for(&self, session_id: &str) -> LoadedResourcesSnapshot {
        if !session_id.is_empty() {
            let slot = {
                let sessions = self.sessions.read().await;
                sessions.get(session_id).cloned()
            };
            if let Some(slot) = slot {
                let mut guard = slot.driver.lock().await;
                if let Some(driver) = guard.as_mut() {
                    let _ = driver.poll_prepare_progress().await;
                    return driver.loaded_resources_snapshot().await;
                }
            }
        }
        let slots: Vec<Arc<SessionSlot>> = {
            let sessions = self.sessions.read().await;
            sessions.values().cloned().collect()
        };
        for slot in slots {
            let mut guard = slot.driver.lock().await;
            if let Some(driver) = guard.as_mut() {
                let _ = driver.poll_prepare_progress().await;
                return driver.loaded_resources_snapshot().await;
            }
        }
        let mut guard = self.ensure_resource_driver().await;
        let driver = guard.as_mut().expect("resource driver initialized");
        driver.begin_session_prepare().await;
        let _ = driver.poll_prepare_progress().await;
        driver.loaded_resources_snapshot().await
    }
}

pub struct RunInflightGuard {
    slot: Arc<SessionSlot>,
    active: bool,
}

impl RunInflightGuard {
    pub fn try_acquire(slot: &Arc<SessionSlot>) -> Option<Self> {
        if slot
            .turn_inflight
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            Some(Self {
                slot: slot.clone(),
                active: true,
            })
        } else {
            None
        }
    }
}

impl Drop for RunInflightGuard {
    fn drop(&mut self) {
        if self.active {
            self.slot.turn_inflight.store(false, Ordering::SeqCst);
        }
    }
}

impl SessionSlot {
    pub fn new(
        session_id: impl Into<String>,
        in_process_downlink: broadcast::Sender<Notification>,
        loop_registry: Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>>,
    ) -> Arc<Self> {
        Self::with_seq(session_id, in_process_downlink, loop_registry, 1)
    }

    pub fn with_seq(
        session_id: impl Into<String>,
        in_process_downlink: broadcast::Sender<Notification>,
        root_loop_registry: Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>>,
        next_seq: u64,
    ) -> Arc<Self> {
        let session_id = session_id.into();
        Arc::new(Self {
            journal: Mutex::new(EventJournal::with_default_capacity_and_seq(
                &session_id,
                next_seq,
            )),
            session_id,
            driver: Arc::new(Mutex::new(None)),
            subscribers: Mutex::new(HashMap::new()),
            observing: Mutex::new(HashSet::new()),
            linger_task: Mutex::new(None),
            eviction_gate: Mutex::new(()),
            gateway: Arc::new(ReverseRpcGateway::new()),
            in_process_downlink,
            next_conn: AtomicU64::new(1),
            resync_offered: AtomicBool::new(false),
            turn_inflight: AtomicBool::new(false),
            pending_model: std::sync::Mutex::new(None),
            pending_thinking: std::sync::Mutex::new(None),
            latest_tape: std::sync::Mutex::new(None),
            mcp_watch_started: AtomicBool::new(false),
            bash_run_cancel: std::sync::Mutex::new(None),
            idempotency: IdempotencyLedger::default(),
            workspaces: std::sync::Mutex::new(WorkspaceSet::default()),
            automation: Mutex::new(None),
            automation_task: Mutex::new(None),
            loop_registry: root_loop_registry.map(|registry| Arc::new(registry.derive())),
        })
    }

    pub fn turn_inflight_active(&self) -> bool {
        self.turn_inflight.load(Ordering::SeqCst)
    }

    pub async fn add_subscriber(&self, tx: MuxSink) -> u64 {
        let id = self.next_conn.fetch_add(1, Ordering::Relaxed);
        self.subscribers.lock().await.insert(id, tx);
        id
    }

    fn clear_pending_runtime(&self) {
        if let Ok(mut g) = self.pending_model.lock() {
            *g = None;
        }
        if let Ok(mut g) = self.pending_thinking.lock() {
            *g = None;
        }
    }

    async fn flush_pending_runtime(&self) {
        let model = self.pending_model.lock().ok().and_then(|mut g| g.take());
        let thinking = self.pending_thinking.lock().ok().and_then(|mut g| g.take());
        if model.is_none() && thinking.is_none() {
            return;
        }
        let mut guard = self.driver.lock().await;
        let Some(driver) = guard.as_mut() else {
            return;
        };
        if let Some(id) = model.as_deref()
            && let Err(e) = driver.select_model(id).await
        {
            log::warn!(target: "xylitol::host", "flush pending model {id}: {e}");
        }
        if let Some(level) = thinking
            && let Err(e) = driver.set_thinking_level(level).await
        {
            log::warn!(target: "xylitol::host", "flush pending thinking: {e}");
        }
    }

    pub fn mark_run_inflight_for_test(&self, on: bool) {
        self.turn_inflight.store(on, Ordering::SeqCst);
    }

    pub async fn flush_pending_runtime_for_test(&self) {
        self.flush_pending_runtime().await;
    }

    fn subscribed_payload(
        &self,
        seq: u64,
        inflight: bool,
        want_partial: bool,
    ) -> SessionSubscribedPayload {
        let partial = self.partial_tape(inflight, want_partial);
        SessionSubscribedPayload {
            session_id: self.session_id.clone(),
            seq,
            inflight,
            partial,
        }
    }

    fn partial_tape(&self, inflight: bool, want_partial: bool) -> Option<Value> {
        if !(want_partial && inflight) {
            return None;
        }
        self.latest_tape
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .and_then(|m| serde_json::to_value(m).ok())
    }

    fn resync_notification(&self) -> Notification {
        downlink_notification(
            SESSION_RESYNC_REQUIRED,
            SessionResyncRequiredPayload {
                session_id: self.session_id.clone(),
            },
        )
    }

    async fn replay_or_resync(
        &self,
        bound: &[u64],
        observe: bool,
        inflight: bool,
        last_seq: u64,
    ) -> RpcResult {
        let want_partial = inflight && last_seq > 0;
        let journal = self.journal.lock().await;
        let wrapped_gap = journal_wrapped_gap(&journal, last_seq);
        if wrapped_gap && !self.resync_offered.swap(true, Ordering::SeqCst) {
            drop(journal);
            self.broadcast(self.resync_notification()).await;
            return RpcResult::ok_value(self.subscribed_result(0, inflight, want_partial));
        }
        match journal.replay_from(last_seq) {
            None => {
                drop(journal);
                self.resync_offered.store(true, Ordering::SeqCst);
                self.broadcast(self.resync_notification()).await;
                RpcResult::ok_value(self.subscribed_result(0, inflight, want_partial))
            }
            Some(events) => {
                let seq = journal.max_seq();
                drop(journal);
                if !observe {
                    for (ev_seq, event) in events {
                        let msg = downlink_notification(
                            SESSION_EVENT,
                            SessionEventPayload {
                                session_id: self.session_id.clone(),
                                seq: ev_seq,
                                event,
                            },
                        );
                        for id in bound {
                            self.send_to(*id, &msg).await;
                        }
                    }
                }
                self.broadcast(downlink_notification(
                    SESSION_SUBSCRIBED,
                    self.subscribed_payload(seq, inflight, want_partial),
                ))
                .await;
                RpcResult::ok_value(self.subscribed_result(seq, inflight, want_partial))
            }
        }
    }

    fn subscribed_result(&self, seq: u64, inflight: bool, want_partial: bool) -> Value {
        serde_json::to_value(self.subscribed_payload(seq, inflight, want_partial))
            .unwrap_or(Value::Null)
    }

    fn observe_passes(note: &Notification) -> bool {
        if note.method != SESSION_EVENT {
            return true;
        }
        note.params
            .get("event")
            .and_then(|v| serde_json::from_value::<xylitol_protocol::Event>(v.clone()).ok())
            .map(|ev| matches!(ev, xylitol_protocol::Event::AgentEnd { .. }))
            .unwrap_or(true)
    }

    pub async fn observe<I: IntoIterator<Item = u64>>(&self, ids: I) {
        self.observing.lock().await.extend(ids);
    }

    pub async fn broadcast(&self, msg: Notification) {
        let _ = self.in_process_downlink.send(msg.clone());
        let observe_filter = !Self::observe_passes(&msg);
        let mut subs = self.subscribers.lock().await;
        let mut observing = self.observing.lock().await;
        let mut dead = Vec::new();
        let resync = self.resync_notification();
        for (id, tx) in subs.iter() {
            if observe_filter && observing.contains(id) {
                continue;
            }
            match tx.try_send(msg.clone()) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    if tx.try_send(resync.clone()).is_err() {
                        dead.push(*id);
                    }
                }
                Err(mpsc::error::TrySendError::Closed(_)) => dead.push(*id),
            }
        }
        for id in dead {
            subs.remove(&id);
            observing.remove(&id);
        }
    }

    async fn send_to(&self, id: u64, msg: &Notification) -> bool {
        let subs = self.subscribers.lock().await;
        match subs.get(&id) {
            Some(tx) => tx.send(msg.clone()).await.is_ok(),
            None => false,
        }
    }

    pub async fn append_and_push(&self, event: &XyEvent) {
        let Some(event) = self.prepare_event(event).await else {
            return;
        };
        let seq = {
            let mut j = self.journal.lock().await;
            j.append(event.clone())
        };
        self.broadcast(downlink_notification(
            SESSION_EVENT,
            SessionEventPayload {
                session_id: self.session_id.clone(),
                seq,
                event,
            },
        ))
        .await;
    }

    async fn prepare_event(&self, event: &XyEvent) -> Option<xylitol_protocol::Event> {
        if self.try_push_tape(event).await {
            return None;
        }
        self.clear_tape_on_agent_end(event);
        match xylitol_wire::event_to_wire(event) {
            Ok(event) => Some(event),
            Err(e) => {
                log::error!(target: "xylitol::host", "session event dropped: {e}");
                None
            }
        }
    }

    async fn try_push_tape(&self, event: &XyEvent) -> bool {
        if !is_tape_event(event) {
            return false;
        }
        self.push_tape(event).await;
        true
    }

    fn clear_tape_on_agent_end(&self, event: &XyEvent) {
        if matches!(event, XyEvent::AgentEnd { .. })
            && let Ok(mut g) = self.latest_tape.lock()
        {
            *g = None;
        }
    }

    async fn push_tape(&self, event: &XyEvent) {
        if let XyEvent::MessageUpdate { message, .. } = event
            && let Some(message) = message
            && let Ok(mut g) = self.latest_tape.lock()
        {
            *g = Some(message.clone());
        }
        let event = match xylitol_wire::event_to_wire(event) {
            Ok(event) => event,
            Err(e) => {
                log::error!(target: "xylitol::host", "session tape dropped: {e}");
                return;
            }
        };
        self.broadcast(downlink_notification(
            SESSION_EVENT,
            SessionEventPayload {
                session_id: self.session_id.clone(),
                seq: self.journal.lock().await.max_seq(),
                event,
            },
        ))
        .await;
    }

    pub async fn sync_model_downlink(&self) {
        let Some((provider, model_id)) = self.writer_model().await else {
            return;
        };
        self.broadcast(downlink_notification(
            SESSION_MODEL,
            SessionModelPayload {
                session_id: self.session_id.clone(),
                provider,
                model_id,
            },
        ))
        .await;
    }

    async fn writer_model(&self) -> Option<(String, String)> {
        let mut g = self.driver.lock().await;
        let d = g.as_mut()?;
        let id = d.current_model()?.id;
        let provider = d.model_provider(&id).unwrap_or_default();
        Some((provider, id))
    }

    pub async fn push_resources(&self, snap: LoadedResourcesSnapshot) {
        self.broadcast(downlink_notification(
            SESSION_RESOURCES,
            SessionResourcesPayload {
                session_id: self.session_id.clone(),
                snapshot: serde_json::to_value(&snap).unwrap_or(Value::Null),
            },
        ))
        .await;
    }

    pub async fn push_bash_output(&self, chunk: xylitol_plugin_sdk::BashChunk) {
        self.broadcast(downlink_notification(
            BASH_OUTPUT,
            BashOutputPayload {
                session_id: self.session_id.clone(),
                bash_id: chunk.bash_id,
                seq: chunk.seq,
                data: chunk.data,
            },
        ))
        .await;
    }

    pub fn ensure_mcp_resources_watch(self: &Arc<Self>) {
        if self.mcp_watch_started.swap(true, Ordering::SeqCst) {
            return;
        }
        let weak = Arc::downgrade(self);
        tokio::spawn(async move {
            let mut last: Option<LoadedResourcesSnapshot> = None;
            loop {
                let Some(slot) = weak.upgrade() else {
                    return;
                };
                let snap = {
                    let mut g = slot.driver.lock().await;
                    let Some(driver) = g.as_mut() else {
                        drop(g);
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        continue;
                    };
                    let _ = driver.poll_prepare_progress().await;
                    driver.loaded_resources_snapshot().await
                };
                if last.as_ref() != Some(&snap) {
                    last = Some(snap.clone());
                    slot.push_resources(snap).await;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        });
    }

    pub async fn request_approval(
        &self,
        call_id: String,
    ) -> tokio::sync::oneshot::Receiver<ReverseRpcResult> {
        let rx = self.gateway.register(call_id.clone());
        self.broadcast(downlink_notification(
            APPROVAL_REQUESTED,
            ApprovalRequestedPayload { call_id },
        ))
        .await;
        rx
    }

    pub async fn request_question(
        &self,
        call_id: String,
        questions: serde_json::Value,
    ) -> tokio::sync::oneshot::Receiver<ReverseRpcResult> {
        let rx = self.gateway.register(call_id.clone());
        let payload = serde_json::to_value(QuestionRequestedPayload { call_id, questions })
            .unwrap_or(Value::Null);
        self.broadcast(Notification {
            method: QUESTION_REQUESTED.into(),
            params: payload,
        })
        .await;
        rx
    }
}

struct SlotAskGateway {
    slot: Arc<SessionSlot>,
}

#[async_trait::async_trait]
impl AskUserGateway for SlotAskGateway {
    async fn prompt(&self, args: AskArgs) -> Result<String, XyToolError> {
        let rpc_id = uuid::Uuid::new_v4().to_string();
        let questions = serde_json::to_value(&args.questions).unwrap_or(Value::Null);
        let rx = self.slot.request_question(rpc_id.clone(), questions).await;
        match timeout(crate::occupancy::ws::REVERSE_RPC_TIMEOUT, rx).await {
            Ok(Ok(ReverseRpcResult::Answered(s))) => Ok(s),
            Ok(Ok(ReverseRpcResult::Approved)) => Ok("{}".into()),
            Ok(Ok(ReverseRpcResult::Denied)) => Err(XyToolError::Aborted),
            Ok(Err(_)) | Err(_) => {
                self.slot.gateway.remove(&rpc_id);
                Err(XyToolError::Timeout(
                    crate::occupancy::ws::REVERSE_RPC_TIMEOUT,
                ))
            }
        }
    }
}

#[async_trait::async_trait]
impl McpElicitationGateway for SlotAskGateway {
    async fn elicit(&self, request: McpElicitationRequest) -> McpElicitationResponse {
        let questions = elicitation_questions(&request.schema, &request.message);
        let rpc_id = uuid::Uuid::new_v4().to_string();
        let questions_value = serde_json::to_value(&questions).unwrap_or(Value::Null);
        let rx = self
            .slot
            .request_question(rpc_id.clone(), questions_value)
            .await;
        let json = match timeout(std::time::Duration::from_secs(300), rx).await {
            Ok(Ok(ReverseRpcResult::Answered(s))) => s,
            Ok(Ok(ReverseRpcResult::Approved)) => "{}".into(),
            Ok(Ok(ReverseRpcResult::Denied)) => String::new(),
            Ok(Err(_)) | Err(_) => {
                self.slot.gateway.remove(&rpc_id);
                String::new()
            }
        };
        if json.is_empty() {
            return McpElicitationResponse {
                action: McpElicitationAction::Decline,
                content: None,
            };
        }
        parse_elicitation_response(&json, &request.schema)
    }
}

fn elicitation_questions(schema: &Value, message: &str) -> Vec<AskQuestionArg> {
    let mut questions = vec![AskQuestionArg {
        id: "action".into(),
        prompt: message.to_string(),
        label: Some("Elicitation".into()),
        mode: AskModeArg::Single,
        options: vec![
            AskOptionArg {
                value: "accept".into(),
                label: "Accept".into(),
                description: None,
                recommended: true,
            },
            AskOptionArg {
                value: "decline".into(),
                label: "Decline".into(),
                description: None,
                recommended: false,
            },
            AskOptionArg {
                value: "cancel".into(),
                label: "Cancel".into(),
                description: None,
                recommended: false,
            },
        ],
        allow_other: false,
    }];
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return questions;
    };
    let required: Vec<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    for (name, spec) in properties {
        let Some(spec) = spec.as_object() else {
            continue;
        };
        let kind = spec.get("type").and_then(Value::as_str).unwrap_or("string");
        let mut options = Vec::new();
        if let Some(values) = spec.get("enum").and_then(Value::as_array) {
            for value in values.iter().filter_map(Value::as_str) {
                options.push(AskOptionArg {
                    value: value.to_string(),
                    label: value.to_string(),
                    description: None,
                    recommended: false,
                });
            }
        } else if kind == "boolean" {
            options.push(AskOptionArg {
                value: "true".into(),
                label: "true".into(),
                description: None,
                recommended: false,
            });
            options.push(AskOptionArg {
                value: "false".into(),
                label: "false".into(),
                description: None,
                recommended: false,
            });
        }
        let description = spec
            .get("description")
            .and_then(Value::as_str)
            .or_else(|| spec.get("title").and_then(Value::as_str))
            .unwrap_or("");
        let marker = if required.contains(&name.as_str()) {
            "required"
        } else {
            "optional"
        };
        let label = format!("{name} ({kind}, {marker})");
        questions.push(AskQuestionArg {
            id: name.clone(),
            prompt: if description.is_empty() {
                label.clone()
            } else {
                format!("{description} [{label}]")
            },
            label: Some(label),
            mode: AskModeArg::Single,
            options,
            allow_other: true,
        });
    }
    questions
}

fn parse_elicitation_response(json: &str, schema: &Value) -> McpElicitationResponse {
    let value: Value = serde_json::from_str(json).unwrap_or(Value::Null);
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("skipped");
    if status != "answered" {
        return McpElicitationResponse {
            action: McpElicitationAction::Decline,
            content: None,
        };
    }
    let answers = value
        .get("answers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let answer_for = |id: &str| -> Option<String> {
        answers
            .iter()
            .find(|answer| answer.get("id").and_then(Value::as_str) == Some(id))
            .and_then(|answer| answer.get("values").and_then(Value::as_array))
            .and_then(|values| values.first())
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    let action = answer_for("action")
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    match action.as_str() {
        "accept" => {
            let mut content = serde_json::Map::new();
            if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
                for (name, spec) in properties {
                    let Some(raw) = answer_for(name) else {
                        continue;
                    };
                    if raw.is_empty() {
                        continue;
                    }
                    let kind = spec.get("type").and_then(Value::as_str).unwrap_or("string");
                    let value = match kind {
                        "boolean" => Value::Bool(raw.eq_ignore_ascii_case("true")),
                        "integer" => raw
                            .parse::<i64>()
                            .map(|number| Value::Number(number.into()))
                            .unwrap_or(Value::String(raw)),
                        "number" => raw
                            .parse::<f64>()
                            .ok()
                            .and_then(serde_json::Number::from_f64)
                            .map(Value::Number)
                            .unwrap_or(Value::String(raw)),
                        _ => Value::String(raw),
                    };
                    content.insert(name.clone(), value);
                }
            }
            McpElicitationResponse {
                action: McpElicitationAction::Accept,
                content: Some(Value::Object(content)),
            }
        }
        "cancel" => McpElicitationResponse {
            action: McpElicitationAction::Cancel,
            content: None,
        },
        _ => McpElicitationResponse {
            action: McpElicitationAction::Decline,
            content: None,
        },
    }
}

#[async_trait::async_trait]
impl OccupiedHost for HostState {
    fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(Ordering::Relaxed)
    }

    fn mark_shutting_down(&self) {
        self.shutting_down.store(true, Ordering::Relaxed);
    }

    async fn register_unbound_mux(&self, tx: MuxSink) -> u64 {
        HostState::register_unbound_mux(self, tx).await
    }

    async fn dispatch_text(self: Arc<Self>, body: &[u8], mux_id: Option<u64>) -> Reply {
        envelope::dispatch_text(&self, body, mux_id).await
    }

    async fn prune_mux(self: Arc<Self>, clean_close: bool) {
        prune_mux_and_maybe_release(&self, clean_close).await;
    }
}

pub fn is_writer_method(method: &str) -> bool {
    xylitol_protocol::registry::lookup(method)
        .is_some_and(|e| e.auth == xylitol_protocol::registry::Auth::Writer)
}

fn parse_workspaces(payload: &Value) -> (Option<PathBuf>, Vec<PathBuf>) {
    let mut paths: Vec<PathBuf> = payload
        .get("workspaces")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|raw| !raw.is_empty())
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default();
    if paths.is_empty()
        && let Some(cwd) = payload
            .get("cwd")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|raw| !raw.is_empty())
    {
        paths.push(PathBuf::from(cwd));
    }
    let primary = paths.first().cloned();
    let additional = paths.into_iter().skip(1).collect();
    (primary, additional)
}

fn workspace_from_payload(host: &HostState, payload: &Value) -> PathBuf {
    parse_workspaces(payload)
        .0
        .unwrap_or_else(|| host.reload.cwd.clone())
}

fn session_workspace(host: &HostState, slot: &SessionSlot, payload: &Value) -> PathBuf {
    if let Ok(guard) = slot.workspaces.lock()
        && !guard.primary.as_os_str().is_empty()
    {
        return guard.primary.clone();
    }
    workspace_from_payload(host, payload)
}

async fn session_primary_workspace(host: &HostState, session_id: &str) -> Option<PathBuf> {
    let entries = host.ports.store.load_entries(session_id).await.ok()?;
    entries.into_iter().find_map(|entry| match entry {
        SessionEntry::Header(header) if !header.cwd.trim().is_empty() => {
            Some(PathBuf::from(header.cwd))
        }
        _ => None,
    })
}

async fn resolve_workspace_set(
    host: &HostState,
    session_id: &str,
    payload: &Value,
) -> WorkspaceSet {
    let (client_primary, additional) = parse_workspaces(payload);
    let primary = session_primary_workspace(host, session_id)
        .await
        .or(client_primary)
        .unwrap_or_else(|| host.reload.cwd.clone());
    if !additional.is_empty() {
        log::warn!(
            target: "xylitol::host",
            "additional workspace set is not supported yet; binding primary {}",
            primary.display()
        );
    }
    WorkspaceSet {
        primary,
        additional,
    }
}

fn same_workspace(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => a == b,
    }
}

fn project_trusted_for(host: &HostState, workspace: &Path) -> bool {
    let mgr = xylitol_runtime::trust::TrustManager::with_audit(
        &host.reload.agent_dir,
        host.reload.trust_audit.clone(),
    );
    let cwd = workspace.to_string_lossy();
    let audit = xylitol_runtime::TrustAuditContext::new(
        xylitol_runtime::TrustActor::Host,
        xylitol_runtime::TrustSource::Resume,
    );
    match mgr.recheck(&cwd, &audit) {
        Some(decision) => decision,
        None if same_workspace(workspace, &host.reload.cwd) => host.reload.project_trusted,
        None => {
            xylitol_runtime::trust::resolve_project_trusted(
                &mgr,
                &cwd,
                None,
                xylitol_runtime::trust::DefaultProjectTrust::default(),
                false,
                &audit,
                |_| None,
            )
            .trusted
        }
    }
}

#[allow(dead_code)]
pub async fn materialize_writer(
    host: &HostState,
    slot: &Arc<SessionSlot>,
) -> Result<(), XyDriverError> {
    materialize_writer_at(host, slot, &host.reload.cwd).await
}

pub async fn materialize_writer_at(
    host: &HostState,
    slot: &Arc<SessionSlot>,
    workspace: &Path,
) -> Result<(), XyDriverError> {
    let mut guard = slot.driver.lock().await;
    if let Some(driver) = guard.as_mut() {
        driver.set_workspaces(vec![workspace.to_string_lossy().into_owned()]);
        drop(guard);
        slot.ensure_mcp_resources_watch();
        start_automation_if_needed(host, slot).await;
        return Ok(());
    }
    let mut driver = host.factory.new_driver_at(workspace);
    driver.set_workspaces(vec![workspace.to_string_lossy().into_owned()]);
    if let Some(registry) = slot.loop_registry.clone() {
        driver.set_loop_middleware_scoped(Some(registry), xylitol_plugin_sdk::LoopScope::Session);
    }
    let _ = project_trusted_for(host, workspace);
    driver.bind_session(&slot.session_id)?;
    let ask_gateway = Arc::new(SlotAskGateway { slot: slot.clone() });
    driver.install_ask_tool(ask_gateway.clone());
    xylitol_plugin_sdk::mcp::set_mcp_elicitation_gateway(ask_gateway);
    xylitol_plugin_sdk::mcp::set_mcp_oauth_gateway(Arc::new(HostOAuthGateway::with_slot(
        slot.clone(),
    )));
    driver.begin_session_prepare().await;
    restore_default_model(host, driver.as_mut()).await;
    *guard = Some(driver);
    drop(guard);
    slot.ensure_mcp_resources_watch();
    start_automation_if_needed(host, slot).await;
    Ok(())
}

async fn start_automation_if_needed(host: &HostState, slot: &Arc<SessionSlot>) {
    let mut task = slot.automation_task.lock().await;
    if task.is_some() {
        return;
    }
    let engine = Arc::new(Mutex::new(AutomationEngine::new(
        host.ports.store.clone(),
        slot.session_id.clone(),
    )));
    *slot.automation.lock().await = Some(engine.clone());
    let slot_for_task = slot.clone();
    let handle = tokio::spawn(async move {
        automation_loop(slot_for_task, engine).await;
    });
    *task = Some(handle);
}

async fn automation_loop(slot: Arc<SessionSlot>, engine: Arc<Mutex<AutomationEngine>>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;
        let busy = slot.turn_inflight_active();
        let permission_required = HostState::live_subscriber_count(&slot).await == 0;
        let actions = {
            let mut guard = engine.lock().await;
            match guard.admit(Local::now(), busy, permission_required).await {
                Ok(actions) => actions,
                Err(error) => {
                    log::warn!(
                        target: "xylitol::host",
                        "automation admit failed session={} error={error}",
                        slot.session_id
                    );
                    Vec::new()
                }
            }
        };
        for action in actions {
            if let AutomationAction::Run {
                trigger_id, prompt, ..
            } = action
            {
                run_automation(&slot, &engine, &trigger_id, &prompt).await;
            }
        }
    }
}

async fn run_automation(
    slot: &Arc<SessionSlot>,
    engine: &Arc<Mutex<AutomationEngine>>,
    trigger_id: &str,
    prompt: &str,
) {
    let Some(guard) = RunInflightGuard::try_acquire(slot) else {
        return;
    };
    let stream = {
        let mut guard = slot.driver.lock().await;
        let Some(driver) = guard.as_mut() else {
            return;
        };
        driver.run_with_context(prompt, Vec::new()).await
    };
    let mut stream = stream;
    let mut outcome = RunOutcome::Success;
    while let Some(event) = stream.next().await {
        if matches!(event, XyEvent::Error(_)) {
            outcome = RunOutcome::Failed;
        }
        slot.append_and_push(&event).await;
    }
    drop(guard);
    slot.flush_pending_runtime().await;
    if let Err(error) = engine
        .lock()
        .await
        .finish(trigger_id, outcome, Local::now())
        .await
    {
        log::warn!(
            target: "xylitol::host",
            "automation finish failed session={} error={error}",
            slot.session_id
        );
    }
}

fn new_reader_driver(host: &HostState) -> Box<dyn OccupancyEngine> {
    host.factory.new_driver()
}

async fn materialize_reader(
    host: &HostState,
    session_id: &str,
) -> Result<Box<dyn OccupancyEngine>, XyDriverError> {
    let mut driver = new_reader_driver(host);
    driver.switch_session(session_id).await?;
    Ok(driver)
}

async fn defer_runtime_setting(slot: &SessionSlot, method: &str, payload: &Value) -> RpcResult {
    let mut g = slot.driver.lock().await;
    let Some(driver) = g.as_mut() else {
        return RpcResult::error("unavailable", "no writer engine");
    };
    match method {
        "set_model" => {
            let id = payload
                .get("model_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let Some(model) = driver.available_models().into_iter().find(|m| m.id == id) else {
                return RpcResult::error("not_found", format!("model not found: {id}"));
            };
            drop(g);
            if let Ok(mut pending) = slot.pending_model.lock() {
                *pending = Some(model.id.clone());
            }
            RpcResult::ok_value(model_data(&model))
        }
        "cycle_model" => {
            let list = driver.available_models();
            if list.is_empty() {
                return RpcResult::error("not_found", "no models available");
            }
            let current = driver.current_model().map(|m| m.id);
            let idx = current
                .as_ref()
                .and_then(|cur| list.iter().position(|m| m.id == *cur))
                .unwrap_or(0);
            let model = list[(idx + 1) % list.len()].clone();
            drop(g);
            if let Ok(mut pending) = slot.pending_model.lock() {
                *pending = Some(model.id.clone());
            }
            RpcResult::ok_value(model_data(&model))
        }
        "set_thinking_level" => {
            let level = payload
                .get("level")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            drop(g);
            if level.trim().is_empty() {
                return RpcResult::error("invalid_input", "empty thinking level");
            }
            if let Ok(mut pending) = slot.pending_thinking.lock() {
                *pending = Some(level.clone());
            }
            RpcResult::ok_value(json!({ "thinking_level": level }))
        }
        _ => RpcResult::error(
            "unregistered_method",
            format!("unregistered method {method}"),
        ),
    }
}

fn model_data(m: &ModelInfo) -> Value {
    json!({
        "id": m.id,
        "display_name": m.display_name,
        "thinking": m.thinking,
        "thinking_levels": m.thinking_levels,
        "context_window": m.context_window,
    })
}

fn rpc_err(e: XyDriverError) -> RpcResult {
    RpcResult::error(stable_business_code(e.kind()), e.to_string())
}

fn stable_business_code(kind: &str) -> String {
    let mut code = String::with_capacity(kind.len() + 4);
    for (i, ch) in kind.char_indices() {
        if ch.is_ascii_uppercase() {
            if i > 0 {
                code.push('_');
            }
            code.push(ch.to_ascii_lowercase());
        } else {
            code.push(ch);
        }
    }
    code
}

fn occupancy_err(err: XySessionStoreError) -> RpcResult {
    match err {
        XySessionStoreError::WriteHeld { session_id } => RpcResult::error(
            "session_write_held",
            format!("session {session_id} is being written by another process"),
        ),
        other => rpc_err(XyDriverError::from(other)),
    }
}

async fn ensure_write_occupancy(
    host: &HostState,
    slot: &Arc<SessionSlot>,
    workspace: &Path,
) -> Result<(), Box<RpcResult>> {
    HostState::cancel_write_linger(slot).await;
    if let Err(e) = host.ports.store.try_acquire_write_lock(&slot.session_id) {
        return Err(Box::new(occupancy_err(e)));
    }
    materialize_writer_at(host, slot, workspace)
        .await
        .map_err(|e| Box::new(rpc_err(e)))?;
    Ok(())
}

pub fn is_tape_event(event: &XyEvent) -> bool {
    matches!(
        event,
        XyEvent::TextDelta(_)
            | XyEvent::ThinkingDelta(_)
            | XyEvent::MessageUpdate { .. }
            | XyEvent::ToolExecutionUpdate { .. }
    )
}

pub async fn prune_mux_and_maybe_release(host: &Arc<HostState>, clean: bool) {
    let slots: Vec<Arc<SessionSlot>> = host.sessions.read().await.values().cloned().collect();
    for slot in slots {
        if HostState::live_subscriber_count(&slot).await == 0 {
            on_last_subscriber_gone(host, &slot, clean).await;
        }
    }
}

async fn on_last_subscriber_gone(host: &Arc<HostState>, slot: &Arc<SessionSlot>, clean: bool) {
    if slot.turn_inflight_active() {
        return;
    }
    HostState::cancel_write_linger(slot).await;
    let linger = host.linger_duration();
    if clean || linger.is_zero() {
        release_and_evict(host, slot).await;
        return;
    }
    schedule_linger_eviction(host, slot, linger).await;
}

async fn release_and_evict(host: &Arc<HostState>, slot: &Arc<SessionSlot>) {
    let _gate = slot.eviction_gate.lock().await;
    host.ports.store.release_write_lock(&slot.session_id);
    host.evict_slot(slot).await;
}

async fn linger_evict_if_idle(host: Arc<HostState>, slot: Arc<SessionSlot>) {
    let _gate = slot.eviction_gate.lock().await;
    if slot.turn_inflight_active() {
        return;
    }
    if HostState::live_subscriber_count(&slot).await > 0 {
        return;
    }
    host.ports.store.release_write_lock(&slot.session_id);
    host.evict_slot(&slot).await;
}

async fn gated_subscribe(
    host: &Arc<HostState>,
    slot: Arc<SessionSlot>,
    session_id: &str,
    last_seq: u64,
    mux_id: Option<u64>,
    workspace_set: WorkspaceSet,
) -> RpcResult {
    let _gate = slot.eviction_gate.lock().await;
    let same = host
        .existing_slot(session_id)
        .await
        .is_some_and(|current| Arc::ptr_eq(&current, &slot));
    let slot = if same {
        slot.clone()
    } else {
        host.slot(session_id).await
    };
    HostState::cancel_write_linger(&slot).await;
    if let Err(e) = host.ports.store.try_acquire_write_lock(session_id)
        && matches!(e, XySessionStoreError::WriteHeld { .. })
    {
        return occupancy_err(e);
    }
    if let Ok(mut guard) = slot.workspaces.lock() {
        *guard = workspace_set.clone();
    }
    if let Err(e) = materialize_writer_at(host, &slot, &workspace_set.primary).await {
        log::warn!(
            target: "xylitol::host",
            "subscribe materialize_writer failed: {e}"
        );
    }
    host.bind_mux_to_session(&slot, last_seq, mux_id).await
}

async fn schedule_linger_eviction(
    host: &Arc<HostState>,
    slot: &Arc<SessionSlot>,
    linger: Duration,
) {
    let host = host.clone();
    let slot_wait = slot.clone();
    let task = tokio::spawn(async move {
        tokio::time::sleep(linger).await;
        linger_evict_if_idle(host, slot_wait).await;
    });
    *slot.linger_task.lock().await = Some(task);
}

async fn on_prompt_finished(host: &Arc<HostState>, slot: &Arc<SessionSlot>) {
    if HostState::live_subscriber_count(slot).await == 0 {
        on_last_subscriber_gone(host, slot, false).await;
    }
}

pub async fn handle_unary(
    host: &Arc<HostState>,
    rpc_id: Option<&str>,
    method: &str,
    payload: Value,
    mux_id: Option<u64>,
) -> RpcResult {
    if method == METHOD_HOST_DESCRIBE {
        return RpcResult::ok_value(describe_value());
    }
    if !is_unary_method(method) {
        return RpcResult::error(
            "unregistered_method",
            format!("unregistered method {method}"),
        );
    }

    if method == METHOD_SESSION_DETACH {
        let session_id = host.session_id_from_payload(&payload);
        if session_id.is_empty() {
            return RpcResult::error("invalid_input", "session_detach requires session_id");
        }
        let Some(slot) = host.existing_slot(&session_id).await else {
            return RpcResult::ok_value(json!({ "detached": true }));
        };
        if slot.turn_inflight_active() {
            return RpcResult::ok_value(json!({
                "detached": false,
                "reason": "session busy"
            }));
        }
        if HostState::live_subscriber_count(&slot).await > 1 {
            return RpcResult::ok_value(json!({
                "detached": false,
                "reason": "other clients attached"
            }));
        }
        host.factory.release_session(&session_id);
        return RpcResult::ok_value(json!({ "detached": true }));
    }
    if method == METHOD_RELOAD {
        return match host.reload_resources().await {
            Ok(report) => RpcResult::ok_value(serde_json::to_value(report).unwrap_or(Value::Null)),
            Err(e) => rpc_err(e),
        };
    }
    if method == METHOD_LOADED_RESOURCES {
        let session_id = host.session_id_from_payload(&payload);
        let snapshot = host.loaded_resources_snapshot_for(&session_id).await;
        return RpcResult::ok_value(serde_json::to_value(snapshot).unwrap_or(Value::Null));
    }
    if method == METHOD_ABORT {
        let abort_session = host.session_id_from_payload(&payload);
        if let Some(slot) = host.existing_slot(&abort_session).await
            && let Some(cancel) = slot.bash_run_cancel.lock().ok().and_then(|c| c.clone())
        {
            cancel.cancel();
            return RpcResult::ok_value(json!({ "cancelled": true }));
        }
        if host.abort_reload().await {
            return RpcResult::ok_value(json!({ "cancelled": true }));
        }
    }

    let session_id = if method == METHOD_SUBSCRIBE {
        payload
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    } else {
        host.session_id_from_payload(&payload)
    };
    let slot = host.slot(&session_id).await;
    let workspace = session_workspace(host, &slot, &payload);

    let reservation = match slot.idempotency.admit(rpc_id, method, &payload).await {
        Admission::Proceed(reservation) => reservation,
        Admission::Replay(result) => return result,
        Admission::Conflict => return conflict_result(),
    };

    let result = dispatch_session_unary(
        host,
        &slot,
        &session_id,
        method,
        payload,
        &workspace,
        mux_id,
    )
    .await;
    reservation.complete(result.clone());
    result
}

async fn dispatch_session_unary(
    host: &Arc<HostState>,
    slot: &Arc<SessionSlot>,
    session_id: &str,
    method: &str,
    payload: Value,
    workspace: &Path,
    mux_id: Option<u64>,
) -> RpcResult {
    if method == METHOD_SUBSCRIBE {
        if session_id.is_empty() {
            return RpcResult::error("invalid_input", "subscribe requires session_id");
        }
        let last_seq = payload
            .get("last_seq")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let workspace_set = resolve_workspace_set(host, session_id, &payload).await;
        return gated_subscribe(
            host,
            slot.clone(),
            session_id,
            last_seq,
            mux_id,
            workspace_set,
        )
        .await;
    }

    if method == METHOD_PROMPT {
        let message = payload
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let contexts = payload
            .get("contexts")
            .cloned()
            .and_then(|value| serde_json::from_value::<Vec<McpResourceContext>>(value).ok())
            .unwrap_or_default();
        if message.is_empty() {
            return RpcResult::error("invalid_input", "missing message");
        }
        if let Err(e) = ensure_write_occupancy(host, slot, workspace).await {
            return *e;
        }
        let Some(guard) = RunInflightGuard::try_acquire(slot) else {
            return RpcResult::error(
                "session_busy",
                "会话回合进行中：根提交须等待结束后重试，或以插话（steer / follow_up）改向",
            );
        };
        let driver = slot.driver.clone();
        let slot_push = slot.clone();
        let host_push = host.clone();
        let prompt_model = payload
            .get("model_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let prompt_thinking = payload
            .get("thinking_level")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        tokio::spawn(async move {
            let stream = {
                let mut g = driver.lock().await;
                let Some(d) = g.as_mut() else {
                    return;
                };
                slot_push.clear_pending_runtime();
                if let Some(id) = prompt_model.as_deref()
                    && let Err(e) = d.select_model(id).await
                {
                    log::warn!(
                        target: "xylitol::host",
                        "prompt model_id={id} select failed: {e}"
                    );
                }
                if let Some(level) = prompt_thinking.as_deref()
                    && let Err(e) = d.set_thinking_level(level.to_string()).await
                {
                    log::warn!(
                        target: "xylitol::host",
                        "prompt thinking_level select failed: {e}"
                    );
                }
                d.run_with_context(&message, contexts).await
            };
            let mut stream = stream;
            while let Some(event) = stream.next().await {
                slot_push.append_and_push(&event).await;
            }
            drop(guard);
            slot_push.flush_pending_runtime().await;
            on_prompt_finished(&host_push, &slot_push).await;
        });
        return RpcResult::ok_value(json!({ "session_id": session_id }));
    }

    if method == METHOD_ARM_TOOL_FREEZE {
        if let Err(e) = materialize_writer_at(host, slot, workspace).await {
            log::warn!(
                target: "xylitol::host",
                "arm_tool_freeze materialize_writer failed: {e}"
            );
        }
        let mut g = slot.driver.lock().await;
        let Some(driver) = g.as_mut() else {
            return RpcResult::error("unavailable", "no writer engine");
        };
        driver.ensure_tool_table_frozen().await;
        let snapshot = driver.loaded_resources_snapshot().await;
        return RpcResult::ok_value(serde_json::to_value(&snapshot).unwrap_or(Value::Null));
    }

    if method == METHOD_PERSIST_TRUST {
        if let Err(e) = ensure_write_occupancy(host, slot, workspace).await {
            return *e;
        }
        let mode = match payload.get("mode").and_then(Value::as_str) {
            Some("trust_cwd") | None => ProjectTrustMode::TrustCwd,
            Some("trust_parent") => ProjectTrustMode::TrustParent,
            Some("deny") => ProjectTrustMode::Deny,
            Some(other) => {
                return RpcResult::error("invalid_input", format!("unknown trust mode: {other}"));
            }
        };
        let mut g = slot.driver.lock().await;
        let Some(driver) = g.as_mut() else {
            return RpcResult::error("unavailable", "no writer engine");
        };
        let report = match driver.persist_project_trust(mode).await {
            Ok(report) => report,
            Err(e) => return rpc_err(e),
        };
        return RpcResult::ok_value(json!({
            "trusted": report.trusted,
            "saved_path": report.saved_path,
            "message": report.message,
        }));
    }

    if method == METHOD_MCP_CATALOG {
        let mut g = slot.driver.lock().await;
        let Some(driver) = g.as_mut() else {
            return RpcResult::error("unavailable", "no writer engine");
        };
        return match driver.mcp_catalog().await {
            Ok(catalog) => {
                RpcResult::ok_value(serde_json::to_value(catalog).unwrap_or(Value::Null))
            }
            Err(e) => rpc_err(e),
        };
    }

    if method == METHOD_MCP_RESOURCE_READ {
        let server = payload.get("server").and_then(Value::as_str).unwrap_or("");
        let uri = payload.get("uri").and_then(Value::as_str).unwrap_or("");
        let mut g = slot.driver.lock().await;
        let Some(driver) = g.as_mut() else {
            return RpcResult::error("unavailable", "no writer engine");
        };
        return match driver.mcp_resource_read(server, uri).await {
            Ok(content) => {
                RpcResult::ok_value(serde_json::to_value(content).unwrap_or(Value::Null))
            }
            Err(e) => rpc_err(e),
        };
    }

    if method == METHOD_MCP_PROMPT_GET {
        let server = payload.get("server").and_then(Value::as_str).unwrap_or("");
        let name = payload.get("name").and_then(Value::as_str).unwrap_or("");
        let arguments = payload.get("arguments").cloned().unwrap_or(Value::Null);
        let mut g = slot.driver.lock().await;
        let Some(driver) = g.as_mut() else {
            return RpcResult::error("unavailable", "no writer engine");
        };
        return match driver.mcp_prompt_get(server, name, arguments).await {
            Ok(value) => RpcResult::ok_value(value),
            Err(e) => rpc_err(e),
        };
    }

    if method == METHOD_MCP_OAUTH_START {
        let server = payload.get("server").and_then(Value::as_str).unwrap_or("");
        let mut g = slot.driver.lock().await;
        let Some(driver) = g.as_mut() else {
            return RpcResult::error("unavailable", "no writer engine");
        };
        return match driver.mcp_oauth_start(server).await {
            Ok(flow) => RpcResult::ok_value(serde_json::to_value(flow).unwrap_or(Value::Null)),
            Err(e) => rpc_err(e),
        };
    }

    if method == METHOD_MCP_OAUTH_CANCEL {
        let flow_id = payload.get("flow_id").and_then(Value::as_str).unwrap_or("");
        let mut g = slot.driver.lock().await;
        let Some(driver) = g.as_mut() else {
            return RpcResult::error("unavailable", "no writer engine");
        };
        return match driver.mcp_oauth_cancel(flow_id).await {
            Ok(()) => RpcResult::ok_value(Value::Null),
            Err(e) => rpc_err(e),
        };
    }

    if method == METHOD_MCP_OAUTH_STATUS {
        let flow_id = payload.get("flow_id").and_then(Value::as_str).unwrap_or("");
        let mut g = slot.driver.lock().await;
        let Some(driver) = g.as_mut() else {
            return RpcResult::error("unavailable", "no writer engine");
        };
        return match driver.mcp_oauth_status(flow_id).await {
            Ok(status) => RpcResult::ok_value(serde_json::to_value(status).unwrap_or(Value::Null)),
            Err(e) => rpc_err(e),
        };
    }

    if is_writer_method(method) {
        return dispatch_writer_unary(host, slot, method, payload, workspace).await;
    }

    dispatch_readonly_unary(host, slot, session_id, method, payload, workspace).await
}

async fn dispatch_writer_unary(
    host: &Arc<HostState>,
    slot: &Arc<SessionSlot>,
    method: &str,
    payload: Value,
    workspace: &Path,
) -> RpcResult {
    if let Err(e) = ensure_write_occupancy(host, slot, workspace).await {
        return *e;
    }
    if slot.turn_inflight_active()
        && matches!(method, "set_model" | "cycle_model" | "set_thinking_level")
    {
        return defer_runtime_setting(slot, method, &payload).await;
    }
    let (payload, staged_export, import_temp) = match stage_payload(method, payload) {
        Ok(staged) => staged,
        Err(err) => return *err,
    };
    let cmd = match parse_command(method, &payload) {
        Ok(cmd) => cmd,
        Err(e) => return RpcResult::error("invalid_input", e),
    };
    let mut g = slot.driver.lock().await;
    let Some(driver) = g.as_mut() else {
        return RpcResult::error("unavailable", "no writer engine");
    };
    let bash_forwarder = start_bash_forwarder(slot, driver.as_mut(), method).await;
    let result = match driver.execute_command(cmd).await {
        Ok(mut value) => {
            inline_staged_export(staged_export, &mut value);
            RpcResult::ok_value(value)
        }
        Err(e) => rpc_err(e),
    };
    if bash_forwarder.is_some() {
        finish_bash_forwarder(driver.as_mut(), slot);
    }
    drop(g);
    if let Some(fwd) = bash_forwarder {
        let _ = fwd.await;
    }
    if let Some(temp) = import_temp {
        let _ = std::fs::remove_file(temp);
    }
    result
}

fn stage_payload(
    method: &str,
    mut payload: Value,
) -> Result<(Value, bool, Option<std::path::PathBuf>), Box<RpcResult>> {
    let staged_export = matches!(method, "export_html" | "export_jsonl")
        && payload.get("output_path").is_none_or(Value::is_null);
    let staged_import = method == METHOD_IMPORT_JSONL
        && payload.get("input_path").is_none()
        && payload.get("content").is_some();
    let import_temp = if staged_import {
        let temp =
            std::env::temp_dir().join(format!("xylitol-import-{}.jsonl", uuid::Uuid::new_v4()));
        if let Err(e) = std::fs::write(&temp, payload["content"].as_str().unwrap_or_default()) {
            return Err(Box::new(RpcResult::error(
                "internal_error",
                format!("stage import write: {e}"),
            )));
        }
        Some(temp)
    } else {
        None
    };
    if staged_export {
        let ext = if method == METHOD_EXPORT_JSONL {
            "jsonl"
        } else {
            "html"
        };
        payload["output_path"] = json!(
            std::env::temp_dir().join(format!("xylitol-export-{}.{ext}", uuid::Uuid::new_v4()))
        );
    }
    if let Some(temp) = &import_temp {
        payload["input_path"] = json!(temp);
    }
    Ok((payload, staged_export, import_temp))
}

async fn start_bash_forwarder(
    slot: &Arc<SessionSlot>,
    driver: &mut dyn OccupancyEngine,
    method: &str,
) -> Option<tokio::task::JoinHandle<()>> {
    if method != "bash" {
        return None;
    }
    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    let cancel = CancellationToken::new();
    if let Ok(mut slot_cancel) = slot.bash_run_cancel.lock() {
        *slot_cancel = Some(cancel.clone());
    }
    let slot_push = slot.clone();
    let fwd = tokio::spawn(async move {
        while let Some(chunk) = rx.recv().await {
            slot_push.push_bash_output(chunk).await;
        }
    });
    driver.set_bash_run_sink(Some(xylitol_plugin_sdk::BashOutputSink { tx, cancel }));
    Some(fwd)
}

fn finish_bash_forwarder(driver: &mut dyn OccupancyEngine, slot: &SessionSlot) {
    driver.set_bash_run_sink(None);
    if let Ok(mut slot_cancel) = slot.bash_run_cancel.lock() {
        *slot_cancel = None;
    }
}

fn inline_staged_export(staged_export: bool, value: &mut Value) {
    if !staged_export {
        return;
    }
    let Some(path) = value
        .get("path")
        .and_then(Value::as_str)
        .map(std::path::PathBuf::from)
    else {
        return;
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            value["content"] = Value::String(content);
        }
        Err(e) => log::warn!(
            target: "xylitol::host",
            "export stage read {}: {e}",
            path.display()
        ),
    }
    let _ = std::fs::remove_file(&path);
}

async fn dispatch_readonly_unary(
    host: &Arc<HostState>,
    slot: &Arc<SessionSlot>,
    session_id: &str,
    method: &str,
    payload: Value,
    workspace: &Path,
) -> RpcResult {
    if method == METHOD_GET_AVAILABLE_MODELS && slot.driver.lock().await.is_none() {
        let models: Vec<Value> = host
            .ports
            .model_registry
            .list()
            .iter()
            .map(|m| {
                json!({
                    "id": m.id,
                    "display_name": m.display_name,
                    "thinking": m.thinking,
                    "thinking_levels": m.thinking_levels,
                    "context_window": m.context_window,
                })
            })
            .collect();
        return RpcResult::ok_value(json!({ "models": models }));
    }

    if !is_writer_method(method) && slot.driver.lock().await.is_none() {
        if matches!(method, "list_sessions" | "get_commands") {
            let mut reader = new_reader_driver(host);
            let cmd = match parse_command(method, &payload) {
                Ok(c) => c,
                Err(e) => return RpcResult::error("invalid_input", e),
            };
            return match reader.execute_command(cmd).await {
                Ok(outcome) => RpcResult::ok_value(outcome),
                Err(e) => rpc_err(e),
            };
        }

        let session_exists = host.ports.store.exists(session_id).await;
        if !session_exists && method == METHOD_SESSION_TREE {
            let cwd = workspace.to_string_lossy().into_owned();
            if let Err(e) = host.ports.store.create(session_id, Some(&cwd), None).await {
                return rpc_err(e.into());
            }
        } else if !session_exists {
            return match method {
                "get_state" => RpcResult::ok_value(json!({
                    "session_id": session_id,
                    "model": Value::Null,
                    "thinking_level": Value::Null,
                    "leaf_entry_id": host.ports.store.leaf_id(session_id),
                })),
                "get_messages" => RpcResult::ok_value(json!({ "entries": [], "has_more": false })),
                "get_session_stats" => RpcResult::ok_value(json!({
                    "session_id": session_id,
                    "user_messages": 0,
                    "assistant_messages": 0,
                    "total_messages": 0,
                })),
                "queue_stats" => RpcResult::ok_value(json!({
                    "steer_count": 0,
                    "follow_up_count": 0,
                })),
                _ => RpcResult::error("not_found", format!("session not found: {session_id}")),
            };
        }

        let mut reader = match materialize_reader(host, session_id).await {
            Ok(driver) => driver,
            Err(e) => return rpc_err(e),
        };
        let cmd = match parse_command(method, &payload) {
            Ok(c) => c,
            Err(e) => return RpcResult::error("invalid_input", e),
        };
        return match reader.execute_command(cmd).await {
            Ok(outcome) => RpcResult::ok_value(outcome),
            Err(e) => rpc_err(e),
        };
    }

    let cmd = match parse_command(method, &payload) {
        Ok(c) => c,
        Err(e) => return RpcResult::error("invalid_input", e),
    };
    let mut g = slot.driver.lock().await;
    let Some(driver) = g.as_mut() else {
        return RpcResult::error("unavailable", "no writer engine");
    };
    match driver.execute_command(cmd).await {
        Ok(outcome) => RpcResult::ok_value(outcome),
        Err(e) => rpc_err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use xylitol_plugin_sdk::BashChunk;

    fn test_slot(id: &str) -> Arc<SessionSlot> {
        let (tx, _rx) = broadcast::channel(8);
        SessionSlot::with_seq(id, tx, None, 0)
    }

    #[tokio::test]
    async fn inflight_guard_drop_resets_slot() {
        let slot = test_slot("guard-drop");
        let guard = RunInflightGuard::try_acquire(&slot).expect("acquire");
        assert!(slot.turn_inflight_active());
        assert!(RunInflightGuard::try_acquire(&slot).is_none());
        drop(guard);
        assert!(!slot.turn_inflight_active());
        assert!(RunInflightGuard::try_acquire(&slot).is_some());
    }

    #[tokio::test]
    async fn inflight_guard_unwinds_on_panic() {
        let slot = test_slot("guard-panic");
        let guard = RunInflightGuard::try_acquire(&slot).expect("acquire");
        let handle = tokio::spawn(async move {
            let _guard = guard;
            panic!("run task panicked");
        });
        assert!(handle.await.is_err(), "task must unwind");
        assert!(
            !slot.turn_inflight_active(),
            "panic must release the in-flight slot"
        );
    }

    #[tokio::test]
    async fn inflight_guard_releases_on_early_drop() {
        let slot = test_slot("guard-early");
        let guard = RunInflightGuard::try_acquire(&slot).expect("acquire");
        drop(guard);
        assert!(!slot.turn_inflight_active());
    }

    #[test]
    fn elicitation_questions_and_response_round_trip() {
        let schema = json!({
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": {"type": "string", "description": "Name"},
                "count": {"type": "integer"},
                "enabled": {"type": "boolean"}
            }
        });
        let questions = elicitation_questions(&schema, "Fill this");
        assert_eq!(questions[0].id, "action");
        assert!(questions.iter().any(|q| q.id == "name"));
        assert!(
            questions
                .iter()
                .find(|q| q.id == "enabled")
                .is_some_and(|q| q.options.len() == 2)
        );

        let response = parse_elicitation_response(
            &json!({
                "status": "answered",
                "answers": [
                    {"id": "action", "values": ["accept"]},
                    {"id": "name", "values": ["Ada"]},
                    {"id": "count", "values": ["3"]},
                    {"id": "enabled", "values": ["true"]}
                ]
            })
            .to_string(),
            &schema,
        );
        assert_eq!(response.action, McpElicitationAction::Accept);
        let content = response.content.expect("content");
        assert_eq!(content["name"], json!("Ada"));
        assert_eq!(content["count"], json!(3));
        assert_eq!(content["enabled"], json!(true));

        let declined = parse_elicitation_response(
            &json!({"status": "skipped", "answers": []}).to_string(),
            &schema,
        );
        assert_eq!(declined.action, McpElicitationAction::Decline);
        assert!(declined.content.is_none());
    }

    #[test]
    fn parse_workspaces_prefers_list_and_falls_back_to_cwd() {
        let (primary, additional) = parse_workspaces(&json!({
            "workspaces": ["/a/project", "/a/other"],
            "cwd": "/legacy"
        }));
        assert_eq!(primary, Some(PathBuf::from("/a/project")));
        assert_eq!(additional, vec![PathBuf::from("/a/other")]);

        let (primary, additional) = parse_workspaces(&json!({"cwd": "/legacy"}));
        assert_eq!(primary, Some(PathBuf::from("/legacy")));
        assert!(additional.is_empty());

        let (primary, additional) = parse_workspaces(&json!({}));
        assert!(primary.is_none());
        assert!(additional.is_empty());
    }

    #[test]
    fn stage_export_treats_null_output_path_as_absent() {
        let (payload, staged, _) =
            stage_payload("export_html", json!({"output_path": null})).expect("stage");
        assert!(staged);
        assert!(
            payload["output_path"]
                .as_str()
                .is_some_and(|p| p.contains("xylitol-export-")),
            "staged export must get a temp path, got {payload}"
        );
    }

    #[test]
    fn stage_export_keeps_explicit_output_path() {
        let (payload, staged, _) =
            stage_payload("export_html", json!({"output_path": "a.html"})).expect("stage");
        assert!(!staged);
        assert_eq!(payload["output_path"], json!("a.html"));
    }

    #[test]
    fn stage_export_treats_missing_output_path_as_absent() {
        let (_, staged, _) = stage_payload("export_jsonl", json!({})).expect("stage");
        assert!(staged);
    }

    #[test]
    fn stable_business_code_is_snake_case_of_domain_kind() {
        assert_eq!(stable_business_code("NotFound"), "not_found");
        assert_eq!(stable_business_code("NoSession"), "no_session");
        assert_eq!(stable_business_code("Busy"), "busy");
    }

    fn slot() -> Arc<SessionSlot> {
        SessionSlot::new("s0", broadcast::channel(16).0, None)
    }

    struct Marker(&'static str);

    #[async_trait::async_trait]
    impl xylitol_plugin_sdk::LoopMiddleware for Marker {
        fn id(&self) -> &str {
            self.0
        }

        fn stages(&self) -> &'static [xylitol_plugin_sdk::LoopStage] {
            &[xylitol_plugin_sdk::LoopStage::BeforeTool]
        }

        async fn before_tool(
            &self,
            ctx: xylitol_plugin_sdk::BeforeToolCtx,
        ) -> xylitol_plugin_sdk::LoopStep<xylitol_plugin_sdk::BeforeToolCtx> {
            xylitol_plugin_sdk::LoopStep::continued(ctx)
        }
    }

    #[test]
    fn session_slots_derive_independent_loop_registries() {
        let root = Arc::new(xylitol_plugin_sdk::LoopMiddlewareRegistry::new());
        root.register(Arc::new(Marker("shared"))).expect("shared");

        let slot_a = SessionSlot::with_seq("a", broadcast::channel(16).0, Some(root.clone()), 1);
        let slot_b = SessionSlot::with_seq("b", broadcast::channel(16).0, Some(root.clone()), 1);

        root.register(Arc::new(Marker("late"))).expect("late");

        let ids = |slot: &SessionSlot| slot.loop_registry.as_ref().expect("session registry").ids();
        assert_eq!(ids(&slot_a), vec!["shared".to_string()]);
        assert_eq!(ids(&slot_b), vec!["shared".to_string()]);
        assert_eq!(root.ids(), vec!["shared".to_string(), "late".to_string()]);

        slot_a
            .loop_registry
            .as_ref()
            .expect("session registry")
            .register(Arc::new(Marker("only-a")))
            .expect("only-a");

        assert_eq!(
            ids(&slot_a),
            vec!["shared".to_string(), "only-a".to_string()]
        );
        assert_eq!(ids(&slot_b), vec!["shared".to_string()]);
        assert_eq!(root.ids(), vec!["shared".to_string(), "late".to_string()]);
    }

    fn text_delta(text: &str) -> XyEvent {
        XyEvent::TextDelta(text.into())
    }

    fn step_start(index: u32) -> XyEvent {
        XyEvent::StepStart { step_index: index }
    }

    async fn subscribe(slot: &SessionSlot) -> (u64, mpsc::Receiver<Notification>) {
        let (tx, rx) = mpsc::channel(32);
        let id = slot.add_subscriber(tx).await;
        (id, rx)
    }

    #[tokio::test]
    async fn broadcast_evicts_subscriber_when_resync_also_cannot_fit() {
        let slot = slot();
        let (tx, mut rx) = mpsc::channel(1);
        let id = slot.add_subscriber(tx).await;

        slot.broadcast(Notification {
            method: "fill".into(),
            params: json!({}),
        })
        .await;
        slot.broadcast(Notification {
            method: "overflow".into(),
            params: json!({}),
        })
        .await;

        assert!(
            !slot.subscribers.lock().await.contains_key(&id),
            "a subscriber that cannot take the resync must be evicted"
        );
        assert_eq!(rx.recv().await.expect("first notification").method, "fill");
        assert!(
            rx.try_recv().is_err(),
            "the full channel never received the resync"
        );
    }

    #[tokio::test]
    async fn broadcast_evicts_closed_subscriber() {
        let slot = slot();
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let id = slot.add_subscriber(tx).await;

        slot.broadcast(Notification {
            method: "closed".into(),
            params: json!({}),
        })
        .await;

        assert!(
            !slot.subscribers.lock().await.contains_key(&id),
            "a closed subscriber must be removed"
        );
    }

    #[tokio::test]
    async fn append_and_push_wraps_the_domain_event_as_session_event() {
        let slot = slot();
        let (_, mut rx) = subscribe(&slot).await;
        slot.append_and_push(&step_start(1)).await;
        let note = rx.recv().await.expect("notification");
        assert_eq!(note.method, SESSION_EVENT);
        assert_eq!(note.params["session_id"], "s0");
        assert_eq!(note.params["seq"], 1);
        assert_eq!(
            note.params["event"],
            json!({"type": "step_start", "step_index": 1})
        );
    }

    #[tokio::test]
    async fn tape_events_never_touch_the_journal() {
        let slot = slot();
        let (_, mut rx) = subscribe(&slot).await;
        slot.append_and_push(&text_delta("hi")).await;
        slot.append_and_push(&XyEvent::ThinkingDelta("think".into()))
            .await;
        slot.append_and_push(&XyEvent::ToolExecutionUpdate {
            id: "t1".into(),
            output: "chunk".into(),
        })
        .await;
        assert_eq!(
            slot.journal.lock().await.current_seq(),
            1,
            "tape must not consume any seq"
        );
        assert!(slot.journal.lock().await.max_seq() == 0);
        for _ in 0..3 {
            let note = rx.recv().await.expect("tape notification");
            assert_eq!(note.method, SESSION_EVENT);
            assert_eq!(note.params["seq"], 0, "tape rides the current fact seq");
        }
    }

    #[tokio::test]
    async fn tape_does_not_accumulate_in_journal_between_facts() {
        let slot = slot();
        let (tx, mut rx) = mpsc::channel(128);
        slot.add_subscriber(tx).await;
        slot.append_and_push(&step_start(1)).await;
        for i in 0..50 {
            slot.append_and_push(&text_delta(&format!("d{i}"))).await;
        }
        slot.append_and_push(&XyEvent::StepEnd { step_index: 1 })
            .await;
        let journal = slot.journal.lock().await;
        assert_eq!(journal.max_seq(), 2, "only the two facts are journaled");
        drop(journal);

        let mut tape_seqs = Vec::new();
        while let Ok(note) = rx.try_recv() {
            let Some(event) = note.params.get("event") else {
                continue;
            };
            if matches!(
                serde_json::from_value::<xylitol_protocol::Event>(event.clone()),
                Ok(xylitol_protocol::Event::TextDelta { .. })
            ) {
                tape_seqs.push(note.params["seq"].as_u64().unwrap_or(u64::MAX));
            }
        }
        assert_eq!(tape_seqs.len(), 50);
        assert!(
            tape_seqs.iter().all(|seq| *seq == 1),
            "tape frames ride the current fact position and share its seq"
        );
    }

    #[tokio::test]
    async fn non_event_downlinks_do_not_consume_journal_seq() {
        let slot = slot();
        slot.append_and_push(&step_start(1)).await;
        let _rx = slot.request_approval("c1".into()).await;
        assert_eq!(slot.journal.lock().await.current_seq(), 2);
        slot.push_bash_output(BashChunk {
            bash_id: "b1".into(),
            seq: 1,
            data: "out".into(),
        })
        .await;
        assert_eq!(slot.journal.lock().await.current_seq(), 2);
        slot.push_resources(LoadedResourcesSnapshot::default())
            .await;
        assert_eq!(slot.journal.lock().await.current_seq(), 2);
        let _answer = slot
            .request_question("q1".into(), json!([{"question": "?"}]))
            .await;
        assert_eq!(slot.journal.lock().await.current_seq(), 2);
    }

    #[tokio::test]
    async fn bash_output_is_its_own_method_with_bash_payload() {
        let slot = slot();
        let (_, mut rx) = subscribe(&slot).await;
        slot.push_bash_output(BashChunk {
            bash_id: "b1".into(),
            seq: 4,
            data: "out".into(),
        })
        .await;
        let note = rx.recv().await.expect("notification");
        assert_eq!(note.method, "bash/output");
        assert_eq!(
            note.params,
            json!({"session_id": "s0", "bash_id": "b1", "seq": 4, "data": "out"})
        );
    }

    #[tokio::test]
    async fn approval_request_registers_the_gateway_and_announces_call_id() {
        let slot = slot();
        let (_, mut rx) = subscribe(&slot).await;
        let answer = slot.request_approval("c1".into()).await;
        let note = rx.recv().await.expect("notification");
        assert_eq!(note.method, APPROVAL_REQUESTED);
        assert_eq!(note.params, json!({"call_id": "c1"}));
        assert!(
            slot.gateway
                .handle_respond("c1", &json!({"approved": true}))
        );
        assert_eq!(answer.await, Ok(ReverseRpcResult::Approved));
    }

    #[tokio::test]
    async fn question_request_carries_call_id_and_questions() {
        let slot = slot();
        let (_, mut rx) = subscribe(&slot).await;
        let _answer = slot
            .request_question("q1".into(), json!([{"question": "?"}]))
            .await;
        let note = rx.recv().await.expect("notification");
        assert_eq!(note.method, QUESTION_REQUESTED);
        assert_eq!(
            note.params,
            json!({"call_id": "q1", "questions": [{"question": "?"}]})
        );
        assert_eq!(slot.gateway.pending_count(), 1);
    }

    #[tokio::test]
    async fn subscribe_continuation_replays_only_newer_events_then_subscribed() {
        let slot = slot();
        for i in 1..=3 {
            slot.append_and_push(&step_start(i)).await;
        }
        let (id, mut rx) = subscribe(&slot).await;
        let result = slot.replay_or_resync(&[id], false, false, 1).await;
        let first = rx.recv().await.expect("event 2");
        let second = rx.recv().await.expect("event 3");
        let subscribed = rx.recv().await.expect("subscribed");
        assert_eq!(first.method, SESSION_EVENT);
        assert_eq!(first.params["seq"], 2);
        assert_eq!(second.params["seq"], 3);
        assert_eq!(subscribed.method, SESSION_SUBSCRIBED);
        let expected = json!({"session_id": "s0", "seq": 3, "inflight": false});
        assert_eq!(subscribed.params, expected);
        assert_eq!(result, RpcResult::ok_value(expected));
    }

    #[tokio::test]
    async fn subscribe_continuation_reports_inflight_in_result_and_notification() {
        let slot = slot();
        slot.append_and_push(&step_start(1)).await;
        let (id, mut rx) = subscribe(&slot).await;
        let result = slot.replay_or_resync(&[id], true, true, 0).await;
        let subscribed = rx.recv().await.expect("subscribed");
        let expected = json!({"session_id": "s0", "seq": 1, "inflight": true});
        assert_eq!(subscribed.params, expected);
        assert_eq!(result, RpcResult::ok_value(expected));
    }

    #[tokio::test]
    async fn wrapped_journal_offers_resync_and_reports_seq_zero() {
        let slot = slot();
        *slot.journal.lock().await = EventJournal::new("s0", 2);
        for i in 1..=3 {
            slot.append_and_push(&step_start(i)).await;
        }
        let (id, mut rx) = subscribe(&slot).await;
        let result = slot.replay_or_resync(&[id], false, false, 0).await;
        let resync = rx.recv().await.expect("resync");
        assert_eq!(resync.method, SESSION_RESYNC_REQUIRED);
        assert_eq!(resync.params, json!({"session_id": "s0"}));
        assert_eq!(
            result,
            RpcResult::ok_value(json!({"session_id": "s0", "seq": 0, "inflight": false}))
        );
    }

    #[test]
    fn observers_only_pass_agent_end_session_events() {
        let event = |wire| {
            downlink_notification(
                SESSION_EVENT,
                SessionEventPayload {
                    session_id: "s0".into(),
                    seq: 1,
                    event: wire,
                },
            )
        };
        assert!(SessionSlot::observe_passes(&event(
            xylitol_protocol::Event::AgentEnd { reason: None }
        )));
        assert!(!SessionSlot::observe_passes(&event(
            xylitol_protocol::Event::TextDelta { text: "x".into() }
        )));
        let approval = downlink_notification(
            APPROVAL_REQUESTED,
            ApprovalRequestedPayload {
                call_id: "c".into(),
            },
        );
        assert!(SessionSlot::observe_passes(&approval));
    }

    #[tokio::test]
    async fn evict_records_watermark_and_clears_driver() {
        let slot = SessionSlot::with_seq("s0", broadcast::channel(16).0, None, 1);
        for _ in 0..5 {
            slot.journal
                .lock()
                .await
                .append(xylitol_protocol::Event::TextDelta { text: "x".into() });
        }
        assert_eq!(slot.journal.lock().await.next_seq(), 6);

        let watermarks: std::sync::Mutex<HashMap<String, u64>> =
            std::sync::Mutex::new(HashMap::new());
        watermarks
            .lock()
            .unwrap()
            .insert("s0".to_string(), slot.journal.lock().await.next_seq());

        let rebuilt = SessionSlot::with_seq(
            "s0",
            broadcast::channel(16).0,
            None,
            *watermarks.lock().unwrap().get("s0").unwrap(),
        );
        assert_eq!(
            rebuilt.journal.lock().await.next_seq(),
            6,
            "a rebuilt slot must resume the sequence, never restart at 1"
        );
        let seq = rebuilt
            .journal
            .lock()
            .await
            .append(xylitol_protocol::Event::TextDelta { text: "y".into() });
        assert_eq!(seq, 6, "the next event continues above the watermark");
    }

    fn partial_update(text: &str) -> XyEvent {
        XyEvent::MessageUpdate {
            message: Some(xylitol_plugin_sdk::message::AgentMessage::Llm(
                xylitol_plugin_sdk::message::XyLlmMessage::user(text),
            )),
        }
    }

    #[tokio::test]
    async fn latest_tape_keeps_only_the_newest_snapshot() {
        let slot = slot();
        slot.append_and_push(&partial_update("a")).await;
        slot.append_and_push(&partial_update("ab")).await;
        slot.append_and_push(&partial_update("abc")).await;
        let held = slot.latest_tape.lock().unwrap().clone().expect("tape held");
        assert_eq!(held.text(), "abc", "only the newest snapshot is kept");
    }

    #[tokio::test]
    async fn agent_end_clears_latest_tape() {
        let slot = slot();
        slot.append_and_push(&partial_update("abc")).await;
        assert!(slot.latest_tape.lock().unwrap().is_some());
        slot.append_and_push(&XyEvent::AgentEnd {
            reason: None,
            messages: vec![],
        })
        .await;
        assert!(
            slot.latest_tape.lock().unwrap().is_none(),
            "a finished turn leaves no partial behind"
        );
    }

    #[tokio::test]
    async fn midturn_reconnect_carries_partial_only_for_positive_last_seq() {
        let slot = slot();
        slot.append_and_push(&step_start(1)).await;
        slot.append_and_push(&partial_update("half done")).await;
        let (id, _rx) = subscribe(&slot).await;

        let with_partial = slot.replay_or_resync(&[id], false, true, 1).await;
        let value = match with_partial {
            RpcResult {
                ok: true,
                value: Some(v),
                ..
            } => v,
            other => panic!("unexpected: {other:?}"),
        };
        assert_eq!(value["inflight"], true);
        assert_eq!(
            value["partial"]["content"][0]["text"], "half done",
            "a mid-turn reconnect must see the current snapshot"
        );

        let cold = slot.replay_or_resync(&[id], false, true, 0).await;
        let cold_value = match cold {
            RpcResult {
                ok: true,
                value: Some(v),
                ..
            } => v,
            other => panic!("unexpected: {other:?}"),
        };
        assert!(
            cold_value.get("partial").is_none(),
            "a cold subscribe gets the full transcript elsewhere, not a partial"
        );
    }

    use std::sync::Mutex as StdMutex;

    use async_trait::async_trait;
    use xylitol_plugin_sdk::XySessionStore;
    use xylitol_plugin_sdk::error::{XySessionError, XySessionStoreError};
    use xylitol_plugin_sdk::session::{
        CUSTOM_TYPE_AUTOMATION, CustomEntry, EntryBase, ForkPosition, SessionContext, SessionEntry,
    };
    use xylitol_protocol::Command;
    use xylitol_runtime::XyDriver;
    use xylitol_runtime::driver::{CommandInfo, EventStream};

    struct MemStore {
        entries: StdMutex<Vec<SessionEntry>>,
    }

    impl MemStore {
        fn new() -> Self {
            Self {
                entries: StdMutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl XySessionStore for MemStore {
        async fn exists(&self, _session_id: &str) -> bool {
            true
        }

        async fn load_entries(
            &self,
            _session_id: &str,
        ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
            Ok(self.entries.lock().map(|e| e.clone()).unwrap_or_default())
        }

        async fn append_session_entry(
            &self,
            _session_id: &str,
            entry: &SessionEntry,
        ) -> Result<(), XySessionStoreError> {
            self.entries
                .lock()
                .map_err(|_| XySessionStoreError::io("lock", std::io::Error::other("poison")))?
                .push(entry.clone());
            Ok(())
        }

        async fn build_session_context(
            &self,
            _session_id: &str,
        ) -> Result<SessionContext, XySessionStoreError> {
            Ok(SessionContext {
                messages: Vec::new(),
                thinking_level: String::new(),
                model: None,
            })
        }

        async fn create(
            &self,
            _id: &str,
            _cwd: Option<&str>,
            _parent: Option<&str>,
        ) -> Result<(), XySessionStoreError> {
            Ok(())
        }

        async fn fork(
            &self,
            _parent_id: &str,
            _child_id: &str,
            _at_entry_id: &str,
            _position: ForkPosition,
        ) -> Result<(), XySessionError> {
            Err(XySessionError::busy("fork unsupported"))
        }

        fn set_leaf(&self, _session_id: &str, _entry_id: Option<&str>) {}

        fn leaf_id(&self, _session_id: &str) -> Option<String> {
            None
        }
    }

    fn model_info() -> ModelInfo {
        ModelInfo {
            id: "test".into(),
            display_name: "Test".into(),
            thinking: false,
            thinking_levels: vec!["off".into()],
            context_window: 1024,
        }
    }

    struct ScriptedEngine {
        runs: Arc<StdMutex<Vec<String>>>,
    }

    #[async_trait]
    impl XyDriver for ScriptedEngine {
        async fn run(&mut self, prompt: &str) -> EventStream {
            self.runs
                .lock()
                .map(|mut runs| runs.push(prompt.to_string()))
                .ok();
            Box::pin(futures::stream::iter(vec![XyEvent::AgentEnd {
                reason: None,
                messages: Vec::new(),
            }]))
        }

        fn abort(&self) {}

        fn current_model(&self) -> Option<ModelInfo> {
            Some(model_info())
        }

        fn available_models(&self) -> Vec<ModelInfo> {
            vec![model_info()]
        }

        fn thinking_level(&self) -> String {
            "off".into()
        }

        fn session_id(&self) -> Option<String> {
            Some("s0".into())
        }

        async fn estimate_context_tokens(
            &self,
        ) -> Result<xylitol_plugin_sdk::model::ContextTokenEstimate, XyDriverError> {
            Ok(Default::default())
        }

        fn get_commands(&self) -> Vec<CommandInfo> {
            Vec::new()
        }

        fn leaf_entry_id(&self) -> Option<String> {
            None
        }

        async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
            LoadedResourcesSnapshot::default()
        }
    }

    #[async_trait]
    impl OccupancyEngine for ScriptedEngine {
        fn install_ask_tool(&mut self, _gateway: Arc<dyn xylitol_plugin_sdk::ask::AskUserGateway>) {
        }

        fn bind_session(&mut self, _session_id: &str) -> Result<(), XyDriverError> {
            Ok(())
        }

        async fn restore_model(&mut self, _model_id: &str) -> Result<(), XyDriverError> {
            Ok(())
        }

        async fn switch_session(&mut self, _session_id: &str) -> Result<String, XyDriverError> {
            Ok("s0".into())
        }

        async fn select_model(&mut self, _model_id: &str) -> Result<ModelInfo, XyDriverError> {
            Ok(model_info())
        }

        fn model_provider(&self, _model_id: &str) -> Option<String> {
            None
        }

        async fn set_thinking_level(&mut self, _level: String) -> Result<(), XyDriverError> {
            Ok(())
        }

        async fn ensure_tool_table_frozen(&mut self) {}

        async fn execute_command(&mut self, _cmd: Command) -> Result<Value, XyDriverError> {
            Ok(Value::Null)
        }

        fn agent_cwd_for_test(&self) -> String {
            ".".into()
        }

        fn set_loop_middleware_scoped(
            &mut self,
            _registry: Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>>,
            _scope: xylitol_plugin_sdk::LoopScope,
        ) {
        }
    }

    fn schedule_entry(created_at: i64, seconds: u64) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_AUTOMATION.into(),
            data: json!({
                "kind": "schedule_defined",
                "schedule_id": "sched",
                "branch_id": "s0",
                "prompt": "poll",
                "cadence": {"type": "interval", "seconds": seconds},
                "max_ticks": 2,
                "created_at": created_at
            }),
        })
    }

    struct GateSink;

    #[async_trait::async_trait]
    impl xylitol_plugin_sdk::XyEventSink for GateSink {
        async fn emit(&self, _event: &XyEvent) {}
    }

    struct GateNoopModel;

    #[async_trait::async_trait]
    impl xylitol_plugin_sdk::XyModel for GateNoopModel {
        fn name(&self) -> &str {
            "gate-noop"
        }

        async fn generate_stream(
            &self,
            _messages: Vec<xylitol_plugin_sdk::message::XyLlmMessage>,
            _tools: &[xylitol_plugin_sdk::model::XyToolSchema],
            _stream: bool,
            _options: xylitol_plugin_sdk::generate::XyGenerateOptions,
        ) -> Result<xylitol_plugin_sdk::XyStream, xylitol_plugin_sdk::error::XyError> {
            Err(xylitol_plugin_sdk::error::XyError::provider(
                xylitol_plugin_sdk::error::XyProviderError {
                    message: "gate fixture model is not callable".into(),
                    ..Default::default()
                },
            ))
        }
    }

    struct StubFactoryForGate;

    impl OccupancyFactory for StubFactoryForGate {
        fn new_driver_at(&self, _workspace: &Path) -> Box<dyn OccupancyEngine> {
            Box::new(ScriptedEngine {
                runs: Arc::new(StdMutex::new(Vec::new())),
            })
        }

        fn project_trusted(&self, _workspace: &Path) -> bool {
            true
        }

        fn default_model_id(&self) -> Option<String> {
            None
        }

        fn cwd(&self) -> PathBuf {
            PathBuf::from(".")
        }

        fn agent_dir(&self) -> PathBuf {
            PathBuf::from(".")
        }
    }

    fn gate_host() -> Arc<HostState> {
        let store: Arc<dyn XySessionStore> = Arc::new(MemStore::new());
        let ports = RuntimeBaseline {
            store,
            sink: Arc::new(GateSink),
            permission: xylitol_runtime::permission::allow_all_permission(),
            hook_bus: None,
            model_registry: xylitol_runtime::agent::ModelRegistry::new(),
            model_builder: Arc::new(|_| {
                Arc::new(GateNoopModel) as Arc<dyn xylitol_plugin_sdk::XyModel>
            }),
            cwd: ".".into(),
            steering_mode: xylitol_runtime::agent::capabilities::QueueMode::default(),
            follow_up_mode: xylitol_runtime::agent::capabilities::QueueMode::default(),
            compaction_settings: None,
            compaction_factory: xylitol_plugin_sdk::noop_compaction_factory(),
            tools: xylitol_runtime::agent::tools::ToolSet::empty(),
            system_prompt: None,
            context_files: Vec::new(),
            append_system_prompt: Vec::new(),
            skills: Vec::new(),
            batch_mode: xylitol_plugin_sdk::XyBatchMode::Sequential,
        };
        HostState::new(
            ports,
            ReloadBaseline {
                cwd: PathBuf::from("."),
                agent_dir: PathBuf::from("."),
                project_trusted: true,
                default_model_id: None,
                trust_audit: xylitol_runtime::TrustAuditConfig::default(),
            },
            Arc::new(StubFactoryForGate),
            "gate-fallback".into(),
        )
    }

    async fn spin_until(condition: impl Fn() -> bool) {
        for _ in 0..64 {
            tokio::time::advance(Duration::from_millis(20)).await;
            tokio::task::yield_now().await;
            if condition() {
                return;
            }
        }
        panic!("condition not reached under paused time");
    }

    #[tokio::test(start_paused = true)]
    async fn linger_eviction_aborts_when_subscriber_binds_under_gate() {
        let host = gate_host();
        let slot = host.slot("s1").await;
        host.ports
            .store
            .try_acquire_write_lock("s1")
            .expect("subscribe-style lock");
        host.clone().arm_test_linger();

        let gate = slot.eviction_gate.lock().await;
        on_last_subscriber_gone(&host, &slot, false).await;
        for _ in 0..16 {
            tokio::task::yield_now().await;
        }

        let (_id, _rx) = subscribe(&slot).await;
        drop(gate);
        for _ in 0..32 {
            tokio::task::yield_now().await;
        }

        assert!(
            host.existing_slot("s1").await.is_some(),
            "a subscriber bound under the gate must abort linger eviction"
        );
        assert_eq!(HostState::live_subscriber_count(&slot).await, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn late_subscribe_after_linger_eviction_gets_fresh_watermarked_slot() {
        let host = gate_host();
        let slot = host.slot("s1").await;
        for _ in 0..5 {
            slot.journal
                .lock()
                .await
                .append(xylitol_protocol::Event::TextDelta { text: "x".into() });
        }
        host.ports
            .store
            .try_acquire_write_lock("s1")
            .expect("subscribe-style lock");
        host.clone().arm_test_linger();
        on_last_subscriber_gone(&host, &slot, false).await;
        spin_until(|| {
            futures::executor::block_on(async { host.existing_slot("s1").await.is_none() })
        })
        .await;

        let (mux_tx, mux_rx) = mpsc::channel(32);
        let mux_id = host.register_unbound_mux(mux_tx).await;
        let result = gated_subscribe(
            &host,
            slot.clone(),
            "s1",
            0,
            Some(mux_id),
            WorkspaceSet::default(),
        )
        .await;
        assert!(result.ok, "late subscribe must succeed: {result:?}");

        let fresh = host.existing_slot("s1").await.expect("recreated slot");
        assert!(
            !Arc::ptr_eq(&fresh, &slot),
            "late subscribe must bind the recreated slot, never the evicted one"
        );
        assert!(
            fresh.subscribers.lock().await.contains_key(&mux_id),
            "the mux must be bound on the fresh slot"
        );
        assert_eq!(
            fresh.journal.lock().await.next_seq(),
            6,
            "the fresh slot must resume from the preserved watermark"
        );
        drop(mux_rx);
    }

    #[tokio::test(start_paused = true)]
    async fn automation_timer_runs_scheduled_prompt() {
        let slot = slot();
        let store = Arc::new(MemStore::new());
        store
            .append_session_entry("s0", &schedule_entry(Local::now().timestamp() - 1, 1))
            .await
            .expect("schedule");
        let runs = Arc::new(StdMutex::new(Vec::new()));
        *slot.driver.lock().await = Some(Box::new(ScriptedEngine { runs: runs.clone() }));
        let (_id, _rx) = subscribe(&slot).await;
        let engine = Arc::new(Mutex::new(AutomationEngine::new(store, "s0")));
        let task = tokio::spawn(automation_loop(slot.clone(), engine));
        for _ in 0..16 {
            if !runs.lock().map(|runs| runs.is_empty()).unwrap_or(true) {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(
            runs.lock().map(|runs| runs.clone()).unwrap_or_default(),
            vec!["poll".to_string()]
        );
        task.abort();
    }
}

fn journal_wrapped_gap(journal: &EventJournal, last_seq: u64) -> bool {
    journal.is_full() && last_seq < journal.min_seq() && journal.min_seq() > 1
}

async fn restore_default_model(host: &HostState, driver: &mut dyn OccupancyEngine) {
    let Some(id) = host.factory.default_model_id() else {
        return;
    };
    if let Err(e) = driver.restore_model(&id).await {
        log::warn!(
            target: "xylitol::host",
            "writer default model restore failed id={id}: {e}"
        );
    }
}
