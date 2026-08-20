//! Host occupancy: one process-wide [`RuntimePorts`] baseline, lazy session slots.
//!
//! Occupancy unit is a **session slot** (journal + seq + writer engine). A
//! single Driver still must not run two root submits or two session bindings
//! at once.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use futures::StreamExt;
use serde_json::{Value, json};
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::time::timeout;

use crate::agent::RuntimePorts;
use crate::app::core::composition::{
    BuildAgentOptions, McpServerSpec, build_ports, build_ports_with_store,
};
use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::core::driver::{ModelInfo, XyDriver, XyInProcessDriver};
use crate::app::core::driver_error::XyDriverError;
use crate::app::server::ws::{
    EventJournal, ReverseRpcGateway, ReverseRpcResult, downlink_server_request,
};
use crate::infra::session::SessionManager;
use crate::protocol::Command;
use crate::protocol::error::XyToolError;
use crate::protocol::lifecycle::XyEvent;
use crate::protocol::ports::XySessionStore;
use crate::protocol::ports::ask::{AskArgs, AskUserGateway};
use crate::protocol::wire::envelope::{
    ApprovalRequestedPayload, HostDescribeValue, PROTOCOL_VERSION, QuestionRequestedPayload,
    RpcMessage, RpcResult, SessionEventPayload, SessionResyncRequiredPayload,
    SessionSubscribedPayload,
};
use crate::protocol::wire::method::is_unary_method;

/// Bounded per-mux-connection queue. Overflow → `session/resync_required` or drop conn.
pub const MUX_CHAN_CAP: usize = 256;

const WRITER_METHODS: &[&str] = &[
    "prompt",
    "abort",
    "set_model",
    "cycle_model",
    "set_thinking_level",
    "bash",
    "compact",
    "export_html",
    "export_jsonl",
    "import_jsonl",
    "switch_session",
    "fork",
    "steer",
    "follow_up",
    "clear_queue",
];

/// Shared Host process state (salvo Depot).
pub struct HostState {
    pub ports: RuntimePorts,
    pub reload: ReloadBaseline,
    pub sessions: RwLock<HashMap<String, Arc<SessionSlot>>>,
    /// Mux connections not yet bound by unary `subscribe`.
    pub unbound_mux: Mutex<Vec<MuxSink>>,
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
}

pub type MuxSink = mpsc::Sender<RpcMessage>;

/// Per-session occupancy: journal + optional writer Driver + mux subscribers.
pub struct SessionSlot {
    pub session_id: String,
    pub journal: Mutex<EventJournal>,
    pub driver: Arc<Mutex<Option<XyInProcessDriver>>>,
    pub writer: AtomicBool,
    pub subscribers: Mutex<HashMap<u64, MuxSink>>,
    pub gateway: Arc<ReverseRpcGateway>,
    next_conn: AtomicU64,
    resync_offered: AtomicBool,
}

impl HostState {
    pub fn new(ports: RuntimePorts, reload: ReloadBaseline, fallback_session: String) -> Arc<Self> {
        Arc::new(Self {
            ports,
            reload,
            sessions: RwLock::new(HashMap::new()),
            unbound_mux: Mutex::new(Vec::new()),
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
            },
            "test-session".into(),
        ))
    }

    /// Default production ports from [`build_ports`] (caller supplies store via composition).
    pub fn from_default_ports(
        reload: ReloadBaseline,
        fallback_session: String,
    ) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let ports = build_ports(BuildAgentOptions::default())?;
        Ok(Self::from_bootstrap(ports, reload, fallback_session))
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
            .or_insert_with(|| SessionSlot::new(session_id))
            .clone()
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
}

impl SessionSlot {
    pub fn new(session_id: impl Into<String>) -> Arc<Self> {
        let session_id = session_id.into();
        Arc::new(Self {
            journal: Mutex::new(EventJournal::with_default_capacity(&session_id)),
            session_id,
            driver: Arc::new(Mutex::new(None)),
            writer: AtomicBool::new(false),
            subscribers: Mutex::new(HashMap::new()),
            gateway: Arc::new(ReverseRpcGateway::new()),
            next_conn: AtomicU64::new(1),
            resync_offered: AtomicBool::new(false),
        })
    }

    pub async fn add_subscriber(&self, tx: MuxSink) -> u64 {
        let id = self.next_conn.fetch_add(1, Ordering::Relaxed);
        self.subscribers.lock().await.insert(id, tx);
        id
    }

    pub async fn remove_subscriber(&self, id: u64) {
        self.subscribers.lock().await.remove(&id);
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
    WRITER_METHODS.contains(&method)
}

pub async fn materialize_writer(
    host: &HostState,
    slot: &Arc<SessionSlot>,
) -> Result<(), XyDriverError> {
    let mut guard = slot.driver.lock().await;
    if guard.is_some() {
        return Ok(());
    }
    let mut agent = host.ports.materialize_runtime();
    agent
        .bind_session(slot.session_id.clone())
        .map_err(|e| XyDriverError::from(e.to_string()))?;
    let mut driver = XyInProcessDriver::new(agent, host.ports.store.clone());
    driver.enable_reload_state(
        host.reload.cwd.clone(),
        host.reload.agent_dir.clone(),
        host.reload.project_trusted,
        host.reload.mcp_servers.clone(),
    );
    driver.install_ask_tool(Arc::new(SlotAskGateway { slot: slot.clone() }));
    driver.begin_mcp_bootstrap().await;
    driver.wait_mcp_bootstrap().await;
    *guard = Some(driver);
    slot.writer.store(true, Ordering::SeqCst);
    Ok(())
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

pub fn command_from_method(method: &str, payload: &Value) -> Result<Command, String> {
    let p = payload.clone();
    match method {
        "abort" => Ok(Command::Abort { id: None }),
        "get_state" => Ok(Command::GetState { id: None }),
        "set_model" => Ok(Command::SetModel {
            id: None,
            provider: p
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            model_id: p
                .get("model_id")
                .and_then(|v| v.as_str())
                .ok_or("missing model_id")?
                .to_string(),
        }),
        "cycle_model" => Ok(Command::CycleModel { id: None }),
        "get_available_models" => Ok(Command::GetAvailableModels { id: None }),
        "set_thinking_level" => Ok(Command::SetThinkingLevel {
            id: None,
            level: p
                .get("level")
                .and_then(|v| v.as_str())
                .ok_or("missing level")?
                .to_string(),
        }),
        "bash" => Ok(Command::Bash {
            id: None,
            command: p
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or("missing command")?
                .to_string(),
            exclude_from_context: p
                .get("exclude_from_context")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        }),
        "compact" => Ok(Command::Compact {
            id: None,
            instructions: p
                .get("instructions")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        }),
        "get_session_stats" => Ok(Command::GetSessionStats { id: None }),
        "export_html" => Ok(Command::ExportHtml {
            id: None,
            output_path: p
                .get("output_path")
                .or_else(|| p.get("path"))
                .and_then(|v| v.as_str())
                .map(str::to_string),
        }),
        "export_jsonl" => Ok(Command::ExportJsonl {
            id: None,
            output_path: p
                .get("output_path")
                .or_else(|| p.get("path"))
                .and_then(|v| v.as_str())
                .map(str::to_string),
        }),
        "import_jsonl" => Ok(Command::ImportJsonl {
            id: None,
            input_path: p
                .get("input_path")
                .or_else(|| p.get("path"))
                .and_then(|v| v.as_str())
                .ok_or("missing input_path")?
                .to_string(),
        }),
        "switch_session" => Ok(Command::SwitchSession {
            id: None,
            session_path: p
                .get("session_id")
                .or_else(|| p.get("session_path"))
                .and_then(|v| v.as_str())
                .ok_or("missing session_id")?
                .to_string(),
        }),
        "fork" => Ok(Command::Fork {
            id: None,
            entry_id: p
                .get("entry_id")
                .and_then(|v| v.as_str())
                .ok_or("missing entry_id")?
                .to_string(),
            position: p
                .get("position")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        }),
        "get_messages" => Ok(Command::GetMessages { id: None }),
        "get_commands" => Ok(Command::GetCommands { id: None }),
        "steer" => Ok(Command::Steer {
            id: None,
            message: p
                .get("message")
                .and_then(|v| v.as_str())
                .ok_or("missing message")?
                .to_string(),
        }),
        "follow_up" => Ok(Command::FollowUp {
            id: None,
            message: p
                .get("message")
                .and_then(|v| v.as_str())
                .ok_or("missing message")?
                .to_string(),
        }),
        "clear_queue" => Ok(Command::ClearQueue {
            id: None,
            clear_steer: p
                .get("clear_steer")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            clear_follow_up: p
                .get("clear_follow_up")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        }),
        other => Err(format!("unmapped unary {other}")),
    }
}

fn rpc_err(e: XyDriverError) -> RpcResult {
    RpcResult::error(e.kind(), e.to_string())
}

/// Dispatch a registered unary method against the session slot.
pub async fn handle_unary(host: &Arc<HostState>, method: &str, payload: Value) -> RpcResult {
    if method == "host.describe" {
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

    let session_id = if method == "subscribe" {
        payload
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    } else {
        host.session_id_from_payload(&payload)
    };

    if method == "subscribe" {
        if session_id.is_empty() {
            return RpcResult::error("invalid_input", "subscribe requires session_id");
        }
        let last_seq = payload
            .get("last_seq")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let slot = host.slot(&session_id).await;
        return host.bind_mux_to_session(&slot, last_seq).await;
    }

    let slot = host.slot(&session_id).await;

    if method == "prompt" {
        let message = payload
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if message.is_empty() {
            return RpcResult::error("invalid_input", "missing message");
        }
        if let Err(e) = materialize_writer(host, &slot).await {
            return rpc_err(e);
        }
        let driver = slot.driver.clone();
        let slot_push = slot.clone();
        tokio::spawn(async move {
            let stream = {
                let mut g = driver.lock().await;
                let Some(d) = g.as_mut() else {
                    return;
                };
                d.run(&message).await
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
        });
        return RpcResult::ok_value(json!({ "session_id": session_id }));
    }

    if is_writer_method(method)
        && let Err(e) = materialize_writer(host, &slot).await
    {
        return rpc_err(e);
    }

    if method == "get_available_models" && slot.driver.lock().await.is_none() {
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
        // Read-only with no writer: empty snapshot, do not materialize.
        return match method {
            "get_state" => RpcResult::ok_value(json!({
                "session_id": session_id,
                "model": Value::Null,
                "thinking_level": Value::Null,
            })),
            "get_messages" => RpcResult::ok_value(json!({ "entries": [] })),
            "get_commands" => RpcResult::ok_value(json!({ "commands": [] })),
            "get_session_stats" => RpcResult::ok_value(json!({
                "session_id": session_id,
                "user_messages": 0,
                "assistant_messages": 0,
                "total_messages": 0,
            })),
            _ => RpcResult::ok_value(json!({})),
        };
    }

    let cmd = match command_from_method(method, &payload) {
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
