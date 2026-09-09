//! Remote HTTP/WS [`XyRemoteDriver`] (feature = "server").

use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use serde_json::Value;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::app::core::host_client::{HostClient, HostClientError, HttpWsClient};
use crate::protocol::model::THINKING_OFF;
use crate::protocol::ports::XyBashResult;
use crate::protocol::session::{SessionEntry, SessionTreeKind, SessionTreeNode, SessionTreeTravel};
use crate::protocol::wire::Command;
use crate::protocol::{Event, RpcMessage};

use super::XyDriver;
use super::XyDriverError;
use super::types::{
    ClipboardCopyOutcome, CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot,
    ModelInfo, ProjectTrustMode, ProjectTrustPersistReport, QueueStats, ReloadStepReport,
    RuntimeReloadReport, SessionListEntry, SessionStats, XyEvent, estimate_from_session_entries,
};

/// Notify the product TUI of mux reverse-RPC (approval/question).
pub type ReverseRpcNotify = Arc<dyn Fn(String, String, Value) + Send + Sync>;

#[derive(Clone)]
struct SharedDownlink {
    events: Arc<std::sync::Mutex<VecDeque<XyEvent>>>,
    notify: Arc<Notify>,
    started: Arc<AtomicBool>,
    /// ath43 coalescing window in millis (trailing-edge debounce).
    window_ms: Arc<AtomicU64>,
    notify_scheduled: Arc<AtomicBool>,
}

impl SharedDownlink {
    fn new() -> Self {
        Self {
            events: Arc::new(std::sync::Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
            started: Arc::new(AtomicBool::new(false)),
            window_ms: Arc::new(AtomicU64::new(10)),
            notify_scheduled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Inject the coalescing window (c2480 tunings; MUST precede the first
    /// downlink burst to matter).
    fn set_window(&self, window: Duration) {
        self.window_ms
            .store(window.as_millis() as u64, Ordering::SeqCst);
    }

    fn push(&self, ev: XyEvent) {
        if let Ok(mut q) = self.events.lock() {
            q.push_back(ev);
        }
        self.request_notify();
    }

    /// One notify per burst: pushes inside the window share a single wake, so
    /// the consumer drains the whole batch as one projection pass.
    fn request_notify(&self) {
        if self.notify_scheduled.swap(true, Ordering::SeqCst) {
            return;
        }
        let notify = self.notify.clone();
        let scheduled = self.notify_scheduled.clone();
        let window_ms = self.window_ms.load(Ordering::SeqCst);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(window_ms)).await;
            // Re-arm before waking: a push landing between these two lines
            // schedules a fresh debounce instead of racing a spent permit.
            scheduled.store(false, Ordering::SeqCst);
            notify.notify_one();
        });
    }

    fn drain(&self) -> Vec<XyEvent> {
        self.events
            .lock()
            .map(|mut q| q.drain(..).collect())
            .unwrap_or_default()
    }

    fn push_front_batch(&self, rest: Vec<XyEvent>) {
        if rest.is_empty() {
            return;
        }
        if let Ok(mut q) = self.events.lock() {
            for ev in rest.into_iter().rev() {
                q.push_front(ev);
            }
        }
    }
}

/// c2480 connection-resilience knobs. Product constants by default; tests
/// inject micro-second values through [`XyRemoteDriver::with_tunings`] —
/// there is deliberately no user-facing config surface for these.
#[derive(Debug, Clone)]
pub struct LinkTunings {
    /// First reconnect delay; every young connection doubles it.
    pub backoff_base: Duration,
    /// Escalation ceiling.
    pub backoff_cap: Duration,
    /// A connection that lived at least this long resets backoff to base —
    /// flapping links (die young) escalate instead of hammering the port.
    pub survive_threshold: Duration,
    /// Downlink event coalescing window (ath43): one notify per burst.
    pub coalesce_window: Duration,
}

impl Default for LinkTunings {
    fn default() -> Self {
        Self {
            backoff_base: Duration::from_millis(200),
            backoff_cap: Duration::from_secs(5),
            survive_threshold: Duration::from_secs(1),
            coalesce_window: Duration::from_millis(10),
        }
    }
}

/// Remote driver — [`XyDriver`] over a [`HostClient`] carrier.
#[cfg(feature = "server")]
pub struct XyRemoteDriver<C = HttpWsClient> {
    host: C,
    session_id: String,
    session_life: CancellationToken,
    turn_cancel: Arc<std::sync::Mutex<CancellationToken>>,
    thinking: std::sync::Mutex<String>,
    leaf_entry_id: Arc<std::sync::Mutex<Option<String>>>,
    last_seq: Arc<AtomicU64>,
    /// c2480 downlink generation: bumped on every (re)start so a stale loop's
    /// late pushes cannot reach the shared projection.
    downlink_gen: Arc<AtomicU64>,
    /// Set when the downlink loop dies irrecoverably (e.g. protocol mismatch,
    /// ath44): `wait_subscribed` fails fast with this message instead of
    /// riding out its full deadline.
    fatal: Arc<std::sync::Mutex<Option<String>>>,
    tunings: Arc<LinkTunings>,
    reverse_rpc: Option<ReverseRpcNotify>,
    downlink: SharedDownlink,
    resync_needed: Arc<AtomicBool>,
    subscribed_ok: Arc<AtomicBool>,
    /// When true, mux drops Agent/Turn/tool tape from a `last_seq=0` subscribe
    /// (cold attach). Cleared on `session/subscribed`. Mid-turn reconnect uses
    /// `last_seq>0` and keeps replay.
    skip_cold_replay: Arc<AtomicBool>,
    /// TUI process cwd; injected on every unary so Host can bind the session workspace.
    client_cwd: String,
    cached_model: Arc<std::sync::Mutex<Option<ModelInfo>>>,
    cached_models: Arc<std::sync::Mutex<Option<Vec<ModelInfo>>>>,
    cached_commands: Arc<std::sync::Mutex<Option<Vec<CommandInfo>>>>,
    cached_queue: Arc<std::sync::Mutex<QueueStats>>,
    cached_resources: Arc<std::sync::Mutex<LoadedResourcesSnapshot>>,
    cached_skills: Arc<std::sync::Mutex<Vec<(String, String)>>>,
    cached_gate_notice: Arc<std::sync::Mutex<Option<String>>>,
    gate_notice_consumed: Arc<AtomicBool>,
    /// Set when mux `session/resources` (or a snapshot unary) updates the cache.
    resources_dirty: Arc<AtomicBool>,
}

#[cfg(feature = "server")]
impl XyRemoteDriver<HttpWsClient> {
    /// Create a new XyRemoteDriver connected to `base_url`.
    ///
    /// `base_url` should be the server root, e.g. `http://127.0.0.1:18790`.
    pub fn new(base_url: impl Into<String>, session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        let session_id = if session_id.trim().is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            session_id
        };
        Self::with_host(HttpWsClient::new(base_url), session_id)
    }

    pub fn host_client(&self) -> &HttpWsClient {
        &self.host
    }
}

#[cfg(feature = "server")]
/// Per-generation downlink state: the driver's shared handles cloned once for
/// one spawned reconnect loop, plus the loop's generation snapshot (c2480).
struct DownlinkCtx<C> {
    host: C,
    session_id: String,
    last_seq: Arc<AtomicU64>,
    reverse_rpc: Option<ReverseRpcNotify>,
    downlink: SharedDownlink,
    life: CancellationToken,
    resync_needed: Arc<AtomicBool>,
    subscribed_ok: Arc<AtomicBool>,
    skip_cold_replay: Arc<AtomicBool>,
    cached_queue: Arc<std::sync::Mutex<QueueStats>>,
    cached_skills: Arc<std::sync::Mutex<Vec<(String, String)>>>,
    cached_resources: Arc<std::sync::Mutex<LoadedResourcesSnapshot>>,
    cached_gate_notice: Arc<std::sync::Mutex<Option<String>>>,
    gate_notice_consumed: Arc<AtomicBool>,
    resources_dirty: Arc<AtomicBool>,
    client_cwd: String,
    downlink_gen: Arc<AtomicU64>,
    fatal: Arc<std::sync::Mutex<Option<String>>>,
    tunings: Arc<LinkTunings>,
    /// Generation this loop belongs to (ath41): frames from a superseded
    /// generation are dropped before they reach the projection.
    my_gen: u64,
}

#[cfg(feature = "server")]
/// Cache a resources snapshot and flip the dirty flag — shared by the
/// driver-side snapshot paths (`apply_resources_cache`) and the downlink
/// `session/resources` arm.
fn store_resources_snapshot(
    cached_skills: &std::sync::Mutex<Vec<(String, String)>>,
    gate_notice_consumed: &AtomicBool,
    cached_gate_notice: &std::sync::Mutex<Option<String>>,
    cached_resources: &std::sync::Mutex<LoadedResourcesSnapshot>,
    resources_dirty: &AtomicBool,
    snap: LoadedResourcesSnapshot,
) {
    let skills: Vec<(String, String)> = snap
        .skill_names
        .iter()
        .map(|name| (name.clone(), String::new()))
        .collect();
    if let Ok(mut cached) = cached_skills.lock() {
        *cached = skills;
    }
    if let Some(notice) = snap.mcp_gate_notice.as_ref()
        && !gate_notice_consumed.load(Ordering::SeqCst)
        && let Ok(mut cached) = cached_gate_notice.lock()
    {
        *cached = Some(notice.clone());
    }
    if let Ok(mut cached) = cached_resources.lock() {
        *cached = snap;
    }
    resources_dirty.store(true, Ordering::SeqCst);
}

#[cfg(feature = "server")]
/// One downlink frame arm (connected phase): session event tape,
/// subscribe/resync/resources lifecycle, reverse-RPC prompts. Unknown frames
/// are ignored; a malformed event is a no-op — recovery is driven by
/// reconnect, never by frame errors (ath42).
async fn handle_frame<C>(ctx: &DownlinkCtx<C>, push_current: &impl Fn(XyEvent), frame: RpcMessage)
where
    C: HostClient + Clone + 'static,
{
    match frame {
        RpcMessage::ServerRequest {
            method, payload, ..
        } if method == "session/event" => {
            if let Some(s) = payload.get("seq").and_then(Value::as_u64) {
                ctx.last_seq.store(s, Ordering::SeqCst);
            }
            let event_val = payload.get("event").cloned().unwrap_or(payload);
            let Ok(ev) = serde_json::from_value::<Event>(event_val) else {
                return;
            };
            let Ok(agent_event) = XyEvent::try_from(&ev) else {
                return;
            };
            if let XyEvent::QueueUpdate {
                steer_count,
                follow_up_count,
            } = &agent_event
                && let Ok(mut cached) = ctx.cached_queue.lock()
            {
                *cached = QueueStats {
                    steer_count: *steer_count,
                    follow_up_count: *follow_up_count,
                };
            }
            if ctx.skip_cold_replay.load(Ordering::SeqCst)
                && XyRemoteDriver::<C>::is_cold_replay_tape(&agent_event)
            {
                return;
            }
            push_current(agent_event);
        }
        RpcMessage::ServerRequest { method, .. } if method == "session/subscribed" => {
            ctx.skip_cold_replay.store(false, Ordering::SeqCst);
        }
        RpcMessage::ServerRequest {
            method, payload, ..
        } if method == "session/resources" => {
            let snap_val = payload.get("snapshot").cloned().unwrap_or(payload);
            if let Ok(snap) = serde_json::from_value::<LoadedResourcesSnapshot>(snap_val) {
                store_resources_snapshot(
                    &ctx.cached_skills,
                    &ctx.gate_notice_consumed,
                    &ctx.cached_gate_notice,
                    &ctx.cached_resources,
                    &ctx.resources_dirty,
                    snap,
                );
            }
        }
        RpcMessage::ServerRequest { method, .. } if method == "session/resync_required" => {
            ctx.last_seq.store(0, Ordering::SeqCst);
            ctx.resync_needed.store(true, Ordering::SeqCst);
            let _ = ctx
                .host
                .unary(
                    "subscribe",
                    serde_json::json!({
                        "session_id": ctx.session_id,
                        "last_seq": 0,
                        "cwd": ctx.client_cwd,
                    }),
                )
                .await;
        }
        RpcMessage::ServerRequest {
            rpc_id,
            method,
            payload,
        } if method == "approval/requested" || method == "question/requested" => {
            if let Some(notify) = &ctx.reverse_rpc {
                notify(rpc_id, method, payload);
            }
        }
        _ => {}
    }
}

#[cfg(feature = "server")]
/// ath42: silent reconnect pause — apply the flapping-aware backoff rule
/// (ath41), then sleep until the pause elapses or the lifecycle token
/// cancels. `None` = cancelled: stop the loop instead of reconnecting.
async fn reconnect_pause<C>(
    ctx: &DownlinkCtx<C>,
    backoff: Duration,
    lived: Duration,
    next_backoff: impl Fn(Duration, Duration) -> Duration,
) -> Option<Duration> {
    if ctx.life.is_cancelled() {
        return None;
    }
    let backoff = next_backoff(backoff, lived);
    tokio::select! {
        _ = ctx.life.cancelled() => None,
        _ = tokio::time::sleep(backoff) => Some(backoff),
    }
}

#[cfg(feature = "server")]
/// The downlink reconnect loop: connect → subscribe → dispatch until the
/// lifecycle token cancels. Split from the former inline `ensure_downlink`
/// body; ath41/ath42/ath44 and c2480 semantics are unchanged.
async fn downlink_loop<C>(ctx: DownlinkCtx<C>)
where
    C: HostClient + Clone + 'static,
{
    // ath41/c2480: pushes from a downlink generation are dropped the
    // moment the driver moves past it — stale frames never reach the
    // shared projection.
    let push_current = {
        let downlink = ctx.downlink.clone();
        let downlink_gen = ctx.downlink_gen.clone();
        let my_gen = ctx.my_gen;
        move |ev: XyEvent| {
            if downlink_gen.load(Ordering::SeqCst) == my_gen {
                downlink.push(ev);
            }
        }
    };
    // ath41: a connection that survived the threshold earned a
    // backoff reset; one that died young escalates (flapping ≠ outage).
    let next_backoff = {
        let tunings = ctx.tunings.clone();
        move |backoff: Duration, lived: Duration| -> Duration {
            if lived >= tunings.survive_threshold {
                tunings.backoff_base
            } else {
                (backoff * 2).min(tunings.backoff_cap)
            }
        }
    };
    let mut backoff = ctx.tunings.backoff_base;
    loop {
        if ctx.life.is_cancelled() {
            break;
        }
        let connected_at = tokio::time::Instant::now();
        match ctx.host.mux().await {
            Ok(mut mux) => {
                // ath44: the carrier already validated this
                // connection's server_hello before handing us the
                // stream; a mismatch would have surfaced as a fatal
                // ProtocolMismatch below.
                let seq = ctx.last_seq.load(Ordering::SeqCst);
                ctx.skip_cold_replay.store(seq == 0, Ordering::SeqCst);
                if ctx
                    .host
                    .unary(
                        "subscribe",
                        serde_json::json!({
                            "session_id": ctx.session_id,
                            "last_seq": seq,
                            "cwd": ctx.client_cwd,
                        }),
                    )
                    .await
                    .is_err()
                {
                    // ath42: silent — reconnect churn must not paint
                    // the transcript; fixed-zone grace UX handles notice.
                    ctx.subscribed_ok.store(false, Ordering::SeqCst);
                    match reconnect_pause(&ctx, backoff, connected_at.elapsed(), &next_backoff)
                        .await
                    {
                        Some(next) => backoff = next,
                        None => return,
                    }
                    continue;
                }
                ctx.subscribed_ok.store(true, Ordering::SeqCst);
                loop {
                    tokio::select! {
                        _ = ctx.life.cancelled() => return,
                        msg = mux.next() => {
                            match msg {
                                Some(Ok(frame)) => {
                                    handle_frame(&ctx, &push_current, frame).await;
                                }
                                Some(Err(_)) => {
                                    // ath42: silent break — the frame
                                    // error is a connection fact, not
                                    // a transcript event; reconnect
                                    // and re-subscribe handle recovery.
                                    break;
                                }
                                None => break,
                            }
                        }
                    }
                }
                // Connection ended: report Down for the grace UX
                // (ath42), survive-or-escalate, then retry.
                ctx.subscribed_ok.store(false, Ordering::SeqCst);
                match reconnect_pause(&ctx, backoff, connected_at.elapsed(), &next_backoff).await {
                    Some(next) => backoff = next,
                    None => return,
                }
            }
            Err(e) if matches!(e, HostClientError::ProtocolMismatch { .. }) => {
                // ath44: a version the client cannot speak is fatal,
                // never a transient failure — surface once and stop.
                let msg = e.to_string();
                if let Ok(mut slot) = ctx.fatal.lock() {
                    *slot = Some(msg.clone());
                }
                push_current(XyEvent::error_msg(msg));
                return;
            }
            Err(_) => {
                // ath42: silent — reconnect churn must not paint the
                // transcript; fixed-zone grace UX handles the notice.
                ctx.subscribed_ok.store(false, Ordering::SeqCst);
                match reconnect_pause(&ctx, backoff, connected_at.elapsed(), &next_backoff).await {
                    Some(next) => backoff = next,
                    None => return,
                }
            }
        }
    }
}

#[cfg(feature = "server")]
impl<C> XyRemoteDriver<C>
where
    C: HostClient + Clone + 'static,
{
    pub fn with_host(host: C, session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        let session_id = if session_id.trim().is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            session_id
        };
        Self {
            host,
            session_id,
            session_life: CancellationToken::new(),
            turn_cancel: Arc::new(std::sync::Mutex::new(CancellationToken::new())),
            thinking: std::sync::Mutex::new(THINKING_OFF.into()),
            leaf_entry_id: Arc::new(std::sync::Mutex::new(None)),
            last_seq: Arc::new(AtomicU64::new(0)),
            downlink_gen: Arc::new(AtomicU64::new(0)),
            fatal: Arc::new(std::sync::Mutex::new(None)),
            tunings: Arc::new(LinkTunings::default()),
            reverse_rpc: None,
            downlink: SharedDownlink::new(),
            resync_needed: Arc::new(AtomicBool::new(false)),
            subscribed_ok: Arc::new(AtomicBool::new(false)),
            skip_cold_replay: Arc::new(AtomicBool::new(false)),
            client_cwd: std::env::current_dir()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| ".".into()),
            cached_model: Arc::new(std::sync::Mutex::new(None)),
            cached_models: Arc::new(std::sync::Mutex::new(None)),
            cached_commands: Arc::new(std::sync::Mutex::new(None)),
            cached_queue: Arc::new(std::sync::Mutex::new(QueueStats::default())),
            cached_resources: Arc::new(std::sync::Mutex::new(LoadedResourcesSnapshot::default())),
            cached_skills: Arc::new(std::sync::Mutex::new(Vec::new())),
            cached_gate_notice: Arc::new(std::sync::Mutex::new(None)),
            gate_notice_consumed: Arc::new(AtomicBool::new(false)),
            resources_dirty: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Builder-style resilience tuning injection (tests / labs only; product
    /// code uses the [`LinkTunings::default`] constants). MUST be called
    /// before the first attach so the downlink loop is spawned with it.
    pub fn with_tunings(mut self, tunings: LinkTunings) -> Self {
        self.downlink.set_window(tunings.coalesce_window);
        self.tunings = Arc::new(tunings);
        self
    }

    pub fn set_reverse_rpc_notify(&mut self, notify: ReverseRpcNotify) {
        self.reverse_rpc = Some(notify);
    }

    fn cache_queue_from_value(&self, data: &Value) {
        let stats = QueueStats {
            steer_count: data.get("steer_count").and_then(Value::as_u64).unwrap_or(0) as usize,
            follow_up_count: data
                .get("follow_up_count")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
        };
        if let Ok(mut cached) = self.cached_queue.lock() {
            *cached = stats;
        }
    }

    fn cache_model(&self, model: ModelInfo) {
        if let Ok(mut cached) = self.cached_model.lock() {
            *cached = Some(model);
        }
    }

    fn cache_model_unset(&self) {
        if let Ok(mut cached) = self.cached_model.lock() {
            *cached = None;
        }
    }

    /// True for Agent/Turn/stream tape that a cold `last_seq=0` subscribe must not
    /// paint as live (c2307). Queue / error / fixed-zone facts still pass.
    fn is_cold_replay_tape(ev: &XyEvent) -> bool {
        matches!(
            ev,
            XyEvent::AgentStart { .. }
                | XyEvent::AgentEnd { .. }
                | XyEvent::TurnStart { .. }
                | XyEvent::TurnEnd { .. }
                | XyEvent::MessageStart { .. }
                | XyEvent::MessageUpdate { .. }
                | XyEvent::MessageEnd { .. }
                | XyEvent::TextDelta(_)
                | XyEvent::ThinkingDelta(_)
                | XyEvent::ToolExecutionStart { .. }
                | XyEvent::ToolExecutionUpdate { .. }
                | XyEvent::ToolExecutionEnd { .. }
                | XyEvent::CompactionStart { .. }
                | XyEvent::CompactionEnd { .. }
                | XyEvent::AutoRetryStart { .. }
                | XyEvent::AutoRetryEnd { .. }
        )
    }

    fn apply_resources_cache(&self, snap: LoadedResourcesSnapshot) {
        store_resources_snapshot(
            &self.cached_skills,
            &self.gate_notice_consumed,
            &self.cached_gate_notice,
            &self.cached_resources,
            &self.resources_dirty,
            snap,
        );
    }

    fn restart_downlink(&mut self) {
        // c2480: the new generation invalidates the old loop's late pushes.
        self.downlink_gen.fetch_add(1, Ordering::SeqCst);
        self.session_life.cancel();
        self.session_life = CancellationToken::new();
        self.subscribed_ok.store(false, Ordering::SeqCst);
        self.downlink.started.store(false, Ordering::SeqCst);
        self.ensure_downlink();
    }

    fn ensure_downlink(&self)
    where
        C: HostClient + Clone + 'static,
    {
        if self.downlink.started.swap(true, Ordering::SeqCst) {
            return;
        }
        if let Ok(mut slot) = self.fatal.lock() {
            *slot = None;
        }
        let ctx = DownlinkCtx {
            host: self.host.clone(),
            session_id: self.session_id.clone(),
            last_seq: self.last_seq.clone(),
            reverse_rpc: self.reverse_rpc.clone(),
            downlink: self.downlink.clone(),
            life: self.session_life.clone(),
            resync_needed: self.resync_needed.clone(),
            subscribed_ok: self.subscribed_ok.clone(),
            skip_cold_replay: self.skip_cold_replay.clone(),
            cached_queue: self.cached_queue.clone(),
            cached_skills: self.cached_skills.clone(),
            cached_resources: self.cached_resources.clone(),
            cached_gate_notice: self.cached_gate_notice.clone(),
            gate_notice_consumed: self.gate_notice_consumed.clone(),
            resources_dirty: self.resources_dirty.clone(),
            client_cwd: self.client_cwd.clone(),
            downlink_gen: self.downlink_gen.clone(),
            fatal: self.fatal.clone(),
            tunings: self.tunings.clone(),
            my_gen: self.downlink_gen.load(Ordering::SeqCst),
        };
        tokio::spawn(downlink_loop(ctx));
    }

    async fn wait_subscribed(&self) -> Result<(), XyDriverError> {
        let wait_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !self.subscribed_ok.load(Ordering::SeqCst) {
            if let Some(msg) = self.fatal.lock().ok().and_then(|mut slot| slot.take()) {
                return Err(XyDriverError::remote(msg));
            }
            if tokio::time::Instant::now() > wait_deadline {
                return Err(XyDriverError::remote("subscribe timed out"));
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok(())
    }

    async fn refresh_fixed_zone_caches(&self) -> Result<(), XyDriverError> {
        if let Ok(data) = self.unary_cmd(Command::GetState {}).await {
            self.update_leaf_from_state(&data);
            if let Some(m) = data.get("model").filter(|m| !m.is_null())
                && let Ok(model) = Self::model_from_value(m)
            {
                self.cache_model(model);
            } else {
                self.cache_model_unset();
            }
        }
        if let Ok(data) = self.unary_cmd(Command::GetAvailableModels {}).await {
            let arr = data
                .get("models")
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();
            if let Ok(models) = arr
                .iter()
                .map(Self::model_from_value)
                .collect::<Result<Vec<_>, _>>()
                && let Ok(mut cached) = self.cached_models.lock()
            {
                *cached = Some(models);
            }
        }
        if let Ok(data) = self.unary_cmd(Command::GetCommands {}).await {
            let arr = data
                .get("commands")
                .and_then(|c| c.as_array())
                .cloned()
                .unwrap_or_default();
            let cmds = arr
                .iter()
                .filter_map(|c| {
                    Some(CommandInfo {
                        name: c.get("name")?.as_str()?.to_string(),
                        description: c
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .collect();
            if let Ok(mut cached) = self.cached_commands.lock() {
                *cached = Some(cmds);
            }
        }
        let snap = match self.unary_cmd(Command::LoadedResources {}).await {
            Ok(data) => serde_json::from_value(data).unwrap_or_default(),
            Err(_) => LoadedResourcesSnapshot::default(),
        };
        self.apply_resources_cache(snap);
        Ok(())
    }

    fn with_session(&self, mut payload: serde_json::Value) -> serde_json::Value {
        let Some(map) = payload.as_object_mut() else {
            return payload;
        };
        let has_sid = map
            .get("session_id")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty());
        if !has_sid {
            map.insert(
                "session_id".into(),
                serde_json::Value::String(self.session_id.clone()),
            );
        }
        let has_cwd = map
            .get("cwd")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty());
        if !has_cwd {
            map.insert(
                "cwd".into(),
                serde_json::Value::String(self.client_cwd.clone()),
            );
        }
        payload
    }

    fn update_leaf_from_state(&self, data: &Value) {
        let leaf = data
            .get("leaf_entry_id")
            .and_then(Value::as_str)
            .map(str::to_string);
        if let Ok(mut cached) = self.leaf_entry_id.lock() {
            *cached = leaf;
        }
    }

    async fn unary(
        &self,
        method: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, XyDriverError> {
        let result = self
            .host
            .unary(method, self.with_session(payload))
            .await
            .map_err(|e| XyDriverError::remote(e.to_string()))?;
        result
            .into_std()
            .map_err(|e| XyDriverError::remote(format!("{}: {}", e.code, e.details)))
    }

    /// Typed unary send (c2530)：方法名与载荷都从 `Command` 自身派生——
    /// wire tag 就是 serde tag，字符串与手搓载荷无从漂移。
    async fn unary_cmd(&self, cmd: Command) -> Result<serde_json::Value, XyDriverError> {
        let value = serde_json::to_value(&cmd).map_err(|e| XyDriverError::remote(e.to_string()))?;
        let Some(method) = value
            .get("type")
            .and_then(Value::as_str)
            .map(str::to_string)
        else {
            return Err(XyDriverError::remote("command missing wire tag"));
        };
        self.unary(&method, value).await
    }

    fn model_from_value(v: &serde_json::Value) -> Result<ModelInfo, XyDriverError> {
        let thinking = v.get("thinking").and_then(|x| x.as_bool()).unwrap_or(false);
        let thinking_levels = v
            .get("thinking_levels")
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_else(|| vec![THINKING_OFF.into()]);
        Ok(ModelInfo {
            id: v
                .get("id")
                .or_else(|| v.get("model"))
                .and_then(|x| x.as_str())
                .ok_or("missing model id")?
                .to_string(),
            display_name: v
                .get("display_name")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            thinking,
            thinking_levels,
            context_window: v
                .get("context_window")
                .and_then(|x| x.as_u64())
                .unwrap_or(0),
        })
    }
}

impl<C> Drop for XyRemoteDriver<C> {
    fn drop(&mut self) {
        self.session_life.cancel();
    }
}

#[cfg(feature = "server")]
#[async_trait]
impl<C> XyDriver for XyRemoteDriver<C>
where
    C: HostClient + Clone + 'static,
{
    async fn attach_session(&mut self) -> Result<(), XyDriverError> {
        // ath44: the version handshake rides on every mux connection's
        // server_hello (carrier-validated); there is no separate unary
        // describe on the attach critical path.
        self.ensure_downlink();
        self.wait_subscribed().await?;
        self.refresh_fixed_zone_caches().await
    }

    async fn refresh_surface_caches(&mut self) -> Result<(), XyDriverError> {
        self.refresh_fixed_zone_caches().await
    }

    fn drain_idle_events(&mut self) -> Vec<XyEvent> {
        self.downlink.drain()
    }

    /// ath42/c2480: attachment health tracks the mux subscription; fixed-zone
    /// grace UX (never transcript error rows) consumes it.
    fn link_health(&self) -> super::LinkHealth {
        if self.subscribed_ok.load(Ordering::SeqCst) {
            super::LinkHealth::Up
        } else {
            super::LinkHealth::Down
        }
    }

    fn take_resync_rebuild(&mut self) -> bool {
        self.resync_needed.swap(false, Ordering::SeqCst)
    }

    async fn run(&mut self, prompt: &str) -> EventStream {
        self.ensure_downlink();
        if let Err(e) = self.wait_subscribed().await {
            let err = e.to_string();
            let stream = async_stream::stream! {
                yield XyEvent::error_msg(err);
            };
            return Box::pin(stream);
        }
        let turn = {
            let mut g = self.turn_cancel.lock().unwrap_or_else(|e| e.into_inner());
            if g.is_cancelled() {
                *g = CancellationToken::new();
            }
            g.clone()
        };
        let host = self.host.clone();
        let prompt = prompt.to_string();
        let session_id = self.session_id.clone();
        let leaf_entry_id = self.leaf_entry_id.clone();
        let downlink = self.downlink.clone();
        let client_cwd = self.client_cwd.clone();
        let model_id = self.current_model().map(|m| m.id);
        let thinking_level = self
            .thinking
            .lock()
            .ok()
            .map(|t| t.clone())
            .filter(|t| !t.is_empty());

        let stream = async_stream::stream! {
            let mut body = serde_json::json!({
                "message": prompt,
                "session_id": session_id,
                "cwd": client_cwd,
            });
            if let Some(id) = model_id {
                body["model_id"] = serde_json::Value::String(id);
            }
            if let Some(level) = thinking_level {
                body["thinking_level"] = serde_json::Value::String(level);
            }
            if let Err(e) = host.unary("prompt", body).await {
                yield XyEvent::error_msg(format!("prompt failed: {e}"));
                return;
            }

            loop {
                let batch = downlink.drain();
                if batch.is_empty() {
                    tokio::select! {
                        _ = turn.cancelled() => {
                            let _ = host.unary("abort", serde_json::json!({
                                "session_id": session_id,
                            })).await;
                            break;
                        }
                        _ = downlink.notify.notified() => {}
                    }
                    continue;
                }
                let mut ended = false;
                let mut rest = Vec::new();
                for ev in batch {
                    if ended {
                        rest.push(ev);
                        continue;
                    }
                    let is_end = matches!(ev, XyEvent::AgentEnd { .. });
                    if is_end
                        && let Ok(result) = host
                            .unary(
                                "get_state",
                                serde_json::json!({
                                    "session_id": session_id,
                                }),
                            )
                            .await
                        && let Some(leaf) = result
                            .value
                            .as_ref()
                            .and_then(|value| value.get("leaf_entry_id"))
                            .and_then(Value::as_str)
                        && let Ok(mut cached) = leaf_entry_id.lock()
                    {
                        *cached = Some(leaf.to_string());
                    }
                    yield ev;
                    if is_end {
                        ended = true;
                    }
                }
                downlink.push_front_batch(rest);
                if ended {
                    break;
                }
            }
        };

        Box::pin(stream)
    }

    fn abort(&self) {
        if let Ok(g) = self.turn_cancel.lock() {
            g.cancel();
        }
        let host = self.host.clone();
        let payload = self.with_session(serde_json::json!({}));
        tokio::spawn(async move {
            let _ = host.unary("abort", payload).await;
        });
    }

    fn current_model(&self) -> Option<ModelInfo> {
        // Product TUI ticks / footer sync call this synchronously. HTTP `block_on`
        // here freezes input (and `/model`) while MCP or Host unary is in flight.
        self.cached_model
            .lock()
            .ok()
            .and_then(|cached| cached.clone())
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        self.cached_models
            .lock()
            .ok()
            .and_then(|cached| cached.clone())
            .unwrap_or_default()
    }

    async fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, XyDriverError> {
        let data = self
            .unary_cmd(Command::SetModel {
                provider: String::new(),
                model_id: model_id.to_string(),
            })
            .await?;
        // Endpoint returns { model, display_name }; enrich via list if needed.
        let selected = if data.get("id").is_some() {
            Self::model_from_value(&data)?
        } else {
            ModelInfo {
                id: data
                    .get("model")
                    .and_then(|m| m.as_str())
                    .unwrap_or(model_id)
                    .to_string(),
                display_name: data
                    .get("display_name")
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string(),
                thinking: false,
                thinking_levels: Vec::new(),
                context_window: 0,
            }
        };
        let default = selected
            .thinking_levels
            .last()
            .cloned()
            .unwrap_or_else(|| THINKING_OFF.into());
        *self.thinking.lock().unwrap() = default;
        self.cache_model(selected.clone());
        Ok(selected)
    }

    async fn cycle_model(&mut self) -> Result<ModelInfo, XyDriverError> {
        let data = self.unary_cmd(Command::CycleModel {}).await?;
        let selected = Self::model_from_value(&data)?;
        *self.thinking.lock().unwrap() = selected
            .thinking_levels
            .last()
            .cloned()
            .unwrap_or_else(|| THINKING_OFF.into());
        self.cache_model(selected.clone());
        Ok(selected)
    }

    async fn set_thinking_level(&mut self, level: String) -> Result<(), XyDriverError> {
        self.unary_cmd(Command::SetThinkingLevel {
            level: level.clone(),
        })
        .await?;
        *self.thinking.lock().unwrap() = level;
        Ok(())
    }

    fn thinking_level(&self) -> String {
        self.thinking.lock().unwrap().clone()
    }

    async fn cycle_thinking_level(&mut self) -> Result<String, XyDriverError> {
        // Remote REST has set-only; cycle locally over the selected model's
        // declared support list.
        let levels = self
            .current_model()
            .map(|model| model.thinking_levels)
            .filter(|levels| !levels.is_empty())
            .unwrap_or_else(|| vec![THINKING_OFF.into()]);
        let cur = self.thinking_level();
        let next = match levels.iter().position(|level| level == &cur) {
            Some(index) => levels[(index + 1) % levels.len()].clone(),
            None => levels
                .last()
                .cloned()
                .unwrap_or_else(|| THINKING_OFF.into()),
        };
        self.set_thinking_level(next.clone()).await?;
        Ok(next)
    }

    fn session_id(&self) -> Option<String> {
        Some(self.session_id.clone())
    }

    async fn execute_bash(
        &self,
        command: &str,
        exclude_from_context: bool,
        _chunk_tx: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,
    ) -> Result<XyBashResult, XyDriverError> {
        // Remote REST bash is request/response — no live chunk uplink.
        let data = self
            .unary_cmd(Command::Bash {
                command: command.to_string(),
                exclude_from_context,
            })
            .await?;
        Ok(XyBashResult {
            output: data
                .get("output")
                .and_then(|o| o.as_str())
                .unwrap_or("")
                .to_string(),
            exit_code: data
                .get("exit_code")
                .and_then(|c| c.as_i64())
                .map(|c| c as i32),
            cancelled: data
                .get("cancelled")
                .and_then(|c| c.as_bool())
                .unwrap_or(false),
            timed_out: data
                .get("timed_out")
                .and_then(|c| c.as_bool())
                .unwrap_or(false),
            truncated: data
                .get("truncated")
                .and_then(|c| c.as_bool())
                .unwrap_or(false),
            full_output_path: None,
        })
    }

    async fn compact(&mut self, instructions: Option<String>) -> Result<bool, XyDriverError> {
        let data = self.unary_cmd(Command::Compact { instructions }).await?;
        Ok(data
            .get("compacted")
            .and_then(|c| c.as_bool())
            .unwrap_or(false))
    }

    async fn export_html(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let data = self
            .unary_cmd(Command::ExportHtml { output_path: None })
            .await?;
        if let Some(content) = data.get("content").and_then(|c| c.as_str()) {
            std::fs::write(path, content).map_err(|e| XyDriverError::io(e.to_string()))?;
            return Ok(path.to_string_lossy().into_owned());
        }
        Ok(data
            .get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string())
    }

    async fn export_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let data = self
            .unary_cmd(Command::ExportJsonl { output_path: None })
            .await?;
        if let Some(content) = data.get("content").and_then(|c| c.as_str()) {
            std::fs::write(path, content).map_err(|e| XyDriverError::io(e.to_string()))?;
            return Ok(path.to_string_lossy().into_owned());
        }
        Ok(data
            .get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string())
    }

    async fn import_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| XyDriverError::io(e.to_string()))?;
        let data = self
            .unary("import_jsonl", serde_json::json!({ "content": content }))
            .await?;
        Ok(data
            .get("session_id")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string())
    }

    async fn fork_session(
        &mut self,
        entry_id: &str,
        position: crate::protocol::session::ForkPosition,
    ) -> Result<String, XyDriverError> {
        let data = self
            .unary_cmd(Command::Fork {
                entry_id: entry_id.to_string(),
                position: Some(
                    match position {
                        crate::protocol::session::ForkPosition::At => "at",
                        crate::protocol::session::ForkPosition::Before => "before",
                    }
                    .to_string(),
                ),
            })
            .await?;
        Ok(data
            .get("session_id")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string())
    }

    async fn switch_session(&mut self, session_id: &str) -> Result<String, XyDriverError> {
        let data = self
            .unary_cmd(Command::SwitchSession {
                session_path: session_id.to_string(),
            })
            .await?;
        let id = data
            .get("session_id")
            .and_then(|p| p.as_str())
            .unwrap_or(session_id)
            .to_string();
        self.session_id = id.clone();
        if let Ok(mut leaf) = self.leaf_entry_id.lock() {
            *leaf = None;
        }
        if self.downlink.started.load(Ordering::SeqCst) {
            self.restart_downlink();
        }
        Ok(id)
    }

    async fn get_messages(&self) -> Result<Vec<SessionEntry>, XyDriverError> {
        let data = self.unary_cmd(Command::GetMessages {}).await?;
        let entries = data
            .get("entries")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        serde_json::from_value(entries).map_err(|e| XyDriverError::remote(e.to_string()))
    }

    async fn get_session_stats(&self) -> Result<SessionStats, XyDriverError> {
        let data = self.unary_cmd(Command::GetSessionStats {}).await?;
        Ok(SessionStats {
            session_id: data
                .get("session_id")
                .and_then(|s| s.as_str())
                .unwrap_or(&self.session_id)
                .to_string(),
            user_messages: data
                .get("user_messages")
                .and_then(|n| n.as_u64())
                .unwrap_or(0) as usize,
            assistant_messages: data
                .get("assistant_messages")
                .and_then(|n| n.as_u64())
                .unwrap_or(0) as usize,
            total_messages: data
                .get("total_messages")
                .and_then(|n| n.as_u64())
                .unwrap_or(0) as usize,
            thinking_level: data
                .get("thinking_level")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            model: data.get("model").and_then(|m| {
                Some((
                    m.get("provider")?.as_str()?.to_string(),
                    m.get("model_id")?.as_str()?.to_string(),
                ))
            }),
        })
    }

    async fn estimate_context_tokens(
        &self,
    ) -> Result<crate::protocol::model::ContextTokenEstimate, XyDriverError> {
        let entries = self.get_messages().await.unwrap_or_default();
        // Remote surface: tokenizer mapping lives on the server; do not inject
        // local AppConfig override here.
        Ok(estimate_from_session_entries(
            &entries,
            self.current_model().map(|m| m.id),
            None,
        ))
    }

    fn get_commands(&self) -> Vec<CommandInfo> {
        self.cached_commands
            .lock()
            .ok()
            .and_then(|cached| cached.clone())
            .unwrap_or_default()
    }

    async fn steer(&mut self, message: &str) -> Result<(), XyDriverError> {
        let data = self
            .unary_cmd(Command::Steer {
                message: message.to_string(),
            })
            .await?;
        self.cache_queue_from_value(&data);
        Ok(())
    }

    async fn follow_up(&mut self, message: &str) -> Result<(), XyDriverError> {
        let data = self
            .unary_cmd(Command::FollowUp {
                message: message.to_string(),
            })
            .await?;
        self.cache_queue_from_value(&data);
        Ok(())
    }

    async fn clear_queue(
        &mut self,
        clear_steer: bool,
        clear_follow_up: bool,
    ) -> Result<(), XyDriverError> {
        let data = self
            .unary_cmd(Command::ClearQueue {
                clear_steer,
                clear_follow_up,
            })
            .await?;
        self.cache_queue_from_value(&data);
        Ok(())
    }

    fn queue_stats(&self) -> QueueStats {
        self.cached_queue.lock().map(|s| *s).unwrap_or_default()
    }

    async fn session_tree(
        &self,
        kind: SessionTreeKind,
    ) -> Result<Vec<SessionTreeNode>, XyDriverError> {
        let data = self.unary_cmd(Command::SessionTree { kind }).await?;
        serde_json::from_value(data.get("tree").cloned().unwrap_or(Value::Null))
            .map_err(|e| XyDriverError::remote(e.to_string()))
    }

    async fn travel_session_tree(
        &self,
        kind: SessionTreeKind,
        entry_id: &str,
    ) -> Result<SessionTreeTravel, XyDriverError> {
        let data = self
            .unary_cmd(Command::TravelSessionTree {
                kind,
                entry_id: entry_id.to_string(),
            })
            .await?;
        serde_json::from_value(data).map_err(|e| XyDriverError::remote(e.to_string()))
    }

    async fn append_entry_label(
        &mut self,
        target_id: &str,
        label: Option<&str>,
    ) -> Result<(), XyDriverError> {
        self.unary_cmd(Command::AppendEntryLabel {
            target_id: target_id.to_string(),
            label: label.map(str::to_string),
        })
        .await?;
        Ok(())
    }

    fn leaf_entry_id(&self) -> Option<String> {
        self.leaf_entry_id.lock().ok().and_then(|leaf| leaf.clone())
    }

    async fn load_debug_scene(&mut self, scene: &str) -> Result<DebugSceneLoad, XyDriverError> {
        let data = self
            .unary("load_debug_scene", serde_json::json!({ "scene": scene }))
            .await?;
        let session_id = data
            .get("session_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let entries = serde_json::from_value(data.get("entries").cloned().unwrap_or(Value::Null))
            .map_err(|e| XyDriverError::remote(e.to_string()))?;
        let note = data
            .get("note")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let model = data
            .get("model")
            .filter(|m| !m.is_null())
            .and_then(|m| Self::model_from_value(m).ok());
        if let Some(m) = model.as_ref() {
            self.cache_model(m.clone());
        }
        Ok(DebugSceneLoad {
            session_id,
            entries,
            note,
            model,
        })
    }

    async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XyDriverError> {
        let data = self.unary_cmd(Command::ListSessions {}).await?;
        serde_json::from_value(data.get("sessions").cloned().unwrap_or(Value::Null))
            .map_err(|e| XyDriverError::remote(e.to_string()))
    }

    async fn load_session_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XyDriverError> {
        let data = self
            .unary_cmd(Command::LoadSessionEntries {
                session_id: session_id.to_string(),
            })
            .await?;
        serde_json::from_value(data.get("entries").cloned().unwrap_or(Value::Null))
            .map_err(|e| XyDriverError::remote(e.to_string()))
    }

    async fn new_session(&mut self) -> Result<String, XyDriverError> {
        let data = self.unary_cmd(Command::NewSession {}).await?;
        let id = data
            .get("session_id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| XyDriverError::remote("new_session response missing session_id"))?
            .to_string();
        self.session_id = id.clone();
        if let Ok(mut leaf) = self.leaf_entry_id.lock() {
            *leaf = None;
        }
        if self.downlink.started.load(Ordering::SeqCst) {
            self.restart_downlink();
        }
        Ok(id)
    }

    async fn get_session_name(&self) -> Result<Option<String>, XyDriverError> {
        let data = self.unary_cmd(Command::GetSessionName {}).await?;
        Ok(data
            .get("name")
            .and_then(|value| value.as_str())
            .map(str::to_string))
    }

    async fn set_session_name(&mut self, name: &str) -> Result<String, XyDriverError> {
        let data = self
            .unary_cmd(Command::SetSessionName {
                name: name.to_string(),
            })
            .await?;
        data.get("name")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .ok_or_else(|| XyDriverError::remote("set_session_name response missing name"))
    }

    async fn set_session_name_for(
        &mut self,
        session_id: &str,
        name: &str,
    ) -> Result<String, XyDriverError> {
        let data = self
            .unary_cmd(Command::SetSessionNameFor {
                session_id: session_id.to_string(),
                name: name.to_string(),
            })
            .await?;
        data.get("name")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .ok_or_else(|| XyDriverError::remote("set_session_name_for response missing name"))
    }

    async fn delete_session(&mut self, session_id: &str) -> Result<(), XyDriverError> {
        self.unary_cmd(Command::DeleteSession {
            session_id: session_id.to_string(),
        })
        .await?;
        Ok(())
    }

    async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
        let snap = match self.unary_cmd(Command::LoadedResources {}).await {
            Ok(data) => serde_json::from_value(data).unwrap_or_else(|e| LoadedResourcesSnapshot {
                mcp_diag_short: vec![format!("remote loaded_resources decode: {e}")],
                ..LoadedResourcesSnapshot::default()
            }),
            Err(e) => LoadedResourcesSnapshot {
                mcp_diag_short: vec![format!("remote loaded_resources: {e}")],
                ..LoadedResourcesSnapshot::default()
            },
        };
        self.apply_resources_cache(snap.clone());
        snap
    }

    fn loaded_resources_cached(&self) -> Option<LoadedResourcesSnapshot> {
        self.cached_resources.lock().ok().map(|snap| snap.clone())
    }

    fn mcp_blocks_agent(&self) -> bool {
        self.cached_resources
            .lock()
            .map(|snap| snap.mcp_configured > 0 && !snap.mcp_bootstrap_complete)
            .unwrap_or(false)
    }

    fn is_tools_frozen(&self) -> bool {
        self.cached_resources
            .lock()
            .map(|snap| snap.mcp_configured == 0 || snap.tools_table_frozen)
            .unwrap_or(true)
    }

    fn dollar_skill_catalog(&self) -> Vec<(String, String)> {
        self.cached_skills
            .lock()
            .map(|skills| skills.clone())
            .unwrap_or_default()
    }

    fn take_mcp_gate_notice(&mut self) -> Option<String> {
        self.gate_notice_consumed.store(true, Ordering::SeqCst);
        self.cached_gate_notice
            .lock()
            .ok()
            .and_then(|mut notice| notice.take())
    }

    async fn arm_tool_freeze_gate(&mut self) {
        let data = match self.unary("arm_tool_freeze", serde_json::json!({})).await {
            Ok(data) => data,
            Err(e) => {
                e.log_failure("remote.arm_tool_freeze");
                return;
            }
        };
        if let Ok(snap) = serde_json::from_value::<LoadedResourcesSnapshot>(data) {
            self.apply_resources_cache(snap);
        }
    }

    async fn poll_mcp_bootstrap(&mut self) -> bool {
        // Attach observes writer MCP via mux `session/resources`, not 16Hz unary.
        self.resources_dirty.swap(false, Ordering::SeqCst)
    }

    async fn reload_runtime(
        &mut self,
        cancel: &CancellationToken,
    ) -> Result<RuntimeReloadReport, XyDriverError> {
        self.gate_notice_consumed.store(false, Ordering::SeqCst);
        // ath37 / sr-abort1: honour the injected cancel token. On cancel, ask the
        // Host to cooperatively cancel the in-flight process-level reload via the
        // existing `abort` unary, then finish locally as cancelled instead of
        // waiting for the original call.
        let data = {
            let fut = self.unary("reload", serde_json::json!({}));
            tokio::pin!(fut);
            tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    let _ = self.unary("abort", serde_json::json!({})).await;
                    return Ok(RuntimeReloadReport {
                        steps: Vec::new(),
                        cancelled: true,
                    });
                }
                data = &mut fut => data?,
            }
        };
        let cancelled = data
            .get("cancelled")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let steps = data
            .get("steps")
            .and_then(|value| value.as_array())
            .map(|values| {
                values
                    .iter()
                    .map(|value| ReloadStepReport {
                        step: match value.get("step").and_then(|v| v.as_str()) {
                            Some("skills") => "skills",
                            Some("mcp") => "mcp",
                            Some("context") => "context",
                            _ => "runtime",
                        },
                        ok: value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false),
                        message: value
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(RuntimeReloadReport { steps, cancelled })
    }

    /// Client-local clipboard semantics live in `super::clipboard` (shared
    /// with the in-process driver; MUST NOT route through the Host, D1).
    async fn copy_text_to_clipboard(
        &mut self,
        text: &str,
    ) -> Result<ClipboardCopyOutcome, XyDriverError> {
        super::clipboard::copy_text_to_clipboard(text.to_string()).await
    }

    async fn stage_clipboard_image(&mut self) -> Result<Option<std::path::PathBuf>, XyDriverError> {
        super::clipboard::stage_clipboard_image().await
    }

    async fn read_clipboard_text(&mut self) -> Result<Option<String>, XyDriverError> {
        super::clipboard::read_clipboard_text().await
    }

    /// Trust is a Host fact (gate + persistence live with the writer), so this
    /// routes through a Host unary scoped to the session workspace — the TUI
    /// MUST NOT write the Host's agent dir itself (D2).
    async fn persist_project_trust(
        &mut self,
        mode: ProjectTrustMode,
    ) -> Result<ProjectTrustPersistReport, XyDriverError> {
        let mode_str = match mode {
            ProjectTrustMode::TrustCwd => "trust_cwd",
            ProjectTrustMode::TrustParent => "trust_parent",
            ProjectTrustMode::Deny => "deny",
        };
        let data = self
            .unary("persist_trust", serde_json::json!({ "mode": mode_str }))
            .await?;
        Ok(ProjectTrustPersistReport {
            trusted: data
                .get("trusted")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            saved_path: data
                .get("saved_path")
                .and_then(Value::as_str)
                .map(str::to_string),
            message: data
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::core::host_client::InProcessClient;
    use crate::app::server::host::HostState;
    use crate::app::server::runtime::{ServerConfig, serve};

    #[test]
    fn empty_session_arg_mints_uuid() {
        let driver = XyRemoteDriver::new("http://127.0.0.1:9", "");
        let sid = driver.session_id().expect("session");
        assert!(uuid::Uuid::parse_str(&sid).is_ok(), "{sid}");
        let kept = XyRemoteDriver::new("http://127.0.0.1:9", "keep-me");
        assert_eq!(kept.session_id().as_deref(), Some("keep-me"));
    }

    #[derive(Clone)]
    struct SnapClient {
        calls: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait]
    impl HostClient for SnapClient {
        async fn unary(
            &self,
            _method: &str,
            _payload: Value,
        ) -> Result<crate::protocol::RpcResult, crate::app::core::host_client::HostClientError>
        {
            self.calls.fetch_add(1, Ordering::SeqCst);
            panic!("poll_mcp_bootstrap must not unary");
        }

        async fn respond(
            &self,
            _rpc_id: &str,
            _payload: Value,
        ) -> Result<(), crate::app::core::host_client::HostClientError> {
            Ok(())
        }

        async fn mux(
            &self,
        ) -> Result<
            crate::app::core::host_client::MuxStream,
            crate::app::core::host_client::HostClientError,
        > {
            Ok(Box::pin(futures::stream::empty()))
        }
    }

    #[tokio::test]
    async fn poll_mcp_bootstrap_is_cache_dirty_only() {
        let client = SnapClient {
            calls: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        };
        let mut driver = XyRemoteDriver::with_host(client.clone(), "s");
        assert!(!driver.poll_mcp_bootstrap().await);
        assert_eq!(client.calls.load(Ordering::SeqCst), 0);
        driver.apply_resources_cache(LoadedResourcesSnapshot {
            mcp_configured: 1,
            mcp_bootstrap_complete: false,
            tools_table_frozen: false,
            mcp_connecting_label: Some("connecting 0/1".into()),
            ..LoadedResourcesSnapshot::default()
        });
        assert!(driver.poll_mcp_bootstrap().await);
        assert!(!driver.poll_mcp_bootstrap().await);
        assert_eq!(client.calls.load(Ordering::SeqCst), 0);
    }

    /// ath37 / sr-abort1 (c2340): cancelling `reload_runtime` mid-flight MUST
    /// ask the Host to cooperatively cancel the in-flight reload via the
    /// existing `abort` unary and finish locally as cancelled.
    #[derive(Clone)]
    struct ReloadCancelClient {
        abort_called: Arc<std::sync::atomic::AtomicBool>,
        release: Arc<tokio::sync::Notify>,
    }

    #[async_trait]
    impl HostClient for ReloadCancelClient {
        async fn unary(
            &self,
            method: &str,
            _payload: Value,
        ) -> Result<crate::protocol::RpcResult, crate::app::core::host_client::HostClientError>
        {
            if method == "reload" {
                // Hold the reload open until the cooperative abort arrives.
                let release = self.release.clone();
                let _ = tokio::select! {
                    _ = release.notified() => {}
                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
                };
                return Ok(crate::protocol::RpcResult::ok_value(
                    serde_json::json!({ "cancelled": true, "steps": [] }),
                ));
            }
            if method == "abort" {
                self.abort_called.store(true, Ordering::SeqCst);
                self.release.notify_waiters();
                return Ok(crate::protocol::RpcResult::ok_value(
                    serde_json::json!({ "cancelled": true }),
                ));
            }
            Err(crate::app::core::host_client::HostClientError::Transport(
                format!("unexpected unary: {method}"),
            ))
        }

        async fn respond(
            &self,
            _rpc_id: &str,
            _payload: Value,
        ) -> Result<(), crate::app::core::host_client::HostClientError> {
            Ok(())
        }

        async fn mux(
            &self,
        ) -> Result<
            crate::app::core::host_client::MuxStream,
            crate::app::core::host_client::HostClientError,
        > {
            Ok(Box::pin(futures::stream::empty()))
        }
    }

    #[tokio::test]
    async fn reload_cancel_requests_host_cooperative_abort() {
        use std::sync::atomic::AtomicBool;

        let client = ReloadCancelClient {
            abort_called: Arc::new(AtomicBool::new(false)),
            release: Arc::new(tokio::sync::Notify::new()),
        };
        let mut driver = XyRemoteDriver::with_host(client.clone(), "s");
        let token = CancellationToken::new();
        let cancel_token = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            cancel_token.cancel();
        });

        let report = driver.reload_runtime(&token).await.expect("reload report");
        assert!(
            report.cancelled,
            "report MUST be cancelled after token fires"
        );
        assert!(
            client.abort_called.load(Ordering::SeqCst),
            "cancel MUST reach the Host as a cooperative abort unary"
        );
    }

    #[derive(Clone, Default)]
    struct CountingClient {
        unaries: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait]
    impl HostClient for CountingClient {
        async fn unary(
            &self,
            _method: &str,
            _payload: Value,
        ) -> Result<crate::protocol::RpcResult, crate::app::core::host_client::HostClientError>
        {
            self.unaries.fetch_add(1, Ordering::SeqCst);
            Ok(crate::protocol::RpcResult::error(
                "not_expected",
                "clipboard MUST NOT unary the host",
            ))
        }

        async fn respond(
            &self,
            _rpc_id: &str,
            _payload: Value,
        ) -> Result<(), crate::app::core::host_client::HostClientError> {
            Ok(())
        }

        async fn mux(
            &self,
        ) -> Result<
            crate::app::core::host_client::MuxStream,
            crate::app::core::host_client::HostClientError,
        > {
            Ok(Box::pin(futures::stream::empty()))
        }
    }

    /// Empty PATH so `plan_clipboard_copy` cannot spawn wl-copy/xclip/pbcopy.
    /// Restored on drop. `env_global` covers the cargo-test in-process fallback
    /// (nextest is already process-per-test).
    struct StarveNativeClipboard {
        saved_path: Option<std::ffi::OsString>,
    }

    impl StarveNativeClipboard {
        fn enter() -> Self {
            let saved_path = std::env::var_os("PATH");
            unsafe {
                std::env::set_var("PATH", "");
            }
            Self { saved_path }
        }
    }

    impl Drop for StarveNativeClipboard {
        fn drop(&mut self) {
            unsafe {
                match self.saved_path.take() {
                    Some(path) => std::env::set_var("PATH", path),
                    None => std::env::remove_var("PATH"),
                }
            }
        }
    }

    // env_global: this test clears PATH so native clipboard tools are not
    // spawned (would clobber the developer clipboard). Elimination path:
    // inject get_env into Driver clipboard helpers.
    #[tokio::test]
    #[serial_test::serial(env_global)]
    async fn clipboard_ops_stay_client_local() {
        let _starve = StarveNativeClipboard::enter();
        let client = CountingClient::default();
        let mut driver = XyRemoteDriver::with_host(client.clone(), "s");

        let outcome = driver
            .copy_text_to_clipboard("copy-me")
            .await
            .expect("OSC52 fallback MUST make copy succeed without native tools");
        assert!(
            outcome.pending_osc52.is_some(),
            "empty PATH MUST take OSC52 fallback, not native copy"
        );
        let _ = driver.stage_clipboard_image().await;
        let _ = driver.read_clipboard_text().await;
        assert_eq!(
            client.unaries.load(Ordering::SeqCst),
            0,
            "clipboard ops MUST NOT unary the host (D1 client-local)"
        );
    }

    #[tokio::test]
    async fn push_resources_is_not_journaled() {
        let host = HostState::for_test().expect("host");
        let slot = host.slot("res").await;
        let mut rx = host.in_process_downlink.subscribe();
        slot.push_resources(LoadedResourcesSnapshot {
            mcp_configured: 1,
            mcp_connecting_label: Some("connecting 0/1".into()),
            ..LoadedResourcesSnapshot::default()
        })
        .await;
        let msg = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("downlink")
            .expect("msg");
        match msg {
            RpcMessage::ServerRequest {
                method, payload, ..
            } => {
                assert_eq!(method, "session/resources");
                assert_eq!(payload["session_id"], "res");
                assert_eq!(payload["snapshot"]["mcp_configured"], 1);
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(
            slot.journal.lock().await.max_seq(),
            0,
            "session/resources MUST NOT consume journal seq"
        );
    }

    #[tokio::test]
    async fn minted_session_subscribe_does_not_require_cli_flag() {
        let host = HostState::for_test().expect("host");
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
        .expect("bind");
        let driver = XyRemoteDriver::new(format!("http://127.0.0.1:{port}"), "");
        let result = driver
            .unary("subscribe", serde_json::json!({ "last_seq": 0 }))
            .await;
        running.shutdown();
        result.expect("subscribe must accept the minted session_id");
    }

    async fn exercise_session_and_host_capabilities<C>(mut driver: XyRemoteDriver<C>)
    where
        C: HostClient + Clone + 'static,
    {
        let session_id = driver.new_session().await.expect("new session");
        let listed = driver.list_sessions().await.expect("list sessions");
        assert!(listed.iter().any(|entry| entry.id == session_id));

        let stored_name = driver
            .set_session_name(" remote name\n")
            .await
            .expect("set name");
        assert_eq!(stored_name, "remote name");
        assert_eq!(
            driver
                .get_session_name()
                .await
                .expect("get name")
                .as_deref(),
            Some("remote name")
        );
        assert_eq!(
            driver
                .load_session_entries(&session_id)
                .await
                .expect("load entries")
                .len(),
            2
        );
        let tree = driver
            .session_tree(SessionTreeKind::MessageHistory)
            .await
            .expect("session tree");
        driver
            .refresh_surface_caches()
            .await
            .expect("refresh fixed zone caches");
        let _ = driver.current_model();
        assert!(driver.leaf_entry_id().is_some());
        let target_id = tree
            .first()
            .and_then(|node| node.entry.entry_id())
            .map(str::to_string)
            .expect("session tree entry");
        driver
            .append_entry_label(&target_id, Some("important"))
            .await
            .expect("append label");
        let labelled_tree = driver
            .session_tree(SessionTreeKind::MessageHistory)
            .await
            .expect("labelled tree");
        assert_eq!(
            labelled_tree.first().and_then(|node| node.label.as_deref()),
            Some("important")
        );
        let travel = driver
            .travel_session_tree(SessionTreeKind::MessageHistory, &target_id)
            .await
            .expect("travel");
        assert_eq!(travel.selected_id, target_id);
        assert_eq!(
            driver
                .set_session_name_for(&session_id, "named for session")
                .await
                .expect("set name for"),
            "named for session"
        );
        assert_eq!(
            driver
                .get_session_name()
                .await
                .expect("get name")
                .as_deref(),
            Some("named for session")
        );

        let snapshot = driver.loaded_resources_snapshot().await;
        assert!(snapshot.mcp_diag_short.is_empty());
        driver.arm_tool_freeze_gate().await;
        assert!(
            driver.is_tools_frozen(),
            "arm_tool_freeze MUST freeze when no MCP is configured"
        );
        driver.steer("nudge").await.expect("steer");
        assert_eq!(driver.queue_stats().steer_count, 1);
        let queued = driver
            .unary("queue_stats", serde_json::json!({}))
            .await
            .expect("queue_stats unary");
        assert_eq!(queued.get("steer_count").and_then(Value::as_u64), Some(1));
        driver.clear_queue(true, true).await.expect("clear queue");
        let report = driver
            .reload_runtime(&CancellationToken::new())
            .await
            .expect("reload");
        assert!(!report.steps.is_empty());
        driver
            .delete_session(&session_id)
            .await
            .expect("delete session");
        assert!(
            !driver
                .list_sessions()
                .await
                .expect("list after delete")
                .iter()
                .any(|entry| entry.id == session_id)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn session_and_host_capabilities_roundtrip_over_host() {
        let host = HostState::for_test().expect("host");
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
        .expect("bind");
        exercise_session_and_host_capabilities(XyRemoteDriver::new(
            format!("http://127.0.0.1:{port}"),
            "remote-seed",
        ))
        .await;
        running.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn session_and_host_capabilities_match_in_process_carrier() {
        let host = HostState::for_test().expect("host");
        let client = InProcessClient::host_state(host);
        exercise_session_and_host_capabilities(XyRemoteDriver::with_host(
            client,
            "in-process-seed",
        ))
        .await;
    }

    fn fixture_mcp(name: &str) -> crate::app::core::mcp_spec::McpServerSpec {
        use crate::app::core::mcp_spec::{McpServerSpec, McpTransportSpec};
        use std::collections::HashMap;

        let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/support/mcp_fixture_server.py");
        let mut env = HashMap::new();
        env.insert("XYLITOL_MCP_FIXTURE_TOOLS".into(), "ping".into());
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

    #[tokio::test]
    async fn loaded_resources_reads_writer_mcp_not_hollow_shell() {
        use crate::app::server::host::materialize_writer;

        let host =
            HostState::for_test_with_mcp(vec![fixture_mcp("a"), fixture_mcp("b")]).expect("host");
        let slot = host.slot("mcp-sess").await;
        materialize_writer(&host, &slot)
            .await
            .expect("materialize writer");
        {
            let mut guard = slot.driver.lock().await;
            let driver = guard.as_mut().expect("writer");
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
            while std::time::Instant::now() < deadline {
                let _ = driver.poll_mcp_bootstrap().await;
                if !driver.mcp_blocks_agent() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
        let snap = host.loaded_resources_snapshot().await;
        assert_eq!(snap.mcp_configured, 2);
        assert!(
            !snap.mcp_connected.is_empty()
                || !snap.mcp_diag_short.is_empty()
                || snap.mcp_connecting_label.is_some(),
            "loaded_resources MUST NOT stay at configured-only hollow shell: {snap:?}"
        );
        if snap.mcp_bootstrap_complete {
            assert!(
                !snap.mcp_connected.is_empty() || !snap.mcp_diag_short.is_empty(),
                "settled snapshot MUST NOT be fake-complete 0 connected: {snap:?}"
            );
        }
    }

    #[tokio::test]
    async fn loaded_resources_snapshot_applies_writer_mcp_without_direct_poll() {
        use crate::app::server::host::materialize_writer;

        let host =
            HostState::for_test_with_mcp(vec![fixture_mcp("a"), fixture_mcp("b")]).expect("host");
        let slot = host.slot("mcp-attach").await;
        materialize_writer(&host, &slot)
            .await
            .expect("materialize writer");
        // Attach TUI only hits `loaded_resources` unary — never `XyDriver::poll_mcp`
        // on the writer. Snapshot MUST join/install so the header can leave
        // `2 configured · 0 connected`.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let mut snap = host.loaded_resources_snapshot().await;
        while std::time::Instant::now() < deadline {
            if !snap.mcp_connected.is_empty() || !snap.mcp_diag_short.is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            snap = host.loaded_resources_snapshot().await;
        }
        assert_eq!(snap.mcp_configured, 2);
        assert!(
            !snap.mcp_connected.is_empty() || !snap.mcp_diag_short.is_empty(),
            "attach-path snapshot MUST apply MCP without a direct writer poll: {snap:?}"
        );
        if snap.mcp_bootstrap_complete {
            assert!(
                !snap.mcp_connected.is_empty() || !snap.mcp_diag_short.is_empty(),
                "settled attach snapshot MUST NOT be 0 connected: {snap:?}"
            );
        }
    }

    #[tokio::test]
    async fn arm_tool_freeze_unary_freezes_after_mcp_settle() {
        use crate::app::server::host::materialize_writer;

        let host =
            HostState::for_test_with_mcp(vec![fixture_mcp("a"), fixture_mcp("b")]).expect("host");
        let slot = host.slot("mcp-freeze").await;
        materialize_writer(&host, &slot)
            .await
            .expect("materialize writer");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let mut snap = host.loaded_resources_snapshot().await;
        while std::time::Instant::now() < deadline {
            if snap.mcp_bootstrap_complete
                && (!snap.mcp_connected.is_empty() || !snap.mcp_diag_short.is_empty())
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            snap = host.loaded_resources_snapshot().await;
        }
        assert!(
            !snap.tools_table_frozen,
            "settle MUST NOT freeze until first-turn gate: {snap:?}"
        );
        let result = crate::app::server::host::handle_unary(
            &host,
            None,
            "arm_tool_freeze",
            serde_json::json!({ "session_id": "mcp-freeze" }),
            None,
        )
        .await;
        assert!(result.ok, "arm_tool_freeze unary failed: {result:?}");
        let snap: LoadedResourcesSnapshot =
            serde_json::from_value(result.value.expect("snapshot")).expect("decode snapshot");
        assert!(
            snap.tools_table_frozen,
            "first-turn arm MUST freeze after MCP settle: {snap:?}"
        );
    }

    #[tokio::test]
    async fn loaded_resources_before_writer_is_not_fake_complete() {
        let host = HostState::for_test_with_mcp(vec![fixture_mcp("pre")]).expect("host");
        let snap = host.loaded_resources_snapshot().await;
        assert_eq!(snap.mcp_configured, 1);
        assert!(
            !snap.mcp_bootstrap_complete
                || snap.mcp_connecting_label.is_some()
                || !snap.mcp_connected.is_empty()
                || !snap.mcp_diag_short.is_empty(),
            "pre-subscribe snapshot MUST NOT be Idle-complete 0 connected: {snap:?}"
        );
    }

    #[tokio::test]
    async fn materialize_writer_does_not_wait_mcp_bootstrap() {
        use crate::app::core::mcp_spec::{McpServerSpec, McpTransportSpec};
        use crate::app::server::host::materialize_writer;

        let hang = McpServerSpec {
            name: "hang".into(),
            transport: McpTransportSpec::Stdio,
            command: Some("sleep".into()),
            args: Some(vec!["30".into()]),
            url: None,
            env: None,
            headers: None,
        };
        let host = HostState::for_test_with_mcp(vec![hang]).expect("host");
        let slot = host.slot("hang-sess").await;
        let t0 = std::time::Instant::now();
        materialize_writer(&host, &slot).await.expect("materialize");
        assert!(
            t0.elapsed() < std::time::Duration::from_secs(2),
            "writer unary MUST NOT wait MCP connect: {:?}",
            t0.elapsed()
        );
        let t1 = std::time::Instant::now();
        let result = crate::app::server::host::handle_unary(
            &host,
            None,
            "set_model",
            serde_json::json!({
                "provider": "",
                "model_id": "missing",
                "session_id": "hang-sess",
            }),
            None,
        )
        .await;
        assert!(
            t1.elapsed() < std::time::Duration::from_secs(2),
            "set_model MUST NOT wait MCP: {:?} result={result:?}",
            t1.elapsed()
        );
    }

    #[tokio::test]
    async fn attach_mux_survives_after_idle_and_receives_later_events() {
        use crate::app::core::host_client::InProcessClient;
        use crate::protocol::Event;

        let host = HostState::for_test().expect("host");
        let client = InProcessClient::host_state(host.clone());
        let mut driver = XyRemoteDriver::with_host(client, "mux-sess");
        driver.attach_session().await.expect("attach");
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        let slot = host.slot("mux-sess").await;
        slot.append_and_push(Event::QueueUpdate {
            steer_count: 1,
            follow_up_count: 0,
        })
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        let first = driver.drain_idle_events();
        assert!(
            first
                .iter()
                .any(|e| matches!(e, XyEvent::QueueUpdate { steer_count: 1, .. })),
            "first downlink event missing: {first:?}"
        );
        slot.append_and_push(Event::AgentEnd).await;
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        let _ = driver.drain_idle_events();
        slot.append_and_push(Event::QueueUpdate {
            steer_count: 0,
            follow_up_count: 1,
        })
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        let later = driver.drain_idle_events();
        assert!(
            later.iter().any(|e| matches!(
                e,
                XyEvent::QueueUpdate {
                    follow_up_count: 1,
                    ..
                }
            )),
            "mux MUST still deliver after AgentEnd: {later:?}"
        );
    }

    #[test]
    fn cold_replay_tape_drops_agent_keeps_queue() {
        assert!(XyRemoteDriver::<InProcessClient>::is_cold_replay_tape(
            &XyEvent::AgentStart {
                session_id: "s".into(),
                model: "m".into(),
            }
        ));
        assert!(XyRemoteDriver::<InProcessClient>::is_cold_replay_tape(
            &XyEvent::TextDelta("hi".into())
        ));
        assert!(!XyRemoteDriver::<InProcessClient>::is_cold_replay_tape(
            &XyEvent::QueueUpdate {
                steer_count: 1,
                follow_up_count: 0,
            }
        ));
        assert!(!XyRemoteDriver::<InProcessClient>::is_cold_replay_tape(
            &XyEvent::error_msg("boom")
        ));
    }

    #[tokio::test]
    async fn cold_subscribe_drops_replay_tape_and_stays_live() {
        use crate::app::core::host_client::InProcessClient;
        use crate::protocol::Event;

        let host = HostState::for_test().expect("host");
        {
            let slot = host.slot("s-cold").await;
            let mut j = slot.journal.lock().await;
            j.append(Event::TextDelta {
                text: "tape-1".into(),
            });
            j.append(Event::TextDelta {
                text: "tape-2".into(),
            });
        }
        let client = InProcessClient::host_state(host.clone());
        let mut driver = XyRemoteDriver::with_host(client, "s-cold");
        driver.attach_session().await.expect("attach");
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        let replay = driver.drain_idle_events();
        assert!(
            !replay
                .iter()
                .any(|e| matches!(e, XyEvent::TextDelta(t) if t.contains("tape"))),
            "cold-replay tape MUST NOT reach the transcript stream (ath36): {replay:?}"
        );
        let slot = host.slot("s-cold").await;
        slot.append_and_push(Event::QueueUpdate {
            steer_count: 0,
            follow_up_count: 2,
        })
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        let live = driver.drain_idle_events();
        assert!(
            live.iter().any(|e| matches!(
                e,
                XyEvent::QueueUpdate {
                    follow_up_count: 2,
                    ..
                }
            )),
            "stream MUST stay live after the recovery window: {live:?}"
        );
    }

    #[tokio::test]
    async fn materialize_writer_binds_client_workspace_cwd() {
        use crate::app::server::host::materialize_writer_at;

        let host = HostState::for_test().expect("host");
        let slot = host.slot("ws-cwd").await;
        let dir = tempfile::tempdir().expect("tmp");
        materialize_writer_at(&host, &slot, dir.path())
            .await
            .expect("materialize");
        let guard = slot.driver.lock().await;
        let cwd = guard.as_ref().expect("writer").agent_cwd_for_test();
        assert_eq!(
            std::path::Path::new(&cwd),
            dir.path(),
            "writer cwd MUST be the TUI workspace, not serve cwd"
        );
    }

    #[tokio::test]
    async fn bash_unary_runs_in_client_workspace() {
        use crate::app::server::host::{handle_unary, materialize_writer_at};

        let host = HostState::for_test().expect("host");
        let slot = host.slot("ws-bang").await;
        let dir = tempfile::tempdir().expect("tmp");
        materialize_writer_at(&host, &slot, dir.path())
            .await
            .expect("materialize");
        let result = handle_unary(
            &host,
            None,
            "bash",
            serde_json::json!({ "session_id": "ws-bang", "command": "pwd" }),
            None,
        )
        .await;
        assert!(result.ok, "bash unary MUST succeed: {result:?}");
        let output = result
            .value
            .as_ref()
            .and_then(|v| v.get("output"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let got = std::path::PathBuf::from(output.trim());
        assert_eq!(
            got.canonicalize().unwrap(),
            dir.path().canonicalize().unwrap(),
            "attach bang MUST run in the session workspace, not serve cwd; got: {output}"
        );
    }

    /// ws1 (c2335): the model-side tool execution surface MUST run in the
    /// session workspace bound at writer materialization — a scripted fake
    /// model emits a `bash pwd` tool call and the result MUST be the
    /// workspace, not the serve process cwd.
    #[tokio::test]
    async fn model_tool_runs_in_client_workspace() {
        use crate::XyModelMeta;
        use crate::XySessionStore;
        use crate::app::core::composition::build_ports_with_store;
        use crate::app::server::host::{ReloadBaseline, materialize_writer_at};
        use crate::infra::provider::factory::{reset_fake_state, set_fake_tool_call};
        use crate::protocol::lifecycle::XyEvent;
        use crate::protocol::model::{XyModelConfig, XyModelKind};
        use futures::StreamExt;
        use std::path::PathBuf;
        use std::sync::Arc;

        reset_fake_state();
        set_fake_tool_call("bash", r#"{"command":"pwd"}"#);

        let dir = tempfile::tempdir().expect("tmp");
        let store: Arc<dyn XySessionStore> = Arc::new(crate::infra::session::SessionManager::new(
            std::env::temp_dir()
                .join(format!("xylitol-host-ws-{}", uuid::Uuid::new_v4()))
                .join("sessions"),
        ));
        let mut registry = crate::agent::ModelRegistry::new();
        registry.register(XyModelMeta {
            id: "fake-ws".into(),
            config: XyModelConfig {
                kind: XyModelKind::Fake,
                api_key: String::new(),
                model: "fake-model".into(),
                base_url: None,
                api: None,
                compat: None,
            },
            display_name: "Fake WS".into(),
            thinking: false,
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
        let options = crate::app::core::composition::BuildAgentOptions {
            model_registry: registry,
            ..Default::default()
        };
        let ports = build_ports_with_store(options, store).expect("ports");
        let host = HostState::new(
            ports,
            ReloadBaseline {
                cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                agent_dir: crate::infra::resource::DefaultResourceLoader::default_agent_dir(),
                project_trusted: true,
                mcp_servers: Vec::new(),
                default_model_id: Some("fake-ws".into()),
            },
            "ws-model-tool-fallback".into(),
        );

        let slot = host.slot("ws-model-tool").await;
        materialize_writer_at(&host, &slot, dir.path())
            .await
            .expect("materialize");

        let mut tool_end_result: Option<String> = None;
        {
            let mut guard = slot.driver.lock().await;
            let driver = guard.as_mut().expect("writer driver");
            driver.select_model("fake-ws").await.expect("select fake");
            let mut stream = driver.run("run pwd via tool").await;
            while let Some(event) = stream.next().await {
                if let XyEvent::ToolExecutionEnd { ref result, .. } = event {
                    tool_end_result = Some(result.clone());
                }
                if matches!(event, XyEvent::AgentEnd { .. }) {
                    break;
                }
            }
        }

        let result = tool_end_result.expect("bash tool must execute during the run");
        assert!(
            !result.contains("error"),
            "model-side bash tool MUST succeed; got {result}"
        );
        assert!(
            result.contains(
                &dir.path()
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            ),
            "model-side bash tool MUST run in the session workspace; got {result}"
        );
    }

    #[tokio::test]
    async fn persist_trust_unary_routes_to_writer() {
        use crate::app::server::host::{handle_unary, materialize_writer_at};

        let host = HostState::for_test().expect("host");
        let slot = host.slot("ws-trust").await;
        let dir = tempfile::tempdir().expect("tmp");
        materialize_writer_at(&host, &slot, dir.path())
            .await
            .expect("materialize");
        let result = handle_unary(
            &host,
            None,
            "persist_trust",
            serde_json::json!({ "session_id": "ws-trust", "mode": "trust_cwd" }),
            None,
        )
        .await;
        assert!(
            result.ok,
            "persist_trust MUST route to the writer: {result:?}"
        );
        let value = result.value.as_ref().expect("value");
        assert!(
            value.get("trusted").is_some() && value.get("message").is_some(),
            "report shape MUST surface trusted/message: {value}"
        );
    }

    #[tokio::test]
    async fn export_unary_returns_content_and_cleans_stage() {
        use crate::app::server::host::{handle_unary, materialize_writer_at};

        let host = HostState::for_test().expect("host");
        let slot = host.slot("ws-export").await;
        let dir = tempfile::tempdir().expect("tmp");
        materialize_writer_at(&host, &slot, dir.path())
            .await
            .expect("materialize");
        let minted = handle_unary(
            &host,
            None,
            "new_session",
            serde_json::json!({ "session_id": "ws-export" }),
            None,
        )
        .await;
        assert!(minted.ok, "new_session MUST succeed: {minted:?}");
        let writer_token = minted
            .value
            .as_ref()
            .and_then(|v| v.get("writerToken"))
            .and_then(Value::as_str)
            .expect("writer token")
            .to_string();
        let result = handle_unary(
            &host,
            None,
            "export_jsonl",
            serde_json::json!({ "session_id": "ws-export" }),
            Some(writer_token),
        )
        .await;
        assert!(result.ok, "export unary MUST succeed: {result:?}");
        let value = result.value.as_ref().expect("value");
        let content = value
            .get("content")
            .and_then(Value::as_str)
            .expect("export response MUST carry content bytes for the TUI to write locally");
        assert!(!content.is_empty(), "jsonl export should have header rows");
        let staged = value
            .get("path")
            .and_then(Value::as_str)
            .map(std::path::PathBuf::from)
            .expect("staged path");
        assert!(
            !staged.exists(),
            "Host-side stage file MUST be cleaned up: {}",
            staged.display()
        );
    }

    #[tokio::test]
    async fn inflight_set_thinking_does_not_apply_until_flush() {
        use crate::app::server::host::{handle_unary, materialize_writer};

        let host = HostState::for_test().expect("host");
        let slot = host.slot("th").await;
        materialize_writer(&host, &slot).await.expect("writer");
        let before = {
            let g = slot.driver.lock().await;
            g.as_ref().expect("driver").thinking_level()
        };
        slot.mark_run_inflight_for_test(true);
        let result = handle_unary(
            &host,
            None,
            "set_thinking_level",
            serde_json::json!({ "session_id": "th", "level": "high" }),
            None,
        )
        .await;
        assert!(result.ok, "defer thinking MUST succeed: {result:?}");
        let during = {
            let g = slot.driver.lock().await;
            g.as_ref().expect("driver").thinking_level()
        };
        assert_eq!(
            during, before,
            "busy thinking MUST NOT retune the in-flight run"
        );
        slot.mark_run_inflight_for_test(false);
        slot.flush_pending_runtime_for_test().await;
    }
}
