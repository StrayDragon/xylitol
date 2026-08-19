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

use crate::app::core::host_client::{HostClient, HttpWsClient};
use crate::protocol::model::THINKING_OFF;
use crate::protocol::ports::XyBashResult;
use crate::protocol::session::{SessionEntry, SessionTreeKind, SessionTreeNode, SessionTreeTravel};
use crate::protocol::wire::envelope::PROTOCOL_VERSION;
use crate::protocol::{Event, RpcMessage};

use super::XyDriver;
use super::XyDriverError;
use super::types::{
    CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot, ModelInfo, QueueStats,
    ReloadStepReport, RuntimeReloadReport, SessionListEntry, SessionStats, XyEvent,
    estimate_from_session_entries,
};

/// Notify the product TUI of mux reverse-RPC (approval/question).
pub type ReverseRpcNotify = Arc<dyn Fn(String, String, Value) + Send + Sync>;

#[derive(Clone)]
struct SharedDownlink {
    events: Arc<std::sync::Mutex<VecDeque<XyEvent>>>,
    notify: Arc<Notify>,
    started: Arc<AtomicBool>,
}

impl SharedDownlink {
    fn new() -> Self {
        Self {
            events: Arc::new(std::sync::Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
            started: Arc::new(AtomicBool::new(false)),
        }
    }

    fn push(&self, ev: XyEvent) {
        if let Ok(mut q) = self.events.lock() {
            q.push_back(ev);
        }
        self.notify.notify_waiters();
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
    handshake_done: Arc<AtomicBool>,
    reverse_rpc: Option<ReverseRpcNotify>,
    downlink: SharedDownlink,
    resync_needed: Arc<AtomicBool>,
    subscribed_ok: Arc<AtomicBool>,
    cached_model: Arc<std::sync::Mutex<Option<ModelInfo>>>,
    cached_models: Arc<std::sync::Mutex<Option<Vec<ModelInfo>>>>,
    cached_commands: Arc<std::sync::Mutex<Option<Vec<CommandInfo>>>>,
    cached_queue: Arc<std::sync::Mutex<QueueStats>>,
    cached_resources: Arc<std::sync::Mutex<LoadedResourcesSnapshot>>,
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
            handshake_done: Arc::new(AtomicBool::new(false)),
            reverse_rpc: None,
            downlink: SharedDownlink::new(),
            resync_needed: Arc::new(AtomicBool::new(false)),
            subscribed_ok: Arc::new(AtomicBool::new(false)),
            cached_model: Arc::new(std::sync::Mutex::new(None)),
            cached_models: Arc::new(std::sync::Mutex::new(None)),
            cached_commands: Arc::new(std::sync::Mutex::new(None)),
            cached_queue: Arc::new(std::sync::Mutex::new(QueueStats::default())),
            cached_resources: Arc::new(std::sync::Mutex::new(LoadedResourcesSnapshot::default())),
        }
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

    fn restart_downlink(&mut self) {
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
        let host = self.host.clone();
        let session_id = self.session_id.clone();
        let last_seq = self.last_seq.clone();
        let handshake_done = self.handshake_done.clone();
        let reverse_rpc = self.reverse_rpc.clone();
        let downlink = self.downlink.clone();
        let life = self.session_life.clone();
        let resync_needed = self.resync_needed.clone();
        let subscribed_ok = self.subscribed_ok.clone();
        let cached_queue = self.cached_queue.clone();
        tokio::spawn(async move {
            let mut backoff = Duration::from_millis(200);
            loop {
                if life.is_cancelled() {
                    break;
                }
                match host.mux().await {
                    Ok(mut mux) => {
                        backoff = Duration::from_millis(200);
                        if !handshake_done.load(Ordering::SeqCst) {
                            match host.unary("host.describe", serde_json::json!({})).await {
                                Ok(result) => {
                                    let proto = result
                                        .value
                                        .as_ref()
                                        .and_then(|v| v.get("protocol"))
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0);
                                    if proto != PROTOCOL_VERSION as u64 {
                                        downlink.push(XyEvent::error_msg(format!(
                                            "host protocol {proto} != {PROTOCOL_VERSION}"
                                        )));
                                        return;
                                    }
                                    handshake_done.store(true, Ordering::SeqCst);
                                }
                                Err(e) => {
                                    downlink.push(XyEvent::error_msg(format!(
                                        "host.describe failed: {e}"
                                    )));
                                    tokio::select! {
                                        _ = life.cancelled() => return,
                                        _ = tokio::time::sleep(backoff) => {}
                                    }
                                    backoff = (backoff * 2).min(Duration::from_secs(5));
                                    continue;
                                }
                            }
                        }
                        let seq = last_seq.load(Ordering::SeqCst);
                        if let Err(e) = host
                            .unary(
                                "subscribe",
                                serde_json::json!({
                                    "session_id": session_id,
                                    "last_seq": seq,
                                }),
                            )
                            .await
                        {
                            downlink.push(XyEvent::error_msg(format!("subscribe failed: {e}")));
                            tokio::select! {
                                _ = life.cancelled() => return,
                                _ = tokio::time::sleep(backoff) => {}
                            }
                            backoff = (backoff * 2).min(Duration::from_secs(5));
                            continue;
                        }
                        subscribed_ok.store(true, Ordering::SeqCst);
                        loop {
                            tokio::select! {
                                _ = life.cancelled() => return,
                                msg = mux.next() => {
                                    match msg {
                                        Some(Ok(RpcMessage::ServerRequest { method, payload, .. }))
                                            if method == "session/event" =>
                                        {
                                            if let Some(s) = payload.get("seq").and_then(Value::as_u64) {
                                                last_seq.store(s, Ordering::SeqCst);
                                            }
                                            let event_val =
                                                payload.get("event").cloned().unwrap_or(payload);
                                            let Ok(ev) = serde_json::from_value::<Event>(event_val) else {
                                                continue;
                                            };
                                            let Ok(agent_event) = XyEvent::try_from(&ev) else {
                                                continue;
                                            };
                                            if let XyEvent::QueueUpdate {
                                                steer_count,
                                                follow_up_count,
                                            } = &agent_event
                                                && let Ok(mut cached) = cached_queue.lock()
                                            {
                                                *cached = QueueStats {
                                                    steer_count: *steer_count,
                                                    follow_up_count: *follow_up_count,
                                                };
                                            }
                                            downlink.push(agent_event);
                                        }
                                        Some(Ok(RpcMessage::ServerRequest { method, .. }))
                                            if method == "session/resync_required" =>
                                        {
                                            last_seq.store(0, Ordering::SeqCst);
                                            resync_needed.store(true, Ordering::SeqCst);
                                            let _ = host
                                                .unary(
                                                    "subscribe",
                                                    serde_json::json!({
                                                        "session_id": session_id,
                                                        "last_seq": 0,
                                                    }),
                                                )
                                                .await;
                                        }
                                        Some(Ok(RpcMessage::ServerRequest { rpc_id, method, payload }))
                                            if method == "approval/requested"
                                                || method == "question/requested" =>
                                        {
                                            if let Some(notify) = &reverse_rpc {
                                                notify(rpc_id, method, payload);
                                            }
                                        }
                                        Some(Ok(_)) => {}
                                        Some(Err(e)) => {
                                            downlink.push(XyEvent::error_msg(e.to_string()));
                                            break;
                                        }
                                        None => break,
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        downlink.push(XyEvent::error_msg(format!("WS connect failed: {e}")));
                        tokio::select! {
                            _ = life.cancelled() => return,
                            _ = tokio::time::sleep(backoff) => {}
                        }
                        backoff = (backoff * 2).min(Duration::from_secs(5));
                    }
                }
            }
        });
    }

    async fn wait_subscribed(&self) -> Result<(), XyDriverError> {
        let wait_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !self.subscribed_ok.load(Ordering::SeqCst) {
            if tokio::time::Instant::now() > wait_deadline {
                return Err(XyDriverError::remote("subscribe timed out"));
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok(())
    }

    async fn refresh_chrome_caches(&self) -> Result<(), XyDriverError> {
        if let Ok(data) = self.unary("get_state", serde_json::json!({})).await {
            self.update_leaf_from_state(&data);
            if let Some(m) = data.get("model").filter(|m| !m.is_null())
                && let Ok(model) = Self::model_from_value(m)
            {
                self.cache_model(model);
            }
        }
        if let Ok(data) = self
            .unary("get_available_models", serde_json::json!({}))
            .await
        {
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
        if let Ok(data) = self.unary("get_commands", serde_json::json!({})).await {
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
        let snap = match self.unary("loaded_resources", serde_json::json!({})).await {
            Ok(data) => serde_json::from_value(data).unwrap_or_default(),
            Err(_) => LoadedResourcesSnapshot::default(),
        };
        if let Ok(mut cached) = self.cached_resources.lock() {
            *cached = snap;
        }
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

    fn block_on<T>(
        &self,
        fut: impl std::future::Future<Output = Result<T, XyDriverError>>,
    ) -> Result<T, XyDriverError> {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
            Err(_) => tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| XyDriverError::io(e.to_string()))?
                .block_on(fut),
        }
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
        if !self.handshake_done.load(Ordering::SeqCst) {
            let data = self.unary("host.describe", serde_json::json!({})).await?;
            let proto = data.get("protocol").and_then(Value::as_u64).unwrap_or(0);
            if proto != PROTOCOL_VERSION as u64 {
                return Err(XyDriverError::remote(format!(
                    "host protocol {proto} != {PROTOCOL_VERSION}"
                )));
            }
            self.handshake_done.store(true, Ordering::SeqCst);
        }
        self.ensure_downlink();
        self.wait_subscribed().await?;
        self.refresh_chrome_caches().await
    }

    fn drain_idle_events(&mut self) -> Vec<XyEvent> {
        self.downlink.drain()
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

        let stream = async_stream::stream! {
            if let Err(e) = host
                .unary(
                    "prompt",
                    serde_json::json!({
                        "message": prompt,
                        "session_id": session_id,
                    }),
                )
                .await
            {
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
        if let Ok(cached) = self.cached_model.lock()
            && cached.is_some()
        {
            return cached.clone();
        }
        self.block_on(async {
            let data = self.unary("get_state", serde_json::json!({})).await?;
            self.update_leaf_from_state(&data);
            match data.get("model") {
                Some(m) if !m.is_null() => Self::model_from_value(m).map(Some),
                _ => Ok(None),
            }
        })
        .ok()
        .flatten()
        .inspect(|model| self.cache_model(model.clone()))
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        if let Ok(cached) = self.cached_models.lock()
            && let Some(models) = cached.as_ref()
        {
            return models.clone();
        }
        self.block_on(async {
            let data = self
                .unary("get_available_models", serde_json::json!({}))
                .await?;
            let arr = data
                .get("models")
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();
            arr.iter().map(Self::model_from_value).collect()
        })
        .inspect(|models: &Vec<ModelInfo>| {
            if let Ok(mut cached) = self.cached_models.lock() {
                *cached = Some(models.clone());
            }
        })
        .unwrap_or_default()
    }

    async fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, XyDriverError> {
        let model_id = model_id.to_string();
        let data = self
            .unary(
                "set_model",
                serde_json::json!({ "provider": "", "model_id": model_id }),
            )
            .await?;
        // Endpoint returns { model, display_name }; enrich via list if needed.
        let selected = if data.get("id").is_some() {
            Self::model_from_value(&data)?
        } else {
            ModelInfo {
                id: data
                    .get("model")
                    .and_then(|m| m.as_str())
                    .unwrap_or(&model_id)
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
        let data = self.unary("cycle_model", serde_json::json!({})).await?;
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
        self.unary("set_thinking_level", serde_json::json!({ "level": level }))
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
            .unary(
                "bash",
                serde_json::json!({
                    "command": command,
                    "exclude_from_context": exclude_from_context,
                }),
            )
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
        let data = self
            .unary(
                "compact",
                serde_json::json!({ "instructions": instructions }),
            )
            .await?;
        Ok(data
            .get("compacted")
            .and_then(|c| c.as_bool())
            .unwrap_or(false))
    }

    async fn export_html(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let data = self.unary("export_html", serde_json::json!({})).await?;
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
        let data = self.unary("export_jsonl", serde_json::json!({})).await?;
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
            .unary(
                "fork",
                serde_json::json!({
                    "entry_id": entry_id,
                    "position": match position {
                        crate::protocol::session::ForkPosition::At => "at",
                        crate::protocol::session::ForkPosition::Before => "before",
                    },
                }),
            )
            .await?;
        Ok(data
            .get("session_id")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string())
    }

    async fn switch_session(&mut self, session_id: &str) -> Result<String, XyDriverError> {
        let data = self
            .unary(
                "switch_session",
                serde_json::json!({ "session_id": session_id }),
            )
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
        let data = self.unary("get_messages", serde_json::json!({})).await?;
        let entries = data
            .get("entries")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        serde_json::from_value(entries).map_err(|e| XyDriverError::remote(e.to_string()))
    }

    async fn get_session_stats(&self) -> Result<SessionStats, XyDriverError> {
        let data = self
            .unary("get_session_stats", serde_json::json!({}))
            .await?;
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
        if let Ok(cached) = self.cached_commands.lock()
            && let Some(cmds) = cached.as_ref()
        {
            return cmds.clone();
        }
        self.block_on(async {
            let data = self.unary("get_commands", serde_json::json!({})).await?;
            let arr = data
                .get("commands")
                .and_then(|c| c.as_array())
                .cloned()
                .unwrap_or_default();
            Ok(arr
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
                .collect())
        })
        .inspect(|cmds: &Vec<CommandInfo>| {
            if let Ok(mut cached) = self.cached_commands.lock() {
                *cached = Some(cmds.clone());
            }
        })
        .unwrap_or_default()
    }

    fn steer(&mut self, message: &str) -> Result<(), XyDriverError> {
        self.block_on(async {
            let data = self
                .unary("steer", serde_json::json!({ "message": message }))
                .await?;
            self.cache_queue_from_value(&data);
            Ok(())
        })
    }

    fn follow_up(&mut self, message: &str) -> Result<(), XyDriverError> {
        self.block_on(async {
            let data = self
                .unary("follow_up", serde_json::json!({ "message": message }))
                .await?;
            self.cache_queue_from_value(&data);
            Ok(())
        })
    }

    fn clear_queue(
        &mut self,
        clear_steer: bool,
        clear_follow_up: bool,
    ) -> Result<(), XyDriverError> {
        self.block_on(async {
            let data = self
                .unary(
                    "clear_queue",
                    serde_json::json!({
                        "clear_steer": clear_steer,
                        "clear_follow_up": clear_follow_up,
                    }),
                )
                .await?;
            self.cache_queue_from_value(&data);
            Ok(())
        })
    }

    fn queue_stats(&self) -> QueueStats {
        self.cached_queue.lock().map(|s| *s).unwrap_or_default()
    }

    async fn session_tree(
        &self,
        kind: SessionTreeKind,
    ) -> Result<Vec<SessionTreeNode>, XyDriverError> {
        let data = self
            .unary(
                "session_tree",
                serde_json::json!({
                    "kind": serde_json::to_value(kind).unwrap_or(Value::Null),
                }),
            )
            .await?;
        serde_json::from_value(data.get("tree").cloned().unwrap_or(Value::Null))
            .map_err(|e| XyDriverError::remote(e.to_string()))
    }

    async fn travel_session_tree(
        &self,
        kind: SessionTreeKind,
        entry_id: &str,
    ) -> Result<SessionTreeTravel, XyDriverError> {
        let data = self
            .unary(
                "travel_session_tree",
                serde_json::json!({
                    "kind": serde_json::to_value(kind).unwrap_or(Value::Null),
                    "entry_id": entry_id,
                }),
            )
            .await?;
        serde_json::from_value(data).map_err(|e| XyDriverError::remote(e.to_string()))
    }

    async fn append_entry_label(
        &mut self,
        target_id: &str,
        label: Option<&str>,
    ) -> Result<(), XyDriverError> {
        self.unary(
            "append_entry_label",
            serde_json::json!({
                "target_id": target_id,
                "label": label,
            }),
        )
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
        let data = self.unary("list_sessions", serde_json::json!({})).await?;
        serde_json::from_value(data.get("sessions").cloned().unwrap_or(Value::Null))
            .map_err(|e| XyDriverError::remote(e.to_string()))
    }

    async fn load_session_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XyDriverError> {
        let data = self
            .unary(
                "load_session_entries",
                serde_json::json!({ "session_id": session_id }),
            )
            .await?;
        serde_json::from_value(data.get("entries").cloned().unwrap_or(Value::Null))
            .map_err(|e| XyDriverError::remote(e.to_string()))
    }

    async fn new_session(&mut self) -> Result<String, XyDriverError> {
        let data = self.unary("new_session", serde_json::json!({})).await?;
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
        let data = self
            .unary("get_session_name", serde_json::json!({}))
            .await?;
        Ok(data
            .get("name")
            .and_then(|value| value.as_str())
            .map(str::to_string))
    }

    async fn set_session_name(&mut self, name: &str) -> Result<String, XyDriverError> {
        let data = self
            .unary("set_session_name", serde_json::json!({ "name": name }))
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
            .unary(
                "set_session_name_for",
                serde_json::json!({
                    "session_id": session_id,
                    "name": name,
                }),
            )
            .await?;
        data.get("name")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .ok_or_else(|| XyDriverError::remote("set_session_name_for response missing name"))
    }

    async fn delete_session(&mut self, session_id: &str) -> Result<(), XyDriverError> {
        self.unary(
            "delete_session",
            serde_json::json!({ "session_id": session_id }),
        )
        .await?;
        Ok(())
    }

    async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
        let snap = match self.unary("loaded_resources", serde_json::json!({})).await {
            Ok(data) => serde_json::from_value(data).unwrap_or_else(|e| LoadedResourcesSnapshot {
                mcp_diag_short: vec![format!("remote loaded_resources decode: {e}")],
                ..LoadedResourcesSnapshot::default()
            }),
            Err(e) => LoadedResourcesSnapshot {
                mcp_diag_short: vec![format!("remote loaded_resources: {e}")],
                ..LoadedResourcesSnapshot::default()
            },
        };
        if let Ok(mut cached) = self.cached_resources.lock() {
            *cached = snap.clone();
        }
        snap
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

    async fn arm_tool_freeze_gate(&mut self) {
        let data = match self.unary("arm_tool_freeze", serde_json::json!({})).await {
            Ok(data) => data,
            Err(e) => {
                e.log_failure("remote.arm_tool_freeze");
                return;
            }
        };
        if let Ok(snap) = serde_json::from_value::<LoadedResourcesSnapshot>(data)
            && let Ok(mut cached) = self.cached_resources.lock()
        {
            *cached = snap;
        }
    }

    async fn poll_mcp_bootstrap(&mut self) -> bool {
        let prev = self
            .cached_resources
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();
        if prev.mcp_configured == 0 {
            return false;
        }
        if prev.tools_table_frozen
            && prev.mcp_bootstrap_complete
            && prev.mcp_connecting_label.is_none()
        {
            return false;
        }
        true
    }

    async fn reload_runtime(
        &mut self,
        _cancel: &CancellationToken,
    ) -> Result<RuntimeReloadReport, XyDriverError> {
        let data = self.unary("reload", serde_json::json!({})).await?;
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

    #[tokio::test]
    async fn minted_session_subscribe_does_not_require_cli_flag() {
        let host = HostState::for_test().expect("host");
        let (running, port) = serve(
            ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
                sessions_dir: None,
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
        driver.steer("nudge").expect("steer");
        assert_eq!(driver.queue_stats().steer_count, 1);
        let queued = driver
            .unary("queue_stats", serde_json::json!({}))
            .await
            .expect("queue_stats unary");
        assert_eq!(queued.get("steer_count").and_then(Value::as_u64), Some(1));
        driver.clear_queue(true, true).expect("clear queue");
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
}
