//! Host occupancy: one process-wide [`RuntimePorts`] baseline, lazy session slots.
//!
//! Occupancy unit is a **session slot** (journal + seq + writer engine). A
//! single Driver still must not run two root submits or two session bindings
//! at once.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use futures::StreamExt;
use serde_json::{Value, json};
use tokio::sync::{Mutex, RwLock, broadcast, mpsc};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use crate::agent::RuntimePorts;
use crate::app::core::composition::{BuildAgentOptions, McpServerSpec, build_ports_with_store};
use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::core::driver::{
    LoadedResourcesSnapshot, ModelInfo, ProjectTrustMode, RuntimeReloadReport, XyDriver,
    XyInProcessDriver,
};
use crate::app::core::driver_error::XyDriverError;
use crate::app::server::idempotency::{Admission, IdempotencyLedger, conflict_result};
use crate::app::server::ws::{
    EventJournal, ReverseRpcGateway, ReverseRpcResult, downlink_server_request,
};
use crate::infra::session::SessionManager;
use crate::protocol::error::XyToolError;
use crate::protocol::lifecycle::XyEvent;
use crate::protocol::ports::XySessionStore;
use crate::protocol::ports::ask::{AskArgs, AskUserGateway};
use crate::protocol::wire::envelope::{
    ApprovalRequestedPayload, HostDescribeValue, PROTOCOL_VERSION, QuestionRequestedPayload,
    RpcMessage, RpcResult, SessionEventPayload, SessionResourcesPayload,
    SessionResyncRequiredPayload, SessionSubscribedPayload,
};
use crate::protocol::wire::method::is_unary_method;
use crate::protocol::wire::registry::parse_command;
use crate::protocol::wire::registry::{
    METHOD_ABORT, METHOD_ARM_TOOL_FREEZE, METHOD_EXPORT_JSONL, METHOD_GET_AVAILABLE_MODELS,
    METHOD_HOST_DESCRIBE, METHOD_IMPORT_JSONL, METHOD_LOADED_RESOURCES, METHOD_PERSIST_TRUST,
    METHOD_PROMPT, METHOD_RELOAD, METHOD_SESSION_TREE, METHOD_SUBSCRIBE,
};

/// Bounded per-mux-connection queue. Overflow → `session/resync_required` or drop conn.
pub const MUX_CHAN_CAP: usize = 256;

/// Shared Host process state (salvo Depot).
pub struct HostState {
    pub ports: RuntimePorts,
    pub reload: ReloadBaseline,
    pub sessions: RwLock<HashMap<String, Arc<SessionSlot>>>,
    /// Mux connections not yet bound by unary `subscribe`.
    pub unbound_mux: Mutex<Vec<MuxSink>>,
    /// Host-level resource owner for `/reload` and `loaded_resources`.
    pub resource_driver: Mutex<Option<XyInProcessDriver>>,
    /// Downlink bus for the in-process Host carrier.
    pub in_process_downlink: broadcast::Sender<RpcMessage>,
    /// Cancellation token for the process-level reload currently in flight.
    pub reload_cancel: Mutex<Option<CancellationToken>>,
    pub shutting_down: AtomicBool,
    pub fallback_session: String,
}

/// Process-wide reload / MCP inputs cloned onto each materialized Driver.
#[derive(Clone)]
pub struct ReloadBaseline {
    pub cwd: PathBuf,
    pub agent_dir: PathBuf,
    pub project_trusted: bool,
    pub mcp_servers: Vec<McpServerSpec>,
    /// `models.default_model` (or CLI `--model`) applied when a writer is materialized.
    /// `RuntimePorts::materialize_runtime` leaves ModelManager unset; Host MUST select.
    pub default_model_id: Option<String>,
}

pub type MuxSink = mpsc::Sender<RpcMessage>;

/// Per-session occupancy: journal + optional writer Driver + mux subscribers.
pub struct SessionSlot {
    pub session_id: String,
    pub journal: Mutex<EventJournal>,
    pub driver: Arc<Mutex<Option<XyInProcessDriver>>>,
    pub writer: AtomicBool,
    /// Lease for non-readonly unary. HTTP is not a long-lived connection, so identity is this token.
    pub writer_token: Mutex<Option<String>>,
    pub subscribers: Mutex<HashMap<u64, MuxSink>>,
    pub gateway: Arc<ReverseRpcGateway>,
    pub in_process_downlink: broadcast::Sender<RpcMessage>,
    next_conn: AtomicU64,
    resync_offered: AtomicBool,
    /// True from Host `prompt` spawn start until the run stream ends.
    run_inflight: AtomicBool,
    /// `/model` during inflight: apply after AgentEnd (this run stays frozen).
    pending_model: std::sync::Mutex<Option<String>>,
    pending_thinking: std::sync::Mutex<Option<String>>,
    mcp_watch_started: AtomicBool,
    /// Out-of-band cancel for the interactive bash currently holding the
    /// writer lock (c2760 follow-up: `abort` must reach it without the lock).
    pub(crate) bash_run_cancel: std::sync::Mutex<Option<CancellationToken>>,
    /// Unary idempotency admission keyed by envelope `rpcId` (c2460).
    pub idempotency: IdempotencyLedger,
}

impl HostState {
    pub fn new(ports: RuntimePorts, reload: ReloadBaseline, fallback_session: String) -> Arc<Self> {
        let (in_process_downlink, _) = broadcast::channel(MUX_CHAN_CAP);
        Arc::new(Self {
            ports,
            reload,
            sessions: RwLock::new(HashMap::new()),
            unbound_mux: Mutex::new(Vec::new()),
            resource_driver: Mutex::new(None),
            in_process_downlink,
            reload_cancel: Mutex::new(None),
            shutting_down: AtomicBool::new(false),
            fallback_session,
        })
    }

    /// Production assembly (shared bootstrap ingredients → RuntimePorts).
    pub fn from_bootstrap(
        ports: RuntimePorts,
        reload: ReloadBaseline,
        fallback_session: String,
    ) -> Arc<Self> {
        Self::new(ports, reload, fallback_session)
    }

    /// Isolated store + default ports (BDD / unit tests; no user `~/.xylitol`).
    pub fn for_test() -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!("xylitol-host-{}", uuid::Uuid::new_v4()));
        let store: Arc<dyn XySessionStore> = Arc::new(SessionManager::new(dir.join("sessions")));
        let ports = build_ports_with_store(BuildAgentOptions::default(), store)?;
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let agent_dir = crate::infra::resource::DefaultResourceLoader::default_agent_dir();
        Ok(Self::new(
            ports,
            ReloadBaseline {
                cwd,
                agent_dir,
                project_trusted: true,
                mcp_servers: Vec::new(),
                default_model_id: None,
            },
            "test-session".into(),
        ))
    }

    /// Isolated host with configured MCP servers (parity tests).
    #[cfg(test)]
    pub fn for_test_with_mcp(
        mcp_servers: Vec<McpServerSpec>,
    ) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!("xylitol-host-{}", uuid::Uuid::new_v4()));
        let store: Arc<dyn XySessionStore> = Arc::new(SessionManager::new(dir.join("sessions")));
        let ports = build_ports_with_store(BuildAgentOptions::default(), store)?;
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let agent_dir = crate::infra::resource::DefaultResourceLoader::default_agent_dir();
        Ok(Self::new(
            ports,
            ReloadBaseline {
                cwd,
                agent_dir,
                project_trusted: true,
                mcp_servers,
                default_model_id: None,
            },
            "test-session".into(),
        ))
    }

    pub async fn slot(&self, session_id: &str) -> Arc<SessionSlot> {
        {
            let map = self.sessions.read().await;
            if let Some(s) = map.get(session_id) {
                return s.clone();
            }
        }
        let mut map = self.sessions.write().await;
        map.entry(session_id.to_string())
            .or_insert_with(|| SessionSlot::new(session_id, self.in_process_downlink.clone()))
            .clone()
    }

    /// Existing slot for `session_id` (read-only; never creates one).
    async fn existing_slot(&self, session_id: &str) -> Option<Arc<SessionSlot>> {
        self.sessions.read().await.get(session_id).cloned()
    }

    pub fn session_id_from_payload(&self, payload: &Value) -> String {
        payload
            .get("session_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| self.fallback_session.clone())
    }

    pub async fn register_unbound_mux(&self, tx: MuxSink) {
        self.unbound_mux.lock().await.push(tx);
    }

    pub async fn bind_mux_to_session(&self, slot: &SessionSlot, last_seq: u64) -> RpcResult {
        let pending = {
            let mut unbound = self.unbound_mux.lock().await;
            std::mem::take(&mut *unbound)
        };
        for tx in pending {
            slot.add_subscriber(tx).await;
        }
        slot.replay_or_resync(last_seq).await
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
    ) -> tokio::sync::MutexGuard<'_, Option<XyInProcessDriver>> {
        let mut guard = self.resource_driver.lock().await;
        if guard.is_none() {
            let agent = self.ports.materialize_runtime();
            let mut driver = XyInProcessDriver::new(agent, self.ports.store.clone());
            driver.enable_reload_state(
                self.reload.cwd.clone(),
                self.reload.agent_dir.clone(),
                self.reload.project_trusted,
                self.reload.mcp_servers.clone(),
            );
            *guard = Some(driver);
        }
        guard
    }

    pub async fn reload_resources(&self) -> Result<RuntimeReloadReport, XyDriverError> {
        let mut guard = self.ensure_resource_driver().await;
        let cancel = CancellationToken::new();
        *self.reload_cancel.lock().await = Some(cancel.clone());
        let result = guard
            .as_mut()
            .expect("resource driver initialized")
            .reload_runtime(&cancel)
            .await;
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

    /// Poll the writer for `session_id` when present; otherwise first occupied slot
    /// or the process-level resource driver.
    ///
    /// Drop `sessions` before polling: `poll_mcp_bootstrap` joins a finished
    /// connect task and installs the manager. Attach TUI never calls that on
    /// the writer (Remote poll is cache-only); without this, progress hits
    /// `connecting n/n` then the header sticks at `N configured · 0 connected`.
    pub async fn loaded_resources_snapshot_for(&self, session_id: &str) -> LoadedResourcesSnapshot {
        if !session_id.is_empty() {
            let slot = {
                let sessions = self.sessions.read().await;
                sessions.get(session_id).cloned()
            };
            if let Some(slot) = slot {
                let mut guard = slot.driver.lock().await;
                if let Some(driver) = guard.as_mut() {
                    let _ = driver.poll_mcp_bootstrap().await;
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
                let _ = driver.poll_mcp_bootstrap().await;
                return driver.loaded_resources_snapshot().await;
            }
        }
        let mut guard = self.ensure_resource_driver().await;
        let driver = guard.as_mut().expect("resource driver initialized");
        driver.begin_mcp_bootstrap().await;
        let _ = driver.poll_mcp_bootstrap().await;
        driver.loaded_resources_snapshot().await
    }
}

impl SessionSlot {
    pub fn new(
        session_id: impl Into<String>,
        in_process_downlink: broadcast::Sender<RpcMessage>,
    ) -> Arc<Self> {
        let session_id = session_id.into();
        Arc::new(Self {
            journal: Mutex::new(EventJournal::with_default_capacity(&session_id)),
            session_id,
            driver: Arc::new(Mutex::new(None)),
            writer: AtomicBool::new(false),
            writer_token: Mutex::new(None),
            subscribers: Mutex::new(HashMap::new()),
            gateway: Arc::new(ReverseRpcGateway::new()),
            in_process_downlink,
            next_conn: AtomicU64::new(1),
            resync_offered: AtomicBool::new(false),
            run_inflight: AtomicBool::new(false),
            pending_model: std::sync::Mutex::new(None),
            pending_thinking: std::sync::Mutex::new(None),
            mcp_watch_started: AtomicBool::new(false),
            bash_run_cancel: std::sync::Mutex::new(None),
            idempotency: IdempotencyLedger::default(),
        })
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

    #[cfg(test)]
    pub(crate) fn mark_run_inflight_for_test(&self, on: bool) {
        self.run_inflight.store(on, Ordering::SeqCst);
    }

    #[cfg(test)]
    pub(crate) async fn flush_pending_runtime_for_test(&self) {
        self.flush_pending_runtime().await;
    }

    async fn replay_or_resync(&self, last_seq: u64) -> RpcResult {
        let journal = self.journal.lock().await;
        let wrapped_gap =
            journal.is_full() && last_seq < journal.min_seq() && journal.min_seq() > 1;
        if wrapped_gap && !self.resync_offered.swap(true, Ordering::SeqCst) {
            drop(journal);
            let msg = downlink_server_request(
                "session/resync_required",
                serde_json::to_value(SessionResyncRequiredPayload {
                    session_id: self.session_id.clone(),
                })
                .unwrap_or(Value::Null),
            );
            self.broadcast(msg).await;
            return RpcResult::ok_value(
                serde_json::to_value(SessionSubscribedPayload {
                    session_id: self.session_id.clone(),
                    seq: 0,
                })
                .unwrap_or(Value::Null),
            );
        }
        match journal.replay_from(last_seq) {
            None => {
                drop(journal);
                self.resync_offered.store(true, Ordering::SeqCst);
                let msg = downlink_server_request(
                    "session/resync_required",
                    serde_json::to_value(SessionResyncRequiredPayload {
                        session_id: self.session_id.clone(),
                    })
                    .unwrap_or(Value::Null),
                );
                self.broadcast(msg).await;
                RpcResult::ok_value(
                    serde_json::to_value(SessionSubscribedPayload {
                        session_id: self.session_id.clone(),
                        seq: 0,
                    })
                    .unwrap_or(Value::Null),
                )
            }
            Some(events) => {
                let seq = journal.max_seq();
                drop(journal);
                for (ev_seq, event) in events {
                    let msg = downlink_server_request(
                        "session/event",
                        serde_json::to_value(SessionEventPayload {
                            session_id: self.session_id.clone(),
                            seq: ev_seq,
                            event,
                        })
                        .unwrap_or(Value::Null),
                    );
                    self.broadcast(msg).await;
                }
                let subscribed = downlink_server_request(
                    "session/subscribed",
                    serde_json::to_value(SessionSubscribedPayload {
                        session_id: self.session_id.clone(),
                        seq,
                    })
                    .unwrap_or(Value::Null),
                );
                self.broadcast(subscribed).await;
                RpcResult::ok_value(
                    serde_json::to_value(SessionSubscribedPayload {
                        session_id: self.session_id.clone(),
                        seq,
                    })
                    .unwrap_or(Value::Null),
                )
            }
        }
    }

    pub async fn broadcast(&self, msg: RpcMessage) {
        let _ = self.in_process_downlink.send(msg.clone());
        let mut subs = self.subscribers.lock().await;
        let mut dead = Vec::new();
        let resync = downlink_server_request(
            "session/resync_required",
            serde_json::to_value(SessionResyncRequiredPayload {
                session_id: self.session_id.clone(),
            })
            .unwrap_or(Value::Null),
        );
        for (id, tx) in subs.iter() {
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
        }
    }

    pub async fn append_and_push(&self, event: crate::protocol::Event) {
        let seq = {
            let mut j = self.journal.lock().await;
            j.append(event.clone())
        };
        let msg = downlink_server_request(
            "session/event",
            serde_json::to_value(SessionEventPayload {
                session_id: self.session_id.clone(),
                seq,
                event,
            })
            .unwrap_or(Value::Null),
        );
        self.broadcast(msg).await;
    }

    /// Fixed-zone-only MCP/skills snapshot. Not journaled (must not consume seq).
    pub async fn push_resources(&self, snap: LoadedResourcesSnapshot) {
        let msg = downlink_server_request(
            "session/resources",
            serde_json::to_value(SessionResourcesPayload {
                session_id: self.session_id.clone(),
                snapshot: serde_json::to_value(&snap).unwrap_or(Value::Null),
            })
            .unwrap_or(Value::Null),
        );
        self.broadcast(msg).await;
    }

    /// Interactive bang output event (c2760). Not journaled (must not consume seq).
    pub async fn push_bash_output(&self, chunk: crate::protocol::ports::BashChunk) {
        let msg = downlink_server_request(
            "session/bash_output",
            serde_json::json!({
                "session_id": self.session_id.clone(),
                "bash_id": chunk.bash_id,
                "seq": chunk.seq,
                "data": chunk.data,
            }),
        );
        self.broadcast(msg).await;
    }

    /// Poll the writer locally and push `session/resources` when dirty.
    pub fn ensure_mcp_resources_watch(self: &Arc<Self>) {
        if self.mcp_watch_started.swap(true, Ordering::SeqCst) {
            return;
        }
        let weak = Arc::downgrade(self);
        tokio::spawn(async move {
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
                    if driver.poll_mcp_bootstrap().await {
                        Some(driver.loaded_resources_snapshot().await)
                    } else {
                        None
                    }
                };
                if let Some(snap) = snap {
                    slot.push_resources(snap).await;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        });
    }

    pub async fn request_approval(
        &self,
        rpc_id: String,
    ) -> tokio::sync::oneshot::Receiver<ReverseRpcResult> {
        let rx = self.gateway.register(rpc_id.clone());
        let msg = downlink_server_request(
            "approval/requested",
            serde_json::to_value(ApprovalRequestedPayload {
                call_id: rpc_id.clone(),
            })
            .unwrap_or(Value::Null),
        );
        // Envelope rpcId is the stable reverse-RPC id.
        let RpcMessage::ServerRequest {
            method, payload, ..
        } = msg
        else {
            unreachable!("downlink helper always builds ServerRequest");
        };
        self.broadcast(RpcMessage::ServerRequest {
            rpc_id,
            method,
            payload,
        })
        .await;
        rx
    }

    pub async fn request_question(
        &self,
        rpc_id: String,
        questions: serde_json::Value,
    ) -> tokio::sync::oneshot::Receiver<ReverseRpcResult> {
        let rx = self.gateway.register(rpc_id.clone());
        let mut payload = serde_json::to_value(QuestionRequestedPayload {
            call_id: rpc_id.clone(),
        })
        .unwrap_or(Value::Null);
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("questions".into(), questions);
        }
        self.broadcast(RpcMessage::ServerRequest {
            rpc_id,
            method: "question/requested".into(),
            payload,
        })
        .await;
        rx
    }
}

/// Ask tool on a Host slot: mux `question/requested`, answer via POST /api/respond.
struct SlotAskGateway {
    slot: Arc<SessionSlot>,
}

#[async_trait::async_trait]
impl AskUserGateway for SlotAskGateway {
    async fn prompt(&self, args: AskArgs) -> Result<String, XyToolError> {
        let rpc_id = uuid::Uuid::new_v4().to_string();
        let questions = serde_json::to_value(&args.questions).unwrap_or(Value::Null);
        let rx = self.slot.request_question(rpc_id.clone(), questions).await;
        match timeout(crate::app::server::ws::REVERSE_RPC_TIMEOUT, rx).await {
            Ok(Ok(ReverseRpcResult::Answered(s))) => Ok(s),
            Ok(Ok(ReverseRpcResult::Approved)) => Ok("{}".into()),
            Ok(Ok(ReverseRpcResult::Denied)) => Err(XyToolError::Aborted),
            Ok(Ok(ReverseRpcResult::Timeout)) | Ok(Err(_)) | Err(_) => {
                self.slot.gateway.remove(&rpc_id);
                Err(XyToolError::ExecutionFailed(anyhow::anyhow!(
                    "approval/question timed out"
                )))
            }
        }
    }
}

pub fn is_writer_method(method: &str) -> bool {
    // c2530 D3：租约准入由注册表能力位声明，取代手维护的 WRITER_METHODS。
    crate::protocol::wire::registry::lookup(method)
        .is_some_and(|e| e.auth == crate::protocol::wire::registry::Auth::Writer)
}

fn workspace_from_payload(host: &HostState, payload: &Value) -> PathBuf {
    payload
        .get("cwd")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| host.reload.cwd.clone())
}

fn same_workspace(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => a == b,
    }
}

fn project_trusted_for(host: &HostState, workspace: &Path) -> bool {
    if same_workspace(workspace, &host.reload.cwd) {
        return host.reload.project_trusted;
    }
    let mgr = crate::infra::trust::TrustManager::new(&host.reload.agent_dir);
    let cwd = workspace.to_string_lossy();
    crate::infra::trust::resolve_project_trusted(
        &mgr,
        &cwd,
        None,
        crate::infra::trust::DefaultProjectTrust::default(),
        false,
        |_| None,
    )
    .trusted
}

/// Unit-test seam (driver delegate tests / BDD); product uses `materialize_writer_at`.
#[allow(dead_code)] // unit-test seam, not product-called
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
    if guard.is_some() {
        drop(guard);
        slot.ensure_mcp_resources_watch();
        return Ok(());
    }
    let mut agent = host.ports.materialize_runtime();
    agent.set_cwd(workspace.to_string_lossy().into_owned());
    agent
        .bind_session(slot.session_id.clone())
        .map_err(|e| XyDriverError::from(e.to_string()))?;
    let mut driver = XyInProcessDriver::new(agent, host.ports.store.clone());
    driver.enable_reload_state(
        workspace.to_path_buf(),
        host.reload.agent_dir.clone(),
        project_trusted_for(host, workspace),
        host.reload.mcp_servers.clone(),
    );
    driver.install_ask_tool(Arc::new(SlotAskGateway { slot: slot.clone() }));
    driver.begin_mcp_bootstrap().await;
    if let Some(id) = host.reload.default_model_id.as_deref()
        && let Err(e) = driver.restore_model(id).await
    {
        log::warn!(
            target: "xylitol::host",
            "writer default model restore failed id={id}: {e}"
        );
    }
    *guard = Some(driver);
    slot.writer.store(true, Ordering::SeqCst);
    drop(guard);
    slot.ensure_mcp_resources_watch();
    Ok(())
}

fn new_reader_driver(host: &HostState) -> XyInProcessDriver {
    let mut agent = host.ports.materialize_runtime();
    // otel25: read-only RPCs must never stomp the process obs slot — it belongs
    // to whichever session's writer bound it last.
    agent.set_obs_slot_writes(false);
    let mut driver = XyInProcessDriver::new(agent, host.ports.store.clone());
    driver.enable_reload_state(
        host.reload.cwd.clone(),
        host.reload.agent_dir.clone(),
        host.reload.project_trusted,
        host.reload.mcp_servers.clone(),
    );
    driver
}

async fn materialize_reader(
    host: &HostState,
    session_id: &str,
) -> Result<XyInProcessDriver, XyDriverError> {
    let mut driver = new_reader_driver(host);
    driver.switch_session(session_id).await?;
    Ok(driver)
}

/// Busy `/model` / thinking: remember for the next run; do not retune this run's LLM calls.
async fn defer_runtime_setting(
    slot: &SessionSlot,
    method: &str,
    payload: &Value,
    token: &str,
) -> RpcResult {
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
            RpcResult::ok_value(attach_writer_token(model_data(&model), token))
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
            RpcResult::ok_value(attach_writer_token(model_data(&model), token))
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
            RpcResult::ok_value(attach_writer_token(
                json!({ "thinking_level": level }),
                token,
            ))
        }
        _ => RpcResult::error("not_found", format!("unregistered method {method}")),
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

pub fn outcome_to_value(outcome: DispatchOutcome) -> Value {
    match outcome {
        DispatchOutcome::Aborted { cancelled } => json!({ "cancelled": cancelled }),
        DispatchOutcome::State(st) => json!({
            "session_id": st.session_id,
            "model": st.model.as_ref().map(model_data),
            "thinking_level": st.thinking_level,
            "leaf_entry_id": st.leaf_entry_id,
        }),
        DispatchOutcome::Model(model) => model_data(&model),
        DispatchOutcome::Models(models) => {
            json!({ "models": models.iter().map(model_data).collect::<Vec<_>>() })
        }
        DispatchOutcome::ThinkingLevel(level) => json!({ "thinking_level": level }),
        DispatchOutcome::Bash(r) => json!({
            "output": r.output,
            "exit_code": r.exit_code,
            "cancelled": r.cancelled,
            "truncated": r.truncated,
        }),
        DispatchOutcome::Compacted(did) => json!({ "compacted": did }),
        DispatchOutcome::SessionStats(v) => v,
        DispatchOutcome::ExportedPath(p) => json!({ "path": p }),
        DispatchOutcome::NewSession(id) | DispatchOutcome::SwitchedSession(id) => {
            json!({ "session_id": id })
        }
        DispatchOutcome::Messages { entries, .. } => json!({
            "entries": serde_json::to_value(entries).unwrap_or(Value::Null)
        }),
        DispatchOutcome::SessionTree(tree) => json!({
            "tree": serde_json::to_value(tree).unwrap_or(Value::Null)
        }),
        DispatchOutcome::SessionTreeTravel(travel) => {
            serde_json::to_value(travel).unwrap_or(Value::Null)
        }
        DispatchOutcome::Sessions(sessions) => json!({
            "sessions": serde_json::to_value(sessions).unwrap_or(Value::Null)
        }),
        DispatchOutcome::SessionEntries(entries) => json!({
            "entries": serde_json::to_value(entries).unwrap_or(Value::Null)
        }),
        DispatchOutcome::SessionName(name) => json!({ "name": name }),
        DispatchOutcome::Reload(report) => serde_json::to_value(report).unwrap_or(Value::Null),
        DispatchOutcome::LoadedResources(snapshot) => {
            serde_json::to_value(snapshot).unwrap_or(Value::Null)
        }
        DispatchOutcome::Empty => json!({}),
        DispatchOutcome::Commands(cmds) => {
            let cmds: Vec<Value> = cmds
                .into_iter()
                .map(|c| json!({ "name": c.name, "description": c.description }))
                .collect();
            json!({ "commands": cmds })
        }
        DispatchOutcome::QueueStats {
            steer_count,
            follow_up_count,
        } => json!({ "steer_count": steer_count, "follow_up_count": follow_up_count }),
    }
}

fn rpc_err(e: XyDriverError) -> RpcResult {
    RpcResult::error(e.kind(), e.to_string())
}

fn attach_writer_token(mut value: Value, token: &str) -> Value {
    if let Value::Object(map) = &mut value {
        map.insert("writerToken".into(), json!(token));
    }
    value
}

async fn take_writer_lease(
    slot: &SessionSlot,
    presented: Option<&str>,
) -> Result<String, RpcResult> {
    let mut guard = slot.writer_token.lock().await;
    match guard.as_ref() {
        None => {
            let token = uuid::Uuid::new_v4().to_string();
            *guard = Some(token.clone());
            slot.writer.store(true, Ordering::SeqCst);
            Ok(token)
        }
        Some(existing) if presented == Some(existing.as_str()) => Ok(existing.clone()),
        Some(_) => Err(RpcResult::error(
            "writer_conflict",
            "another client is the writer for this session",
        )),
    }
}

/// Guard for a writer-method unary call: acquires the session's writer lease,
/// materializes the engine, and stamps every response — success or failure —
/// with the lease token so the client can keep writing.
struct WriterLease {
    token: String,
}

impl WriterLease {
    async fn acquire(
        host: &HostState,
        slot: &Arc<SessionSlot>,
        workspace: &Path,
        presented: Option<&str>,
    ) -> Result<Self, RpcResult> {
        let token = take_writer_lease(slot, presented).await?;
        if let Err(e) = materialize_writer_at(host, slot, workspace).await {
            let mut r = rpc_err(e);
            r.value = Some(json!({ "writerToken": token }));
            return Err(r);
        }
        Ok(Self { token })
    }

    /// Attach the lease token to `result` (inserted into success payloads,
    /// replacing the value on failures).
    fn seal(&self, mut result: RpcResult) -> RpcResult {
        if result.ok {
            if let Some(v) = result.value.take() {
                result.value = Some(attach_writer_token(v, &self.token));
            }
        } else {
            result.value = Some(json!({ "writerToken": self.token }));
        }
        result
    }
}

/// Dispatch a registered unary method.
///
/// `rpc_id` is the envelope `rpcId` on wire carriers and doubles as the
/// idempotency admission key (c2460); carriers without a wire envelope pass
/// `None` and skip admission.
pub async fn handle_unary(
    host: &Arc<HostState>,
    rpc_id: Option<&str>,
    method: &str,
    payload: Value,
    writer_token: Option<String>,
) -> RpcResult {
    if method == METHOD_HOST_DESCRIBE {
        return RpcResult::ok_value(
            serde_json::to_value(HostDescribeValue {
                protocol: PROTOCOL_VERSION,
            })
            .unwrap_or(Value::Null),
        );
    }
    if !is_unary_method(method) {
        return RpcResult::error("not_found", format!("unregistered method {method}"));
    }

    if method == METHOD_RELOAD {
        return match host.reload_resources().await {
            Ok(report) => RpcResult::ok_value(outcome_to_value(DispatchOutcome::Reload(report))),
            Err(e) => rpc_err(e),
        };
    }
    if method == METHOD_LOADED_RESOURCES {
        let session_id = host.session_id_from_payload(&payload);
        let snapshot = host.loaded_resources_snapshot_for(&session_id).await;
        return RpcResult::ok_value(outcome_to_value(DispatchOutcome::LoadedResources(snapshot)));
    }
    if method == METHOD_ABORT {
        // A running interactive bash holds the writer lock, so a `Command::Abort`
        // dispatch would queue behind it; cancel the run out-of-band first
        // (c2760 follow-up: Esc must kill the host-side process tree).
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
    let workspace = workspace_from_payload(host, &payload);
    let slot = host.slot(&session_id).await;
    let presented = writer_token.as_deref();

    // c2460: session-scoped admission keyed by the envelope rpcId — first
    // admission wins, duplicates replay the first result, foreign reuse of a
    // key is a stable conflict.
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
        presented,
        &workspace,
    )
    .await;
    reservation.complete(result.clone());
    result
}

/// Session-scoped unary execution, after idempotency admission.
async fn dispatch_session_unary(
    host: &Arc<HostState>,
    slot: &Arc<SessionSlot>,
    session_id: &str,
    method: &str,
    payload: Value,
    presented: Option<&str>,
    workspace: &Path,
) -> RpcResult {
    if method == METHOD_SUBSCRIBE {
        if session_id.is_empty() {
            return RpcResult::error("invalid_input", "subscribe requires session_id");
        }
        let last_seq = payload
            .get("last_seq")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        if let Err(e) = materialize_writer_at(host, slot, workspace).await {
            log::warn!(
                target: "xylitol::host",
                "subscribe materialize_writer failed: {e}"
            );
        }
        return host.bind_mux_to_session(slot, last_seq).await;
    }

    if method == METHOD_PROMPT {
        let message = payload
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if message.is_empty() {
            return RpcResult::error("invalid_input", "missing message");
        }
        let lease = match WriterLease::acquire(host, slot, workspace, presented).await {
            Ok(l) => l,
            Err(e) => return e,
        };
        let driver = slot.driver.clone();
        let slot_push = slot.clone();
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
                let deadline = std::time::Instant::now()
                    + crate::agent::MCP_FIRST_TURN_GATE_TIMEOUT
                    + std::time::Duration::from_millis(250);
                loop {
                    let mut g = driver.lock().await;
                    let Some(d) = g.as_mut() else {
                        return;
                    };
                    d.arm_tool_freeze_gate().await;
                    let _ = d.poll_mcp_bootstrap().await;
                    if d.is_tools_frozen() || std::time::Instant::now() >= deadline {
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
                        slot_push.run_inflight.store(true, Ordering::SeqCst);
                        let stream = d.run(&message).await;
                        drop(g);
                        break stream;
                    }
                    drop(g);
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
            };
            let mut stream = stream;
            while let Some(event) = stream.next().await {
                if let Some(pe) = event.to_wire_event() {
                    slot_push.append_and_push(pe).await;
                }
                if matches!(event, XyEvent::AgentEnd { .. }) {
                    break;
                }
            }
            slot_push.run_inflight.store(false, Ordering::SeqCst);
            slot_push.flush_pending_runtime().await;
        });
        return lease.seal(RpcResult::ok_value(json!({ "session_id": session_id })));
    }

    if method == METHOD_ARM_TOOL_FREEZE {
        let lease = match WriterLease::acquire(host, slot, workspace, presented).await {
            Ok(l) => l,
            Err(e) => return e,
        };
        let mut g = slot.driver.lock().await;
        let Some(driver) = g.as_mut() else {
            return RpcResult::error("unavailable", "no writer engine");
        };
        driver.arm_tool_freeze_gate().await;
        let _ = driver.poll_mcp_bootstrap().await;
        let snapshot = driver.loaded_resources_snapshot().await;
        return lease.seal(RpcResult::ok_value(
            serde_json::to_value(&snapshot).unwrap_or(Value::Null),
        ));
    }

    if method == METHOD_PERSIST_TRUST {
        let lease = match WriterLease::acquire(host, slot, workspace, presented).await {
            Ok(l) => l,
            Err(e) => return e,
        };
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
            Err(e) => return lease.seal(rpc_err(e)),
        };
        return lease.seal(RpcResult::ok_value(json!({
            "trusted": report.trusted,
            "saved_path": report.saved_path,
            "message": report.message,
        })));
    }

    if is_writer_method(method) {
        return dispatch_writer_unary(host, slot, method, payload, presented, workspace).await;
    }

    dispatch_readonly_unary(host, slot, session_id, method, payload, workspace).await
}

/// Writer-method unary: lease + inflight deferral + wire staging + shared dispatch.
///
/// Split out of [`dispatch_session_unary`] (c2710) so the entry point stays a
/// thin router (special paths → writer → reader).
async fn dispatch_writer_unary(
    host: &Arc<HostState>,
    slot: &Arc<SessionSlot>,
    method: &str,
    payload: Value,
    presented: Option<&str>,
    workspace: &Path,
) -> RpcResult {
    let lease = match WriterLease::acquire(host, slot, workspace, presented).await {
        Ok(l) => l,
        Err(e) => return e,
    };
    if slot.run_inflight.load(Ordering::SeqCst)
        && matches!(method, "set_model" | "cycle_model" | "set_thinking_level")
    {
        return defer_runtime_setting(slot, method, &payload, &lease.token).await;
    }
    // Export over the wire stages to a unique Host-side temp file; the
    // content is read back into the response and the TUI writes its own
    // local copy (D4: the file lands on the machine that asked for it).
    let staged_export =
        matches!(method, "export_html" | "export_jsonl") && payload.get("output_path").is_none();
    // Import over the wire stages the pushed content symmetrically
    // (sr-imp1): the client reads its local file and sends `content`;
    // the Host lands it on a unique temp input path, dispatches, and
    // cleans up afterwards.
    let staged_import = method == METHOD_IMPORT_JSONL
        && payload.get("input_path").is_none()
        && payload.get("content").is_some();
    let import_temp = if staged_import {
        let temp =
            std::env::temp_dir().join(format!("xylitol-import-{}.jsonl", uuid::Uuid::new_v4()));
        if let Err(e) = std::fs::write(&temp, payload["content"].as_str().unwrap_or_default()) {
            return RpcResult::error("internal_error", format!("stage import write: {e}"));
        }
        Some(temp)
    } else {
        None
    };
    let mut payload = payload;
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
    let cmd = match parse_command(method, &payload) {
        Ok(c) => c,
        Err(e) => return RpcResult::error("invalid_input", e),
    };
    let mut g = slot.driver.lock().await;
    let Some(driver) = g.as_mut() else {
        return RpcResult::error("unavailable", "no writer engine");
    };
    // c2760: forward interactive bang output as non-journal session events;
    // register the run cancel so `abort` can reach it while the writer lock
    // is held by this executing bash unary.
    let bash_forwarder = if method == "bash" {
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
        driver.set_bash_run_sink(Some(crate::protocol::ports::BashOutputSink { tx, cancel }));
        Some(fwd)
    } else {
        None
    };
    let result = match dispatch(driver, cmd).await {
        Ok(outcome) => {
            let mut value = outcome_to_value(outcome);
            if staged_export
                && let Some(p) = value
                    .get("path")
                    .and_then(Value::as_str)
                    .map(std::path::PathBuf::from)
            {
                match std::fs::read_to_string(&p) {
                    Ok(content) => {
                        value["content"] = Value::String(content);
                    }
                    Err(e) => log::warn!(
                        target: "xylitol::host",
                        "export stage read {}: {e}",
                        p.display()
                    ),
                }
                let _ = std::fs::remove_file(&p);
            }
            lease.seal(RpcResult::ok_value(value))
        }
        Err(e) => lease.seal(rpc_err(e)),
    };
    if bash_forwarder.is_some() {
        driver.set_bash_run_sink(None);
        if let Ok(mut slot_cancel) = slot.bash_run_cancel.lock() {
            *slot_cancel = None;
        }
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

/// Read-only unary: cache/快速 paths + reader-driver materialization + shared dispatch.
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
            return match dispatch(&mut reader, cmd).await {
                Ok(outcome) => RpcResult::ok_value(outcome_to_value(outcome)),
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
                "get_messages" => RpcResult::ok_value(json!({ "entries": [] })),
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
        return match dispatch(&mut reader, cmd).await {
            Ok(outcome) => RpcResult::ok_value(outcome_to_value(outcome)),
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
    match dispatch(driver, cmd).await {
        Ok(outcome) => RpcResult::ok_value(outcome_to_value(outcome)),
        Err(e) => rpc_err(e),
    }
}
