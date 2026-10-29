use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use serde_json::Value;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::dispatch::DispatchOutcome;
use crate::host_client::json_rpc::{self, Downlink};
use crate::host_client::{HostClient, HttpWsClient};
use xylitol_plugin_sdk::XyBashResult;
use xylitol_plugin_sdk::message::AgentMessage;
use xylitol_plugin_sdk::model::THINKING_OFF;
use xylitol_protocol::registry::{METHOD_HOST_DESCRIBE, METHOD_SUBSCRIBE};
use xylitol_protocol::{Command, Notification};
use xylitol_wire::{WireEventError, event_from_wire};

use super::AttachObserve;
use super::XyDriver;
use super::XyDriverError;
use super::proto::BashRun;
use super::types::{
    ClipboardCopyOutcome, CommandInfo, EventStream, LoadedResourcesSnapshot, ModelInfo,
    ProjectTrustMode, ProjectTrustPersistReport, QueueStats, ReloadStepReport, RuntimeReloadReport,
    XyEvent,
};

#[cfg(test)]
mod tests;

pub type ReverseRpcNotify = Arc<dyn Fn(String, String, Value) + Send + Sync>;

#[derive(Clone)]
struct SharedDownlink {
    events: Arc<std::sync::Mutex<VecDeque<XyEvent>>>,
    notify: Arc<Notify>,
    started: Arc<AtomicBool>,
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

    fn request_notify(&self) {
        if self.notify_scheduled.swap(true, Ordering::SeqCst) {
            return;
        }
        let notify = self.notify.clone();
        let scheduled = self.notify_scheduled.clone();
        let window_ms = self.window_ms.load(Ordering::SeqCst);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(window_ms)).await;
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

#[derive(Debug, Clone)]
pub struct LinkTunings {
    pub backoff_base: Duration,
    pub backoff_cap: Duration,
    pub survive_threshold: Duration,
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

pub struct XyRemoteDriver<C = HttpWsClient> {
    host: C,
    session_id: String,
    session_life: CancellationToken,
    turn_cancel: Arc<std::sync::Mutex<CancellationToken>>,
    thinking: std::sync::Mutex<String>,
    leaf_entry_id: Arc<std::sync::Mutex<Option<String>>>,
    last_seq: Arc<AtomicU64>,
    downlink_gen: Arc<AtomicU64>,
    fatal: Arc<std::sync::Mutex<Option<String>>>,
    tunings: Arc<LinkTunings>,
    reverse_rpc: Option<ReverseRpcNotify>,
    downlink: SharedDownlink,
    resync_needed: Arc<AtomicBool>,
    subscribed_ok: Arc<AtomicBool>,
    skip_cold_replay: Arc<AtomicBool>,
    attach_observe: Arc<std::sync::Mutex<AttachObserve>>,
    client_cwd: String,
    cached_model: Arc<std::sync::Mutex<Option<ModelInfo>>>,
    cached_models: Arc<std::sync::Mutex<Option<Vec<ModelInfo>>>>,
    cached_commands: Arc<std::sync::Mutex<Option<Vec<CommandInfo>>>>,
    cached_queue: Arc<std::sync::Mutex<QueueStats>>,
    cached_resources: Arc<std::sync::Mutex<LoadedResourcesSnapshot>>,
    cached_capabilities: Arc<std::sync::Mutex<xylitol_plugin_sdk::session::CapabilitySnapshot>>,
    cached_skills: Arc<std::sync::Mutex<Vec<(String, String)>>>,
    cached_gate_notice: Arc<std::sync::Mutex<Option<String>>>,
    gate_notice_consumed: Arc<AtomicBool>,
    resources_dirty: Arc<AtomicBool>,
    freeze_arm_inflight: Arc<AtomicBool>,
    bash_run_sink: Arc<std::sync::Mutex<Option<xylitol_plugin_sdk::BashOutputSink>>>,
}

impl XyRemoteDriver<HttpWsClient> {
    pub fn new(base_url: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self::with_host(HttpWsClient::new(base_url), session_id)
    }

    pub fn host_client(&self) -> &HttpWsClient {
        &self.host
    }
}

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
    attach_observe: Arc<std::sync::Mutex<AttachObserve>>,
    cached_queue: Arc<std::sync::Mutex<QueueStats>>,
    cached_skills: Arc<std::sync::Mutex<Vec<(String, String)>>>,
    cached_resources: Arc<std::sync::Mutex<LoadedResourcesSnapshot>>,
    cached_gate_notice: Arc<std::sync::Mutex<Option<String>>>,
    gate_notice_consumed: Arc<AtomicBool>,
    resources_dirty: Arc<AtomicBool>,
    cached_model: Arc<std::sync::Mutex<Option<ModelInfo>>>,
    cached_models: Arc<std::sync::Mutex<Option<Vec<ModelInfo>>>>,
    bash_run_sink: Arc<std::sync::Mutex<Option<xylitol_plugin_sdk::BashOutputSink>>>,
    client_cwd: String,
    downlink_gen: Arc<AtomicU64>,
    fatal: Arc<std::sync::Mutex<Option<String>>>,
    tunings: Arc<LinkTunings>,
    my_gen: u64,
}

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

impl<C> DownlinkCtx<C>
where
    C: HostClient + Clone + 'static,
{
    async fn resolve_model_info(&self, model_id: &str) -> Option<ModelInfo> {
        let find = |models: &[ModelInfo]| models.iter().find(|m| m.id == model_id).cloned();
        let from_cache = || {
            self.cached_models
                .lock()
                .ok()
                .and_then(|g| g.clone())
                .unwrap_or_default()
        };
        if let Some(hit) = find(&from_cache()) {
            return Some(hit);
        }
        let Ok(data) = self
            .host
            .unary("get_available_models", serde_json::json!({}))
            .await
        else {
            return None;
        };
        let models = data
            .value
            .as_ref()
            .and_then(|v| v.get("models"))
            .and_then(|v| v.as_array())?;
        let parsed: Vec<ModelInfo> = models
            .iter()
            .filter_map(|m| {
                Some(ModelInfo {
                    id: m.get("id").and_then(|x| x.as_str())?.to_string(),
                    display_name: m
                        .get("display_name")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    thinking: m.get("thinking").and_then(|x| x.as_bool()).unwrap_or(false),
                    thinking_levels: m
                        .get("thinking_levels")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_else(|| vec![THINKING_OFF.into()]),
                    context_window: m
                        .get("context_window")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(0),
                })
            })
            .collect();
        if let Ok(mut g) = self.cached_models.lock() {
            *g = Some(parsed.clone());
        }
        find(&parsed)
    }
}

async fn handle_frame<C>(ctx: &DownlinkCtx<C>, push_current: &impl Fn(XyEvent), note: Notification)
where
    C: HostClient + Clone + 'static,
{
    let frame = match json_rpc::decode_downlink(note) {
        Ok(Some(frame)) => frame,
        Ok(None) => return,
        Err(e) => {
            log::warn!(target: "xylitol::tui", "downlink frame dropped: {e}");
            return;
        }
    };
    match frame {
        Downlink::SessionEvent(payload) => {
            ctx.last_seq.store(payload.seq, Ordering::SeqCst);
            let agent_event = match event_from_wire(&payload.event) {
                Ok(ev) => ev,
                Err(WireEventError::Unknown) => {
                    log::warn!(target: "xylitol::tui", "session event skipped: unknown wire variant");
                    return;
                }
                Err(e) => {
                    log::error!(target: "xylitol::tui", "session event skipped: {e}");
                    return;
                }
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
            if let XyEvent::ModelSelect { model_id, .. } = &agent_event {
                let found = ctx.resolve_model_info(model_id).await;
                log::debug!(
                    target: "xylitol::tui",
                    "host-synced model {model_id} resolved=> {}",
                    found.as_ref().map(|m| m.id.clone()).unwrap_or_default()
                );
                if let Some(found) = found
                    && let Ok(mut cached) = ctx.cached_model.lock()
                {
                    *cached = Some(found);
                }
            }
            if ctx.skip_cold_replay.load(Ordering::SeqCst)
                && XyRemoteDriver::<C>::is_cold_replay_tape(&agent_event)
            {
                return;
            }
            if matches!(agent_event, XyEvent::AgentEnd { .. })
                && let Ok(mut g) = ctx.attach_observe.lock()
                && *g == AttachObserve::Inflight
            {
                *g = AttachObserve::Ended;
            }
            push_current(agent_event);
        }
        Downlink::SessionSubscribed(_) => {
            ctx.skip_cold_replay.store(false, Ordering::SeqCst);
        }
        Downlink::SessionModel(payload) => {
            let agent_event = XyEvent::ModelSelect {
                provider: payload.provider,
                model_id: payload.model_id,
            };
            if let XyEvent::ModelSelect { model_id, .. } = &agent_event {
                let found = ctx.resolve_model_info(model_id).await;
                log::debug!(
                    target: "xylitol::tui",
                    "host-synced model {model_id} resolved=> {}",
                    found.as_ref().map(|m| m.id.clone()).unwrap_or_default()
                );
                if let Some(found) = found
                    && let Ok(mut cached) = ctx.cached_model.lock()
                {
                    *cached = Some(found);
                }
            }
            push_current(agent_event);
        }
        Downlink::BashOutput(payload) => {
            let chunk = xylitol_plugin_sdk::BashChunk {
                bash_id: payload.bash_id,
                seq: payload.seq,
                data: payload.data,
            };
            if let Ok(guard) = ctx.bash_run_sink.lock()
                && let Some(sink) = guard.as_ref()
            {
                let _ = sink.tx.try_send(chunk);
            }
        }
        Downlink::SessionResources(payload) => {
            if let Ok(snap) = serde_json::from_value::<LoadedResourcesSnapshot>(payload.snapshot) {
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
        Downlink::SessionResyncRequired(_) => {
            ctx.last_seq.store(0, Ordering::SeqCst);
            ctx.resync_needed.store(true, Ordering::SeqCst);
            let _ = ctx
                .host
                .unary(
                    METHOD_SUBSCRIBE,
                    serde_json::json!({
                        "session_id": ctx.session_id,
                        "last_seq": 0,
                        "cwd": ctx.client_cwd,
                    }),
                )
                .await;
        }
        Downlink::Interaction {
            method,
            call_id,
            params,
        } => {
            if let Some(notify) = &ctx.reverse_rpc {
                notify(call_id, method.to_string(), params);
            }
        }
    }
}

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

async fn downlink_loop<C>(ctx: DownlinkCtx<C>)
where
    C: HostClient + Clone + 'static,
{
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
                let seq = ctx.last_seq.load(Ordering::SeqCst);
                ctx.skip_cold_replay.store(seq == 0, Ordering::SeqCst);
                match ctx
                    .host
                    .unary(
                        METHOD_SUBSCRIBE,
                        serde_json::json!({
                            "session_id": ctx.session_id,
                            "last_seq": seq,
                            "cwd": ctx.client_cwd,
                        }),
                    )
                    .await
                {
                    Err(_) => {
                        ctx.subscribed_ok.store(false, Ordering::SeqCst);
                        match reconnect_pause(&ctx, backoff, connected_at.elapsed(), &next_backoff)
                            .await
                        {
                            Some(next) => backoff = next,
                            None => return,
                        }
                        continue;
                    }
                    Ok(result) => {
                        apply_subscribe_result(&ctx, result.into_std().ok(), seq, &push_current);
                    }
                }
                ctx.subscribed_ok.store(true, Ordering::SeqCst);
                pump_frames(&ctx, &mut mux, &push_current).await;
                ctx.subscribed_ok.store(false, Ordering::SeqCst);
                match reconnect_pause(&ctx, backoff, connected_at.elapsed(), &next_backoff).await {
                    Some(next) => backoff = next,
                    None => return,
                }
            }
            Err(e) if e.is_fatal_mismatch() => {
                let msg = e.to_string();
                if let Ok(mut slot) = ctx.fatal.lock() {
                    *slot = Some(msg.clone());
                }
                push_current(XyEvent::error_msg(msg));
                return;
            }
            Err(_) => {
                ctx.subscribed_ok.store(false, Ordering::SeqCst);
                match reconnect_pause(&ctx, backoff, connected_at.elapsed(), &next_backoff).await {
                    Some(next) => backoff = next,
                    None => return,
                }
            }
        }
    }
}

fn apply_subscribe_result<C>(
    ctx: &DownlinkCtx<C>,
    value: Option<Value>,
    seq: u64,
    push_current: &impl Fn(XyEvent),
) where
    C: HostClient + Clone + 'static,
{
    let inflight = value
        .as_ref()
        .and_then(|value| value.get("inflight").and_then(Value::as_bool))
        .unwrap_or(false);
    if let Ok(mut observe) = ctx.attach_observe.lock() {
        *observe = if inflight && seq == 0 {
            AttachObserve::Inflight
        } else {
            AttachObserve::Live
        };
    }
    if let Some(partial) = value
        .as_ref()
        .and_then(|value| value.get("partial"))
        .and_then(|partial| serde_json::from_value::<AgentMessage>(partial.clone()).ok())
    {
        push_current(XyEvent::MessageUpdate {
            message: Some(partial),
        });
    }
}

async fn pump_frames<C>(
    ctx: &DownlinkCtx<C>,
    mux: &mut crate::host_client::MuxStream,
    push_current: &impl Fn(XyEvent),
) where
    C: HostClient + Clone + 'static,
{
    loop {
        tokio::select! {
            _ = ctx.life.cancelled() => return,
            msg = mux.next() => {
                match msg {
                    Some(Ok(frame)) => handle_frame(ctx, push_current, frame).await,
                    Some(Err(_)) | None => return,
                }
            }
        }
    }
}

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
            attach_observe: Arc::new(std::sync::Mutex::new(AttachObserve::Live)),
            client_cwd: std::env::current_dir()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| ".".into()),
            cached_model: Arc::new(std::sync::Mutex::new(None)),
            cached_models: Arc::new(std::sync::Mutex::new(None)),
            cached_commands: Arc::new(std::sync::Mutex::new(None)),
            cached_queue: Arc::new(std::sync::Mutex::new(QueueStats::default())),
            cached_resources: Arc::new(std::sync::Mutex::new(LoadedResourcesSnapshot::default())),
            cached_capabilities: Arc::new(std::sync::Mutex::new(
                xylitol_plugin_sdk::session::CapabilitySnapshot::default(),
            )),
            cached_skills: Arc::new(std::sync::Mutex::new(Vec::new())),
            cached_gate_notice: Arc::new(std::sync::Mutex::new(None)),
            gate_notice_consumed: Arc::new(AtomicBool::new(false)),
            resources_dirty: Arc::new(AtomicBool::new(false)),
            freeze_arm_inflight: Arc::new(AtomicBool::new(false)),
            bash_run_sink: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub fn with_tunings(mut self, tunings: LinkTunings) -> Self {
        self.downlink.set_window(tunings.coalesce_window);
        self.tunings = Arc::new(tunings);
        self
    }

    pub fn set_reverse_rpc_notify(&mut self, notify: ReverseRpcNotify) {
        self.reverse_rpc = Some(notify);
    }

    pub fn mark_subscribed_ok_for_test(&self) {
        self.subscribed_ok.store(true, Ordering::SeqCst);
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

    pub fn is_cold_replay_tape(ev: &XyEvent) -> bool {
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

    pub fn apply_resources_cache(&self, snap: LoadedResourcesSnapshot) {
        store_resources_snapshot(
            &self.cached_skills,
            &self.gate_notice_consumed,
            &self.cached_gate_notice,
            &self.cached_resources,
            &self.resources_dirty,
            snap,
        );
    }

    async fn restart_downlink(&mut self) {
        self.downlink_gen.fetch_add(1, Ordering::SeqCst);
        self.session_life.cancel();
        self.session_life = CancellationToken::new();
        self.subscribed_ok.store(false, Ordering::SeqCst);
        self.downlink.started.store(false, Ordering::SeqCst);
        let _ = self.host.reset_mux().await;
        self.ensure_downlink();
    }

    async fn restart_downlink_for_switched_session(&mut self) {
        self.last_seq.store(0, Ordering::SeqCst);
        self.restart_downlink().await;
    }

    async fn check_host_protocol(&self) -> Result<(), XyDriverError> {
        let described = self
            .host
            .unary(METHOD_HOST_DESCRIBE, serde_json::json!({}))
            .await
            .and_then(|result| json_rpc::check_describe(&result));
        match described {
            Ok(()) => Ok(()),
            Err(e) if e.is_fatal_mismatch() => {
                let msg = e.to_string();
                if let Ok(mut slot) = self.fatal.lock() {
                    *slot = Some(msg.clone());
                }
                self.downlink.push(XyEvent::error_msg(msg.clone()));
                Err(XyDriverError::remote(msg))
            }
            Err(e) => Err(XyDriverError::remote(e.to_string())),
        }
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
        tokio::spawn(downlink_loop(self.downlink_ctx()));
    }

    fn downlink_ctx(&self) -> DownlinkCtx<C> {
        DownlinkCtx {
            host: self.host.clone(),
            session_id: self.session_id.clone(),
            last_seq: self.last_seq.clone(),
            reverse_rpc: self.reverse_rpc.clone(),
            downlink: self.downlink.clone(),
            life: self.session_life.clone(),
            resync_needed: self.resync_needed.clone(),
            subscribed_ok: self.subscribed_ok.clone(),
            skip_cold_replay: self.skip_cold_replay.clone(),
            attach_observe: self.attach_observe.clone(),
            cached_queue: self.cached_queue.clone(),
            cached_skills: self.cached_skills.clone(),
            cached_resources: self.cached_resources.clone(),
            cached_gate_notice: self.cached_gate_notice.clone(),
            gate_notice_consumed: self.gate_notice_consumed.clone(),
            resources_dirty: self.resources_dirty.clone(),
            cached_model: self.cached_model.clone(),
            cached_models: self.cached_models.clone(),
            bash_run_sink: self.bash_run_sink.clone(),
            client_cwd: self.client_cwd.clone(),
            downlink_gen: self.downlink_gen.clone(),
            fatal: self.fatal.clone(),
            tunings: self.tunings.clone(),
            my_gen: self.downlink_gen.load(Ordering::SeqCst),
        }
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

    pub fn queue_stats(&self) -> QueueStats {
        self.cached_queue.lock().map(|s| *s).unwrap_or_default()
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

    pub async fn unary(
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

#[async_trait]
impl<C> XyDriver for XyRemoteDriver<C>
where
    C: HostClient + Clone + 'static,
{
    async fn attach_session(&mut self) -> Result<(), XyDriverError> {
        self.check_host_protocol().await?;
        self.ensure_downlink();
        self.wait_subscribed().await?;
        self.refresh_fixed_zone_caches().await
    }

    async fn bash_run(
        &mut self,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<BashRun, XyDriverError> {
        self.ensure_downlink();
        self.wait_subscribed().await?;
        let host = self.host.clone();
        let cmd = Command::Bash {
            command: command.to_string(),
            exclude_from_context,
        };
        let payload = self.with_session(
            serde_json::to_value(&cmd).map_err(|e| XyDriverError::remote(e.to_string()))?,
        );
        let fut = async move {
            let data = host
                .unary("bash", payload)
                .await
                .map_err(|e| XyDriverError::remote(e.to_string()))?
                .into_std()
                .map_err(|e| XyDriverError::remote(format!("{}: {}", e.code, e.details)))?;
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
        };
        Ok(Box::pin(fut) as BashRun)
    }

    async fn refresh_surface_caches(&mut self) -> Result<(), XyDriverError> {
        self.refresh_fixed_zone_caches().await
    }

    fn drain_idle_events(&mut self) -> Vec<XyEvent> {
        self.downlink.drain()
    }

    fn link_health(&self) -> super::LinkHealth {
        if self.subscribed_ok.load(Ordering::SeqCst) {
            super::LinkHealth::Up
        } else {
            super::LinkHealth::Down
        }
    }

    fn attach_observe(&self) -> super::AttachObserve {
        *self
            .attach_observe
            .lock()
            .unwrap_or_else(|e| e.into_inner())
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
        self.skip_cold_replay.store(false, Ordering::SeqCst);
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
            match host.unary("prompt", body).await {
                Err(e) => {
                    yield XyEvent::error_msg(format!("prompt failed: {e}"));
                    return;
                }
                Ok(result) if !result.ok => {
                    let details = result.error.map(|e| e.details).unwrap_or_default();
                    yield XyEvent::error_msg(if details.is_empty() {
                        "prompt failed".into()
                    } else {
                        format!("prompt failed: {details}")
                    });
                    return;
                }
                Ok(_) => {}
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

    fn thinking_level(&self) -> String {
        self.thinking.lock().unwrap().clone()
    }

    fn session_id(&self) -> Option<String> {
        Some(self.session_id.clone())
    }

    async fn estimate_context_tokens(
        &self,
    ) -> Result<xylitol_plugin_sdk::model::ContextTokenEstimate, XyDriverError> {
        let data = self.unary_cmd(Command::EstimateContext {}).await?;
        serde_json::from_value(data.get("estimate").cloned().unwrap_or(Value::Null))
            .map_err(|e| XyDriverError::remote(format!("estimate_context payload: {e}")))
    }

    fn get_commands(&self) -> Vec<CommandInfo> {
        self.cached_commands
            .lock()
            .ok()
            .and_then(|cached| cached.clone())
            .unwrap_or_default()
    }

    fn set_bash_run_sink(&mut self, sink: Option<xylitol_plugin_sdk::BashOutputSink>) {
        if let Ok(mut guard) = self.bash_run_sink.lock() {
            *guard = sink;
        }
    }

    fn leaf_entry_id(&self) -> Option<String> {
        self.leaf_entry_id.lock().ok().and_then(|leaf| leaf.clone())
    }

    fn capability_snapshot(&self) -> xylitol_plugin_sdk::session::CapabilitySnapshot {
        self.cached_capabilities
            .lock()
            .map(|snap| snap.clone())
            .unwrap_or_default()
    }

    fn capability_pending(&self) -> bool {
        self.capability_snapshot().pending
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
        if self.freeze_arm_inflight.swap(true, Ordering::SeqCst) {
            return;
        }
        let host = self.host.clone();
        let payload = self.with_session(serde_json::json!({}));
        let cached_skills = self.cached_skills.clone();
        let gate_notice_consumed = self.gate_notice_consumed.clone();
        let cached_gate_notice = self.cached_gate_notice.clone();
        let cached_resources = self.cached_resources.clone();
        let resources_dirty = self.resources_dirty.clone();
        let inflight = self.freeze_arm_inflight.clone();
        let life = self.session_life.clone();
        tokio::spawn(async move {
            let result = tokio::select! {
                biased;
                () = life.cancelled() => None,
                result = host.unary("arm_tool_freeze", payload) => Some(result),
            };
            inflight.store(false, Ordering::SeqCst);
            let Some(result) = result else {
                return;
            };
            let data = match result {
                Ok(rpc) => match rpc.into_std() {
                    Ok(data) => data,
                    Err(e) => {
                        log::warn!(
                            target: "xylitol::tui",
                            "remote.arm_tool_freeze: {}: {}",
                            e.code,
                            e.details
                        );
                        return;
                    }
                },
                Err(e) => {
                    log::warn!(target: "xylitol::tui", "remote.arm_tool_freeze: {e}");
                    return;
                }
            };
            if let Ok(snap) = serde_json::from_value::<LoadedResourcesSnapshot>(data) {
                store_resources_snapshot(
                    &cached_skills,
                    &gate_notice_consumed,
                    &cached_gate_notice,
                    &cached_resources,
                    &resources_dirty,
                    snap,
                );
            }
        });
    }

    async fn poll_prepare_progress(&mut self) -> bool {
        self.resources_dirty.swap(false, Ordering::SeqCst)
    }

    async fn reload_runtime(
        &mut self,
        cancel: &CancellationToken,
    ) -> Result<RuntimeReloadReport, XyDriverError> {
        self.gate_notice_consumed.store(false, Ordering::SeqCst);
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

#[async_trait]
impl<C> crate::dispatch::SessionCommandExecutor for XyRemoteDriver<C>
where
    C: HostClient + Clone + 'static,
{
    async fn execute_session_command(
        &mut self,
        cmd: xylitol_protocol::Command,
    ) -> Result<DispatchOutcome, XyDriverError> {
        use xylitol_protocol::Command;

        match cmd {
            Command::Abort { .. } => {
                crate::driver::XyDriver::abort(self);
                Ok(DispatchOutcome::Aborted { cancelled: true })
            }
            Command::GetState { .. } => Ok(DispatchOutcome::State(
                crate::driver::XyDriver::get_state(self),
            )),
            Command::GetAvailableModels { .. } => Ok(DispatchOutcome::Models(
                crate::driver::XyDriver::available_models(self),
            )),
            Command::SetModel { model_id, .. } => self.exec_set_model(model_id).await,
            Command::CycleModel { .. } => self.exec_cycle_model().await,
            Command::SetThinkingLevel { level, .. } => self.exec_set_thinking_level(level).await,
            Command::SetPlanMode { mode, plan_id } => self.exec_set_plan_mode(mode, plan_id).await,
            Command::Bash {
                command,
                exclude_from_context,
                ..
            } => self.exec_bash(command, exclude_from_context).await,
            Command::Compact { instructions, .. } => self.exec_compact(instructions).await,
            Command::GetSessionStats { .. } => self.exec_session_stats().await,
            Command::ExportHtml { output_path, .. } => self.exec_export_html(output_path).await,
            Command::ExportJsonl { output_path, .. } => self.exec_export_jsonl(output_path).await,
            Command::ImportJsonl { input_path, .. } => self.exec_import_jsonl(input_path).await,
            Command::SwitchSession { session_path, .. } => {
                self.exec_switch_session(session_path).await
            }
            Command::Fork {
                entry_id, position, ..
            } => self.exec_fork(entry_id, position).await,
            Command::GetMessages {
                limit,
                before_entry_id,
            } => self.exec_get_messages(limit, before_entry_id).await,
            Command::EstimateContext { .. } => self.exec_estimate_context().await,
            Command::SessionTree { kind, .. } => self.exec_session_tree(kind).await,
            Command::TravelSessionTree { kind, entry_id, .. } => {
                self.exec_travel_session_tree(kind, entry_id).await
            }
            Command::AppendEntryLabel {
                target_id, label, ..
            } => self.exec_append_entry_label(target_id, label).await,
            cmd @ (Command::ListSessions { .. }
            | Command::LoadSessionEntries { .. }
            | Command::NewSession { .. }
            | Command::GetSessionName { .. }
            | Command::SetSessionName { .. }
            | Command::SetSessionNameFor { .. }
            | Command::DeleteSession { .. }) => self.exec_session_meta(cmd).await,
            Command::Reload { .. } => self.exec_reload().await,
            Command::LoadedResources { .. } => self.exec_loaded_resources().await,
            Command::GetQueueStats { .. } => self.exec_get_queue_stats().await,
            Command::GetCommands { .. } => Ok(DispatchOutcome::Commands(
                crate::driver::XyDriver::get_commands(self),
            )),
            cmd @ (Command::GetCapabilities { .. }
            | Command::SetCapability { .. }
            | Command::ClearCapabilities { .. }) => self.exec_capability(cmd).await,
            Command::Steer { message, .. } => self.exec_steer(message).await,
            Command::FollowUp { message, .. } => self.exec_follow_up(message).await,
            Command::ClearQueue {
                clear_steer,
                clear_follow_up,
                ..
            } => self.exec_clear_queue(clear_steer, clear_follow_up).await,
            Command::Prompt { .. }
            | Command::Quit { .. }
            | Command::Subscribe { .. }
            | Command::ApproveTool { .. }
            | Command::AnswerQuestion { .. } => Err(crate::dispatch::transport_variant_error(&cmd)),
        }
    }
}

impl<C> XyRemoteDriver<C>
where
    C: HostClient + Clone + 'static,
{
    async fn exec_set_model(&mut self, model_id: String) -> Result<DispatchOutcome, XyDriverError> {
        let data = self
            .unary_cmd(Command::SetModel {
                provider: String::new(),
                model_id: model_id.clone(),
            })
            .await?;
        let selected = Self::decode_model_selection(&data, &model_id)?;
        self.apply_model_selection(selected.clone());
        Ok(DispatchOutcome::Model(selected))
    }

    async fn exec_cycle_model(&mut self) -> Result<DispatchOutcome, XyDriverError> {
        let data = self.unary_cmd(Command::CycleModel {}).await?;
        let selected = Self::model_from_value(&data)?;
        self.apply_model_selection(selected.clone());
        Ok(DispatchOutcome::Model(selected))
    }

    async fn exec_set_thinking_level(
        &mut self,
        level: String,
    ) -> Result<DispatchOutcome, XyDriverError> {
        self.unary_cmd(Command::SetThinkingLevel {
            level: level.clone(),
        })
        .await?;
        *self.thinking.lock().unwrap() = level.clone();
        Ok(DispatchOutcome::ThinkingLevel(level))
    }

    async fn exec_set_plan_mode(
        &mut self,
        mode: String,
        plan_id: Option<String>,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let data = self
            .unary_cmd(Command::SetPlanMode { mode, plan_id })
            .await?;
        Ok(plan_mode_outcome(&data))
    }

    async fn exec_bash(
        &mut self,
        command: String,
        exclude_from_context: bool,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let run = crate::driver::XyDriver::bash_run(self, &command, exclude_from_context).await?;
        let result = run.await?;
        Ok(DispatchOutcome::Bash(result))
    }

    async fn exec_compact(
        &mut self,
        instructions: Option<String>,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let data = self.unary_cmd(Command::Compact { instructions }).await?;
        Ok(DispatchOutcome::Compacted(
            data.get("compacted")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ))
    }

    async fn exec_session_stats(&self) -> Result<DispatchOutcome, XyDriverError> {
        let data = self.unary_cmd(Command::GetSessionStats {}).await?;
        Ok(DispatchOutcome::SessionStats(data))
    }

    async fn exec_export_html(
        &mut self,
        output_path: Option<String>,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let path = output_path.unwrap_or_else(|| "export.html".to_string());
        let data = self
            .unary_cmd(Command::ExportHtml { output_path: None })
            .await?;
        let written = match data.get("content").and_then(|content| content.as_str()) {
            Some(content) => {
                std::fs::write(&path, content).map_err(|err| XyDriverError::io(err.to_string()))?;
                path.clone()
            }
            None => data
                .get("path")
                .and_then(|path| path.as_str())
                .unwrap_or(&path)
                .to_string(),
        };
        Ok(DispatchOutcome::ExportedPath(written))
    }

    async fn exec_export_jsonl(
        &mut self,
        output_path: Option<String>,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let path = output_path.unwrap_or_else(|| "export.jsonl".to_string());
        let data = self
            .unary_cmd(Command::ExportJsonl { output_path: None })
            .await?;
        let written = match data.get("content").and_then(|content| content.as_str()) {
            Some(content) => {
                std::fs::write(&path, content).map_err(|err| XyDriverError::io(err.to_string()))?;
                path.clone()
            }
            None => data
                .get("path")
                .and_then(|path| path.as_str())
                .unwrap_or(&path)
                .to_string(),
        };
        Ok(DispatchOutcome::ExportedPath(written))
    }

    async fn exec_import_jsonl(
        &mut self,
        input_path: String,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let content = std::fs::read_to_string(&input_path)
            .map_err(|err| XyDriverError::io(err.to_string()))?;
        let data = self
            .unary("import_jsonl", serde_json::json!({ "content": content }))
            .await?;
        Ok(DispatchOutcome::NewSession(
            data.get("session_id")
                .and_then(|session_id| session_id.as_str())
                .unwrap_or("")
                .to_string(),
        ))
    }

    async fn exec_switch_session(
        &mut self,
        session_path: String,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let data = self
            .unary_cmd(Command::SwitchSession { session_path })
            .await?;
        let id = data
            .get("session_id")
            .and_then(|session_id| session_id.as_str())
            .unwrap_or("")
            .to_string();
        let id = self.bind_new_session(id);
        self.restart_downlink_for_switched_session().await;
        Ok(DispatchOutcome::SwitchedSession(id))
    }

    async fn exec_fork(
        &mut self,
        entry_id: String,
        position: Option<String>,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let position = match position.as_deref() {
            Some("before") => "before",
            _ => "at",
        }
        .to_string();
        let data = self
            .unary_cmd(Command::Fork {
                entry_id,
                position: Some(position),
            })
            .await?;
        Ok(DispatchOutcome::NewSession(
            data.get("session_id")
                .and_then(|session_id| session_id.as_str())
                .unwrap_or("")
                .to_string(),
        ))
    }

    async fn exec_get_messages(
        &self,
        limit: Option<usize>,
        before_entry_id: Option<String>,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let data = self
            .unary_cmd(Command::GetMessages {
                limit,
                before_entry_id,
            })
            .await?;
        let entries = data
            .get("entries")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let entries = serde_json::from_value(entries)
            .map_err(|err| XyDriverError::remote(err.to_string()))?;
        let has_more = data
            .get("has_more")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        Ok(DispatchOutcome::Messages {
            session_id: self.session_id.clone(),
            entries,
            has_more,
        })
    }

    async fn exec_estimate_context(&self) -> Result<DispatchOutcome, XyDriverError> {
        let data = self.unary_cmd(Command::EstimateContext {}).await?;
        let estimate = serde_json::from_value(data.get("estimate").cloned().unwrap_or(Value::Null))
            .map_err(|err| XyDriverError::remote(err.to_string()))?;
        Ok(DispatchOutcome::EstimateContext(estimate))
    }

    async fn exec_session_tree(
        &self,
        kind: xylitol_protocol::SessionTreeKind,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let data = self.unary_cmd(Command::SessionTree { kind }).await?;
        let tree = serde_json::from_value(data.get("tree").cloned().unwrap_or(Value::Null))
            .map_err(|err| XyDriverError::remote(err.to_string()))?;
        Ok(DispatchOutcome::SessionTree(tree))
    }

    async fn exec_travel_session_tree(
        &self,
        kind: xylitol_protocol::SessionTreeKind,
        entry_id: String,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let data = self
            .unary_cmd(Command::TravelSessionTree { kind, entry_id })
            .await?;
        let travel =
            serde_json::from_value(data).map_err(|err| XyDriverError::remote(err.to_string()))?;
        Ok(DispatchOutcome::SessionTreeTravel(travel))
    }

    async fn exec_append_entry_label(
        &mut self,
        target_id: String,
        label: Option<String>,
    ) -> Result<DispatchOutcome, XyDriverError> {
        self.unary_cmd(Command::AppendEntryLabel { target_id, label })
            .await?;
        Ok(DispatchOutcome::Empty)
    }

    async fn exec_list_sessions(&self) -> Result<DispatchOutcome, XyDriverError> {
        let data = self.unary_cmd(Command::ListSessions {}).await?;
        let sessions = serde_json::from_value(data.get("sessions").cloned().unwrap_or(Value::Null))
            .map_err(|err| XyDriverError::remote(err.to_string()))?;
        Ok(DispatchOutcome::Sessions(sessions))
    }

    async fn exec_load_session_entries(
        &self,
        session_id: String,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let data = self
            .unary_cmd(Command::LoadSessionEntries { session_id })
            .await?;
        let entries = serde_json::from_value(data.get("entries").cloned().unwrap_or(Value::Null))
            .map_err(|err| XyDriverError::remote(err.to_string()))?;
        Ok(DispatchOutcome::SessionEntries(entries))
    }

    async fn exec_new_session(&mut self) -> Result<DispatchOutcome, XyDriverError> {
        let data = self.unary_cmd(Command::NewSession {}).await?;
        let id = data
            .get("session_id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| XyDriverError::remote("new_session response missing session_id"))?
            .to_string();
        let id = self.bind_new_session(id);
        self.restart_downlink_for_switched_session().await;
        Ok(DispatchOutcome::NewSession(id))
    }

    async fn exec_get_session_name(&self) -> Result<DispatchOutcome, XyDriverError> {
        let data = self.unary_cmd(Command::GetSessionName {}).await?;
        Ok(DispatchOutcome::SessionName(
            data.get("name")
                .and_then(|value| value.as_str())
                .map(str::to_string),
        ))
    }

    async fn exec_set_session_name(
        &mut self,
        name: String,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let data = self.unary_cmd(Command::SetSessionName { name }).await?;
        session_name_outcome(&data, "set_session_name response missing name")
    }

    async fn exec_set_session_name_for(
        &mut self,
        session_id: String,
        name: String,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let data = self
            .unary_cmd(Command::SetSessionNameFor { session_id, name })
            .await?;
        session_name_outcome(&data, "set_session_name_for response missing name")
    }

    async fn exec_delete_session(
        &mut self,
        session_id: String,
    ) -> Result<DispatchOutcome, XyDriverError> {
        self.unary_cmd(Command::DeleteSession { session_id })
            .await?;
        Ok(DispatchOutcome::Empty)
    }

    async fn exec_reload(&mut self) -> Result<DispatchOutcome, XyDriverError> {
        let report = crate::driver::XyDriver::reload_runtime(
            self,
            &tokio_util::sync::CancellationToken::new(),
        )
        .await?;
        Ok(DispatchOutcome::Reload(report))
    }

    fn store_capabilities(
        &self,
        snap: xylitol_plugin_sdk::session::CapabilitySnapshot,
    ) -> xylitol_plugin_sdk::session::CapabilitySnapshot {
        if let Ok(mut cached) = self.cached_capabilities.lock() {
            *cached = snap.clone();
        }
        snap
    }

    async fn exec_capability(&self, cmd: Command) -> Result<DispatchOutcome, XyDriverError> {
        match cmd {
            Command::GetCapabilities { .. } => self.exec_get_capabilities().await,
            Command::SetCapability { id, enabled } => self.exec_set_capability(id, enabled).await,
            Command::ClearCapabilities { .. } => self.exec_clear_capabilities().await,
            other => Err(crate::dispatch::transport_variant_error(&other)),
        }
    }

    async fn exec_session_meta(&mut self, cmd: Command) -> Result<DispatchOutcome, XyDriverError> {
        match cmd {
            Command::ListSessions { .. } => self.exec_list_sessions().await,
            Command::LoadSessionEntries { session_id, .. } => {
                self.exec_load_session_entries(session_id).await
            }
            Command::NewSession { .. } => self.exec_new_session().await,
            Command::GetSessionName { .. } => self.exec_get_session_name().await,
            Command::SetSessionName { name, .. } => self.exec_set_session_name(name).await,
            Command::SetSessionNameFor {
                session_id, name, ..
            } => self.exec_set_session_name_for(session_id, name).await,
            Command::DeleteSession { session_id, .. } => self.exec_delete_session(session_id).await,
            other => Err(crate::dispatch::transport_variant_error(&other)),
        }
    }

    async fn exec_get_capabilities(&self) -> Result<DispatchOutcome, XyDriverError> {
        let data = self.unary_cmd(Command::GetCapabilities {}).await?;
        let snap: xylitol_plugin_sdk::session::CapabilitySnapshot =
            serde_json::from_value(data).map_err(|e| XyDriverError::remote(e.to_string()))?;
        Ok(DispatchOutcome::Capabilities(self.store_capabilities(snap)))
    }

    async fn exec_set_capability(
        &self,
        id: String,
        enabled: bool,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let data = self
            .unary_cmd(Command::SetCapability { id, enabled })
            .await?;
        let snap: xylitol_plugin_sdk::session::CapabilitySnapshot =
            serde_json::from_value(data).map_err(|e| XyDriverError::remote(e.to_string()))?;
        Ok(DispatchOutcome::Capabilities(self.store_capabilities(snap)))
    }

    async fn exec_clear_capabilities(&self) -> Result<DispatchOutcome, XyDriverError> {
        let data = self.unary_cmd(Command::ClearCapabilities {}).await?;
        let snap: xylitol_plugin_sdk::session::CapabilitySnapshot =
            serde_json::from_value(data).map_err(|e| XyDriverError::remote(e.to_string()))?;
        Ok(DispatchOutcome::Capabilities(self.store_capabilities(snap)))
    }

    async fn exec_loaded_resources(&self) -> Result<DispatchOutcome, XyDriverError> {
        Ok(DispatchOutcome::LoadedResources(
            crate::driver::XyDriver::loaded_resources_snapshot(self).await,
        ))
    }

    async fn exec_get_queue_stats(&mut self) -> Result<DispatchOutcome, XyDriverError> {
        let data = self.unary_cmd(Command::GetQueueStats {}).await?;
        self.cache_queue_from_value(&data);
        Ok(self.queue_outcome())
    }

    async fn exec_steer(&mut self, message: String) -> Result<DispatchOutcome, XyDriverError> {
        let data = self.unary_cmd(Command::Steer { message }).await?;
        self.cache_queue_from_value(&data);
        Ok(self.queue_outcome())
    }

    async fn exec_follow_up(&mut self, message: String) -> Result<DispatchOutcome, XyDriverError> {
        let data = self.unary_cmd(Command::FollowUp { message }).await?;
        self.cache_queue_from_value(&data);
        Ok(self.queue_outcome())
    }

    async fn exec_clear_queue(
        &mut self,
        clear_steer: bool,
        clear_follow_up: bool,
    ) -> Result<DispatchOutcome, XyDriverError> {
        let data = self
            .unary_cmd(Command::ClearQueue {
                clear_steer,
                clear_follow_up,
            })
            .await?;
        self.cache_queue_from_value(&data);
        Ok(self.queue_outcome())
    }

    fn apply_model_selection(&self, selected: ModelInfo) {
        let default = selected
            .thinking_levels
            .last()
            .cloned()
            .unwrap_or_else(|| THINKING_OFF.into());
        *self.thinking.lock().unwrap() = default;
        self.cache_model(selected);
    }

    fn decode_model_selection(data: &Value, fallback_id: &str) -> Result<ModelInfo, XyDriverError> {
        if data.get("id").is_some() {
            Self::model_from_value(data)
        } else {
            Ok(ModelInfo {
                id: data
                    .get("model")
                    .and_then(|model| model.as_str())
                    .unwrap_or(fallback_id)
                    .to_string(),
                display_name: data
                    .get("display_name")
                    .and_then(|name| name.as_str())
                    .unwrap_or("")
                    .to_string(),
                thinking: false,
                thinking_levels: Vec::new(),
                context_window: 0,
            })
        }
    }

    fn bind_new_session(&mut self, id: String) -> String {
        self.session_id = id.clone();
        if let Ok(mut leaf) = self.leaf_entry_id.lock() {
            *leaf = None;
        }
        id
    }

    fn queue_outcome(&self) -> DispatchOutcome {
        let stats = self.queue_stats();
        DispatchOutcome::QueueStats {
            steer_count: stats.steer_count,
            follow_up_count: stats.follow_up_count,
        }
    }
}

fn plan_mode_outcome(data: &Value) -> DispatchOutcome {
    DispatchOutcome::PlanMode {
        mode: data
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("execute")
            .to_string(),
        plan_id: data
            .get("plan_id")
            .and_then(Value::as_str)
            .map(str::to_string),
    }
}

fn session_name_outcome(data: &Value, missing: &str) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::SessionName(Some(
        data.get("name")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .ok_or_else(|| XyDriverError::remote(missing))?,
    )))
}
