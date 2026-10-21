//! Host occupancy: one process-wide [`RuntimeBaseline`] baseline, lazy session slots.
//!
//! Occupancy unit is a **session slot** (journal + seq + writer engine). A
//! single Driver still must not run two root submits or two session bindings
//! at once.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use futures::StreamExt;
use serde_json::{Value, json};
use tokio::sync::{Mutex, RwLock, broadcast, mpsc};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use crate::ToWireEvent;
use crate::occupancy::engine::{OccupancyEngine, OccupancyFactory};
use crate::occupancy::idempotency::{Admission, IdempotencyLedger, conflict_result};
use crate::occupancy::ws::{
    EventJournal, ReverseRpcGateway, ReverseRpcResult, downlink_server_request,
};
use crate::{MUX_CHAN_CAP, MuxSink, OccupiedHost};
use xylitol_plugin_sdk::ask::{AskArgs, AskUserGateway};
use xylitol_plugin_sdk::error::{XySessionStoreError, XyToolError};
use xylitol_plugin_sdk::lifecycle::XyEvent;
use xylitol_protocol::envelope::{
    ApprovalRequestedPayload, HostDescribeValue, PROTOCOL_VERSION, QuestionRequestedPayload,
    RpcMessage, RpcResult, SessionEventPayload, SessionResourcesPayload,
    SessionResyncRequiredPayload, SessionSubscribedPayload,
};
use xylitol_protocol::method::is_unary_method;
use xylitol_protocol::registry::parse_command;
use xylitol_protocol::registry::{
    METHOD_ABORT, METHOD_ARM_TOOL_FREEZE, METHOD_EXPORT_JSONL, METHOD_GET_AVAILABLE_MODELS,
    METHOD_HOST_DESCRIBE, METHOD_IMPORT_JSONL, METHOD_LOADED_RESOURCES, METHOD_PERSIST_TRUST,
    METHOD_PROMPT, METHOD_RELOAD, METHOD_SESSION_TREE, METHOD_SUBSCRIBE,
};
use xylitol_runtime::XyDriverError;
use xylitol_runtime::agent::RuntimeBaseline;
use xylitol_runtime::driver::{
    LoadedResourcesSnapshot, ModelInfo, ProjectTrustMode, RuntimeReloadReport,
};

/// Shared Host process occupancy. Injected into `xylitol-serve` as [`OccupiedHost`].
pub struct HostState {
    pub ports: RuntimeBaseline,
    pub reload: ReloadBaseline,
    pub factory: Arc<dyn OccupancyFactory>,
    pub sessions: RwLock<HashMap<String, Arc<SessionSlot>>>,
    /// Mux connections not yet bound by unary `subscribe`.
    pub unbound_mux: Mutex<Vec<MuxSink>>,
    /// Host-level resource owner for `/reload` and `loaded_resources`.
    pub resource_driver: Mutex<Option<Box<dyn OccupancyEngine>>>,
    /// Downlink bus for the in-process Host carrier.
    pub in_process_downlink: broadcast::Sender<RpcMessage>,
    /// Cancellation token for the process-level reload currently in flight.
    pub reload_cancel: Mutex<Option<CancellationToken>>,
    pub shutting_down: AtomicBool,
    pub fallback_session: String,
    /// Abnormal last-subscriber disconnect grace before releasing occupancy.
    pub write_lock_linger: std::sync::Mutex<Duration>,
    /// Persisted sessions directory (empty when the store is not disk-backed).
    pub sessions_dir: std::sync::Mutex<PathBuf>,
}

/// Process-wide reload / MCP inputs cloned onto each materialized Driver.
#[derive(Clone)]
pub struct ReloadBaseline {
    pub cwd: PathBuf,
    pub agent_dir: PathBuf,
    pub project_trusted: bool,
    /// `models.default_model` (or CLI `--model`) applied when a writer is materialized.
    /// `RuntimeBaseline::materialize_runtime` leaves ModelManager unset; Host MUST select.
    pub default_model_id: Option<String>,
}

/// Per-session occupancy: journal + optional writer Driver + mux subscribers.
pub struct SessionSlot {
    pub session_id: String,
    pub journal: Mutex<EventJournal>,
    pub driver: Arc<Mutex<Option<Box<dyn OccupancyEngine>>>>,
    pub writer: AtomicBool,
    pub subscribers: Mutex<HashMap<u64, MuxSink>>,
    /// c2895 观景态订阅：回合中绑定者仅收 AgentEnd 生命周期帧，显式重挂后才看直播。
    observing: Mutex<HashSet<u64>>,
    linger_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
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
    pub bash_run_cancel: std::sync::Mutex<Option<CancellationToken>>,
    /// Unary idempotency admission keyed by envelope `rpcId` (c2460).
    pub idempotency: IdempotencyLedger,
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
            resource_driver: Mutex::new(None),
            in_process_downlink,
            reload_cancel: Mutex::new(None),
            shutting_down: AtomicBool::new(false),
            fallback_session,
            write_lock_linger: std::sync::Mutex::new(Duration::from_secs(60)),
            sessions_dir: std::sync::Mutex::new(PathBuf::new()),
        })
    }

    pub fn remember_sessions_dir(self: Arc<Self>, sessions_dir: PathBuf) -> Arc<Self> {
        if let Ok(mut g) = self.sessions_dir.lock() {
            *g = sessions_dir;
        }
        self
    }

    /// Production assembly (shared bootstrap ingredients → RuntimeBaseline).
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
        let mut bound = Vec::new();
        for tx in pending {
            bound.push(slot.add_subscriber(tx).await);
        }
        Self::cancel_write_linger(slot).await;
        // c2895：回合成中且冷订阅（last_seq=0）→ 观景态（tape 冻结、AgentEnd 例外、
        // 不重放），显式重挂后再收看直播；中段重连（last_seq>0）与空闲绑定照常直播。
        // 重放定向到本次新绑定者。
        let inflight = slot.run_inflight.load(Ordering::SeqCst);
        let observe = inflight && last_seq == 0;
        if observe {
            slot.observe(bound.iter().copied()).await;
        }
        let result = slot
            .replay_or_resync(&bound, observe, inflight, last_seq)
            .await;
        // c2841：绑定后同步会话写者的当前已解析模型（含用户显式配置的默认模型），
        // 使附加端模型徽标收敛而不是停在 NOT-SET；无写者 / 无模型时 no-op。
        slot.sync_model_downlink().await;
        result
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

    /// Disk path of `session.lock` for a persisted session (tests / BDD).
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
            subscribers: Mutex::new(HashMap::new()),
            observing: Mutex::new(HashSet::new()),
            linger_task: Mutex::new(None),
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

    /// 场景缝（BDD / 单测）：直接置回合成况，供观景态与 busy 拒绝场景
    /// 无需真实回合成流即可驱动。产品路径勿用。
    pub fn mark_run_inflight_for_test(&self, on: bool) {
        self.run_inflight.store(on, Ordering::SeqCst);
    }

    pub async fn flush_pending_runtime_for_test(&self) {
        self.flush_pending_runtime().await;
    }

    /// 订阅确立应答载荷（c2895：附带绑定时刻回合成况 `inflight`）。
    fn subscribed_value(&self, seq: u64, inflight: bool) -> Value {
        let mut v = serde_json::to_value(SessionSubscribedPayload {
            session_id: self.session_id.clone(),
            seq,
        })
        .unwrap_or(Value::Null);
        if let Some(o) = v.as_object_mut() {
            o.insert("inflight".into(), Value::Bool(inflight));
        }
        v
    }

    /// 重放/重同步。c2895：journal 重放定向到本次绑定的新订阅者（在订者
    /// 不再吃重放突发）；观景态绑定（`observe`）不重放 tape（冷快照兜底）。
    /// resync 分支维持广播（整槽条件、罕见路径）；subscribed 确认帧广播
    /// （端侧 skip_cold_replay 清除语义不变）。应答 `inflight` 回显绑定时刻
    /// 回合成况（原始值，观景判定由端侧结合自身冷/暖订阅得出）。
    async fn replay_or_resync(
        &self,
        bound: &[u64],
        observe: bool,
        inflight: bool,
        last_seq: u64,
    ) -> RpcResult {
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
            return RpcResult::ok_value(self.subscribed_value(0, inflight));
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
                RpcResult::ok_value(self.subscribed_value(0, inflight))
            }
            Some(events) => {
                let seq = journal.max_seq();
                drop(journal);
                if !observe {
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
                        for id in bound {
                            self.send_to(*id, &msg).await;
                        }
                    }
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
                RpcResult::ok_value(self.subscribed_value(seq, inflight))
            }
        }
    }

    /// c2895：观景态订阅在 session/event 帧上仅放行 AgentEnd 生命周期帧；
    /// 其余 tape 冻结（冷快照 + 显式重挂兜底）。非 session/event 帧与解析失败放行。
    fn observe_passes(msg: &RpcMessage) -> bool {
        match msg {
            RpcMessage::ServerRequest {
                method, payload, ..
            } if method == "session/event" => payload
                .get("event")
                .and_then(|v| serde_json::from_value::<xylitol_protocol::Event>(v.clone()).ok())
                .map(|ev| matches!(ev, xylitol_protocol::Event::AgentEnd))
                .unwrap_or(true),
            _ => true,
        }
    }

    pub async fn observe<I: IntoIterator<Item = u64>>(&self, ids: I) {
        self.observing.lock().await.extend(ids);
    }

    pub async fn broadcast(&self, msg: RpcMessage) {
        let _ = self.in_process_downlink.send(msg.clone());
        let observe_filter = !Self::observe_passes(&msg);
        let mut subs = self.subscribers.lock().await;
        let mut observing = self.observing.lock().await;
        let mut dead = Vec::new();
        let resync = downlink_server_request(
            "session/resync_required",
            serde_json::to_value(SessionResyncRequiredPayload {
                session_id: self.session_id.clone(),
            })
            .unwrap_or(Value::Null),
        );
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

    /// 定向投递：只发指定订阅者（c2895 重放定向，在订者不重复吃重放突发）。
    async fn send_to(&self, id: u64, msg: &RpcMessage) -> bool {
        let subs = self.subscribers.lock().await;
        match subs.get(&id) {
            Some(tx) => tx.send(msg.clone()).await.is_ok(),
            None => false,
        }
    }

    pub async fn append_and_push(&self, event: xylitol_protocol::Event) {
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

    /// 同步会话写者的当前已解析模型（ModelSelect 下行）给订阅者，使附加端
    /// 模型徽标收敛（c2841）。写者未装配或未持有模型时 no-op；幂等（重复
    /// 绑定只会重播同一当前模型）。模型 id 来自写者 current_model，provider
    /// 按 id 查写者注册表真源（ModelInfo 不携带 provider）。
    ///
    /// c2850/r1926: attach 期同步是订阅者收敛而非会话历史——MUST NOT 落
    /// journal、MUST NOT 消耗 seq（与 `push_resources` 同族）。下行 `seq`
    /// 回显当前 max_seq（不递增），避免客户端 `last_seq` 回退或与 journal 错位。
    pub async fn sync_model_downlink(&self) -> () {
        let Some((provider, model_id)) = self.writer_model().await else {
            return;
        };
        let seq = self.journal.lock().await.max_seq();
        let msg = downlink_server_request(
            "session/event",
            serde_json::to_value(SessionEventPayload {
                session_id: self.session_id.clone(),
                seq,
                event: xylitol_protocol::Event::ModelSelect { provider, model_id },
            })
            .unwrap_or(Value::Null),
        );
        self.broadcast(msg).await;
    }

    async fn writer_model(&self) -> Option<(String, String)> {
        let mut g = self.driver.lock().await;
        let d = g.as_mut()?;
        let id = d.current_model()?.id;
        let provider = d.model_provider(&id).unwrap_or_default();
        Some((provider, id))
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
    pub async fn push_bash_output(&self, chunk: xylitol_plugin_sdk::BashChunk) {
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

/// Ask tool on a Host slot: mux `question/requested`, answer via `answer_question` unary.
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
            Ok(Ok(ReverseRpcResult::Timeout)) | Ok(Err(_)) | Err(_) => {
                self.slot.gateway.remove(&rpc_id);
                Err(XyToolError::Timeout(
                    crate::occupancy::ws::REVERSE_RPC_TIMEOUT,
                ))
            }
        }
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

    async fn register_unbound_mux(&self, tx: MuxSink) {
        HostState::register_unbound_mux(self, tx).await;
    }

    async fn handle_uplink(self: Arc<Self>, bytes: &[u8]) -> Result<Vec<u8>, ()> {
        crate::occupancy::envelope::handle_uplink(&self, bytes)
            .await
            .map_err(|_| ())
    }

    fn encode_downlink(&self, method: &str, payload: serde_json::Value) -> Result<Vec<u8>, String> {
        crate::occupancy::envelope::downlink_frame(method, payload, 0)
    }

    async fn prune_mux(self: Arc<Self>, clean_close: bool) {
        prune_mux_and_maybe_release(&self, clean_close).await;
    }
}

pub fn is_writer_method(method: &str) -> bool {
    // c2530 D3：租约准入由注册表能力位声明，取代手维护的 WRITER_METHODS。
    xylitol_protocol::registry::lookup(method)
        .is_some_and(|e| e.auth == xylitol_protocol::registry::Auth::Writer)
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
    let mgr = xylitol_runtime::trust::TrustManager::new(&host.reload.agent_dir);
    let cwd = workspace.to_string_lossy();
    xylitol_runtime::trust::resolve_project_trusted(
        &mgr,
        &cwd,
        None,
        xylitol_runtime::trust::DefaultProjectTrust::default(),
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
    let mut driver = host.factory.new_driver_at(workspace);
    let _ = project_trusted_for(host, workspace);
    driver.bind_session(&slot.session_id)?;
    driver.install_ask_tool(Arc::new(SlotAskGateway { slot: slot.clone() }));
    driver.begin_mcp_bootstrap().await;
    if let Some(id) = host.factory.default_model_id().as_deref()
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

/// Busy `/model` / thinking: remember for the next run; do not retune this run's LLM calls.
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

fn rpc_err(e: XyDriverError) -> RpcResult {
    RpcResult::error(e.kind(), e.to_string())
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

pub async fn prune_mux_and_maybe_release(host: &Arc<HostState>, clean: bool) {
    let slots: Vec<Arc<SessionSlot>> = host.sessions.read().await.values().cloned().collect();
    for slot in slots {
        if HostState::live_subscriber_count(&slot).await == 0 {
            on_last_subscriber_gone(host, &slot, clean).await;
        }
    }
}

async fn on_last_subscriber_gone(host: &Arc<HostState>, slot: &Arc<SessionSlot>, clean: bool) {
    if slot.run_inflight.load(Ordering::SeqCst) {
        return;
    }
    HostState::cancel_write_linger(slot).await;
    let linger = host.linger_duration();
    if clean || linger.is_zero() {
        host.ports.store.release_write_lock(&slot.session_id);
        return;
    }
    let host = host.clone();
    let slot_wait = slot.clone();
    let task = tokio::spawn(async move {
        tokio::time::sleep(linger).await;
        if slot_wait.run_inflight.load(Ordering::SeqCst) {
            return;
        }
        if HostState::live_subscriber_count(&slot_wait).await == 0 {
            host.ports.store.release_write_lock(&slot_wait.session_id);
        }
    });
    *slot.linger_task.lock().await = Some(task);
}

async fn on_prompt_finished(host: &Arc<HostState>, slot: &Arc<SessionSlot>) {
    if HostState::live_subscriber_count(slot).await == 0 {
        on_last_subscriber_gone(host, slot, false).await;
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
) -> RpcResult {
    if method == METHOD_HOST_DESCRIBE {
        return RpcResult::ok_value(
            serde_json::to_value(HostDescribeValue {
                protocol: PROTOCOL_VERSION,
                formats: vec![xylitol_protocol::WIRE_FORMAT_FORY_V3.into()],
            })
            .unwrap_or(Value::Null),
        );
    }
    if !is_unary_method(method) {
        return RpcResult::error("not_found", format!("unregistered method {method}"));
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

    // c2460: session-scoped admission keyed by the envelope rpcId — first
    // admission wins, duplicates replay the first result, foreign reuse of a
    // key is a stable conflict.
    let reservation = match slot.idempotency.admit(rpc_id, method, &payload).await {
        Admission::Proceed(reservation) => reservation,
        Admission::Replay(result) => return result,
        Admission::Conflict => return conflict_result(),
    };

    let result =
        dispatch_session_unary(host, &slot, &session_id, method, payload, &workspace).await;
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
        HostState::cancel_write_linger(slot).await;
        if let Err(e) = host.ports.store.try_acquire_write_lock(session_id)
            && matches!(e, XySessionStoreError::WriteHeld { .. })
        {
            return occupancy_err(e);
        }
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
        if let Err(e) = ensure_write_occupancy(host, slot, workspace).await {
            return *e;
        }
        // c2895 一回合一演员：根提交准入 CAS——回合成中再发根提交稳定拒绝
        // （对齐「忙碌时二次根提交有明确拒绝」），并关闭第二 `d.run()` 重叠流。
        if slot
            .run_inflight
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return RpcResult::error(
                "session_busy",
                "会话回合进行中：根提交须等待结束后重试，或以插话（steer / follow_up）改向",
            );
        }
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
                let deadline = std::time::Instant::now()
                    + xylitol_runtime::agent::MCP_FIRST_TURN_GATE_TIMEOUT
                    + std::time::Duration::from_millis(250);
                loop {
                    let mut g = driver.lock().await;
                    let Some(d) = g.as_mut() else {
                        slot_push.run_inflight.store(false, Ordering::SeqCst);
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
            on_prompt_finished(&host_push, &slot_push).await;
        });
        return RpcResult::ok_value(json!({ "session_id": session_id }));
    }

    if method == METHOD_ARM_TOOL_FREEZE {
        // r1925: freeze the in-process tool table; MUST NOT take write occupancy
        // so another process holding the kernel lock still gets a snapshot.
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
        // c2847: arm 必须**有界决议**——`ensure_tool_table_frozen` 按首轮门时限
        // （MCP_FIRST_TURN_GATE_TIMEOUT）等待 settle，超时 detach 未连上的 bootstrap
        // 并冻结已 armed 子集后返回。此前仅 `arm_tool_freeze_gate` + 单次 poll 即回快照：
        // 配置了但连不上的 MCP 会永驻 connecting，TUI 的 Assembling 门无限等待、
        // turn 永不启动（runtime 的 ensure 只在 run() 后才触发，形成双向死锁）。
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

    if is_writer_method(method) {
        return dispatch_writer_unary(host, slot, method, payload, workspace).await;
    }

    dispatch_readonly_unary(host, slot, session_id, method, payload, workspace).await
}

/// Writer-method unary: occupancy + inflight deferral + wire staging + shared dispatch.
///
/// Split out of [`dispatch_session_unary`] (c2710) so the entry point stays a
/// thin router (special paths → writer → reader).
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
    if slot.run_inflight.load(Ordering::SeqCst)
        && matches!(method, "set_model" | "cycle_model" | "set_thinking_level")
    {
        return defer_runtime_setting(slot, method, &payload).await;
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
        driver.set_bash_run_sink(Some(xylitol_plugin_sdk::BashOutputSink { tx, cancel }));
        Some(fwd)
    } else {
        None
    };
    let result = match driver.execute_command(cmd).await {
        Ok(mut value) => {
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
            RpcResult::ok_value(value)
        }
        Err(e) => rpc_err(e),
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
