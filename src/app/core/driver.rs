//! Driver abstraction — the single dependency of interactive layers.
//!
//! All interactive clients (cli/print/tui) interact with the core through a
//! [`Driver`]; they import agent symbols only from `agent` (mod-level),
//! never reaching into `agent::session`/`agent::runtime` internals or `infra`.
//!
//! - [`InProcessDriver`]: wraps the local agent module (composition root wires
//!   ports and agent together).
//! - [`RemoteDriver`]: speaks the protocol over REST/WS to a remote server.
//!
//! The trait carries not just `run`/`abort` but the full set of command
//! execution semantics (model selection, compaction, export, session ops) so
//! that [`crate::app::core::dispatch`] can be a pure Command→method dispatcher
//! shared by tui (spec ce10). WS-transport-specific commands
//! (Subscribe/ApproveTool/AnswerQuestion) do NOT live here — they stay in
//! `app::server::ws`.
//!
//! NOTE: many trait methods (compact/export_*/get_messages/...) are consumed
//! only when the tui feature is on (via dispatch); under default features they
//! appear unused. ceiling: never consumed without tui. upgrade: tui becomes
//! default or another surface consumes dispatch.

#![allow(dead_code)]

use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use tokio_util::sync::CancellationToken;

use crate::agent::ReActAgent;
use crate::domain::lifecycle::XyEvent;
use crate::domain::session_types::SessionEntry;
use crate::domain::types::{ThinkingLevel, XyModelMeta};
use crate::runtime_protocol::{XyBashResult, XySessionStore};

/// Re-export so Driver implementors under surfaces can name the return type
/// without importing `crate::agent::session` directly (which arch_guard
/// forbids for tui/). Surfaces reference this as
/// `crate::app::core::driver::SessionStats`.
pub use crate::agent::session::SessionStats;

#[cfg(feature = "server")]
use futures::{SinkExt, StreamExt};
#[cfg(feature = "server")]
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[cfg(feature = "server")]
use crate::app::server::ws::{ClientFrame, ServerFrame};

/// A stream of [`XyEvent`] items.
pub type EventStream = Pin<Box<dyn Stream<Item = XyEvent> + Send>>;

/// Minimal info about a slash command (for `GetCommands`), decoupled from the
/// agent's internal `SlashCommandInfo` so the Driver trait does not leak
/// `pub(crate)` agent types.
#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub name: String,
    pub description: String,
}

/// A snapshot of session state (for `GetState`), UI/transport-agnostic.
#[derive(Debug, Clone)]
pub struct SessionState {
    pub session_id: String,
    pub model: Option<ModelInfo>,
    pub thinking_level: ThinkingLevel,
}

/// Minimal model info returned by the Driver, decoupled from `XyModelMeta`'s
/// many fields so callers only see what command dispatch needs.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub thinking: bool,
    pub context_window: u64,
}

impl From<&XyModelMeta> for ModelInfo {
    fn from(m: &XyModelMeta) -> Self {
        Self {
            id: m.id.clone(),
            display_name: m.display_name.clone(),
            thinking: m.thinking,
            context_window: m.context_window,
        }
    }
}

/// Driver — interact with the core without knowing its internals.
///
/// [`InProcessDriver`] keeps a cached `ReActAgent` and is the local
/// (single-process) implementation. [`RemoteDriver`] speaks the protocol over
/// WS/REST to a xylitol server.
#[async_trait]
pub trait Driver: Send {
    /// Submit a prompt and receive a stream of events.
    async fn run(&mut self, prompt: &str) -> EventStream;

    /// Cancel the current turn.
    fn abort(&self);

    /// Currently selected model, if any.
    fn current_model(&self) -> Option<ModelInfo>;

    /// All registered models.
    fn available_models(&self) -> Vec<ModelInfo>;

    /// Select a model by id (exact match on `id` or `config.model`).
    /// Returns the selected model on success.
    fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, String>;

    /// Cycle to the next model in the registry. Returns the newly-selected model.
    fn cycle_model(&mut self) -> Result<ModelInfo, String>;

    /// Set the thinking level.
    fn set_thinking_level(&mut self, level: ThinkingLevel);

    /// Current thinking level.
    fn thinking_level(&self) -> ThinkingLevel;

    /// Current session id (the id the next `run`/export acts on).
    fn session_id(&self) -> Option<String>;

    /// Snapshot of session state for `GetState`.
    fn get_state(&self) -> SessionState {
        SessionState {
            session_id: self.session_id().unwrap_or_default(),
            model: self.current_model(),
            thinking_level: self.thinking_level(),
        }
    }

    /// Execute a bash command (the `Bash` Command variant).
    async fn execute_bash(
        &mut self,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<XyBashResult, String>;

    /// Run auto-compaction. Returns whether a compaction occurred.
    async fn compact(&mut self) -> Result<bool, String>;

    /// Export the session to HTML at `path`. Returns the path used.
    async fn export_html(&mut self, path: &Path) -> Result<String, String>;

    /// Export the session to JSONL at `path`. Returns the path used.
    async fn export_jsonl(&mut self, path: &Path) -> Result<String, String>;

    /// Import a JSONL file. Returns the new session id.
    async fn import_jsonl(&mut self, path: &Path) -> Result<String, String>;

    /// Fork the current session at `entry_id`. Returns the new session id.
    async fn fork_session(&mut self, entry_id: &str) -> Result<String, String>;

    /// Switch to an existing session id. Validates existence first.
    async fn switch_session(&mut self, session_id: &str) -> Result<String, String>;

    /// Load the message entries of the current session.
    async fn get_messages(&self) -> Result<Vec<SessionEntry>, String>;

    /// Load session statistics.
    async fn get_session_stats(&self) -> Result<SessionStats, String>;

    /// List available slash commands.
    fn get_commands(&self) -> Vec<CommandInfo>;

    /// Enqueue a steering message for the active (or next) run.
    fn steer(&mut self, message: &str) -> Result<(), String>;

    /// Enqueue a follow-up message delivered when the run would otherwise stop.
    fn follow_up(&mut self, message: &str) -> Result<(), String>;

    /// Clear one or both pending-message queues.
    fn clear_queue(&mut self, clear_steer: bool, clear_follow_up: bool) -> Result<(), String>;

    /// `(steer_count, follow_up_count)`.
    fn queue_stats(&self) -> (usize, usize);
}

// ── In-process driver ─────────────────────────────────────────────

/// In-process driver wrapping the local agent module.
///
/// Constructed at the composition root (`app::cli` via `bootstrap`) which wires
/// ports and agent together. This is the **only** place in the app surfaces
/// that imports `agent`.
pub struct InProcessDriver {
    agent: ReActAgent,
    /// Session store, held so SwitchSession/GetMessages/Fork can operate. The
    /// agent holds its own clone internally; this one is the surface's handle
    /// for session-management commands.
    store: Arc<dyn XySessionStore>,
}

impl InProcessDriver {
    /// Construct from a built agent plus the store used to build it.
    ///
    /// `store` is the same instance injected into the agent at construction;
    /// holding it here lets session commands operate without reaching into
    /// agent internals.
    pub fn new(agent: ReActAgent, store: Arc<dyn XySessionStore>) -> Self {
        Self { agent, store }
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.agent.cancel_token()
    }

    /// Mutable agent handle for composition-root seams (e.g. MCP reload).
    ///
    /// Surfaces should prefer [`Driver`] methods; this exists so `app::core`
    /// helpers that may import infra can update tools without reach-in from
    /// `app/tui`.
    pub fn agent_mut(&mut self) -> &mut ReActAgent {
        &mut self.agent
    }
}

#[async_trait]
impl Driver for InProcessDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        let stream = self.agent.run(prompt).await;
        Box::pin(stream)
    }

    fn abort(&self) {
        self.agent.abort();
    }

    fn current_model(&self) -> Option<ModelInfo> {
        self.agent.inner().current_model().map(ModelInfo::from)
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        self.agent
            .inner()
            .model_registry()
            .list()
            .iter()
            .map(ModelInfo::from)
            .collect()
    }

    fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, String> {
        // Match by exact id or by config.model alias.
        let registry = self.agent.inner().model_registry();
        let found = registry
            .list()
            .iter()
            .find(|m| m.config.model == model_id || m.id == model_id)
            .map(|m| m.id.clone())
            .ok_or_else(|| format!("model not found: {model_id}"))?;
        self.agent.inner_mut().select_model(&found)?;
        // Re-read the resolved model to return authoritative info.
        Ok(self
            .agent
            .inner()
            .current_model()
            .map(ModelInfo::from)
            .unwrap_or_else(|| ModelInfo {
                id: found.clone(),
                display_name: found,
                thinking: true,
                context_window: 0,
            }))
    }

    fn cycle_model(&mut self) -> Result<ModelInfo, String> {
        let list = self.agent.inner().model_registry().list().to_vec();
        if list.is_empty() {
            return Err("no models available".into());
        }
        let current_id = self.agent.inner().current_model().map(|m| m.id.clone());
        let current_idx = current_id
            .as_ref()
            .and_then(|cur| list.iter().position(|m| m.id == *cur))
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % list.len();
        let next_id = list[next_idx].id.clone();
        self.agent.inner_mut().select_model(&next_id)?;
        Ok(ModelInfo::from(&list[next_idx]))
    }

    fn set_thinking_level(&mut self, level: ThinkingLevel) {
        self.agent.inner_mut().set_thinking_level(level);
    }

    fn thinking_level(&self) -> ThinkingLevel {
        self.agent.inner().thinking_level()
    }

    fn session_id(&self) -> Option<String> {
        self.agent.inner().session_id().map(String::from)
    }

    async fn execute_bash(
        &mut self,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<XyBashResult, String> {
        self.agent
            .inner_mut()
            .execute_bash(command, exclude_from_context)
            .await
    }

    async fn compact(&mut self) -> Result<bool, String> {
        self.agent.inner_mut().maybe_auto_compact().await
    }

    async fn export_html(&mut self, path: &Path) -> Result<String, String> {
        self.agent.inner_mut().export_to_html(path).await?;
        Ok(path.to_string_lossy().into_owned())
    }

    async fn export_jsonl(&mut self, path: &Path) -> Result<String, String> {
        self.agent.inner_mut().export_to_jsonl(path).await?;
        Ok(path.to_string_lossy().into_owned())
    }

    async fn import_jsonl(&mut self, path: &Path) -> Result<String, String> {
        self.agent.inner_mut().import_from_jsonl(path).await
    }

    async fn fork_session(&mut self, entry_id: &str) -> Result<String, String> {
        self.agent.inner_mut().fork_session(entry_id).await
    }

    async fn switch_session(&mut self, session_id: &str) -> Result<String, String> {
        if !self.store.exists(session_id).await {
            return Err(format!("session not found: {session_id}"));
        }
        self.agent.inner_mut().set_session(session_id.to_string());
        Ok(session_id.to_string())
    }

    async fn get_messages(&self) -> Result<Vec<SessionEntry>, String> {
        let sid = self.agent.inner().session_id().ok_or("no active session")?;
        self.store.load_entries(sid).await
    }

    async fn get_session_stats(&self) -> Result<SessionStats, String> {
        self.agent.inner().get_session_stats().await
    }

    fn get_commands(&self) -> Vec<CommandInfo> {
        self.agent
            .inner()
            .get_commands()
            .into_iter()
            .map(|c| CommandInfo {
                name: c.name,
                description: c.description,
            })
            .collect()
    }

    fn steer(&mut self, message: &str) -> Result<(), String> {
        self.agent.steer(message);
        Ok(())
    }

    fn follow_up(&mut self, message: &str) -> Result<(), String> {
        self.agent.follow_up(message);
        Ok(())
    }

    fn clear_queue(&mut self, clear_steer: bool, clear_follow_up: bool) -> Result<(), String> {
        self.agent.clear_queues(clear_steer, clear_follow_up);
        Ok(())
    }

    fn queue_stats(&self) -> (usize, usize) {
        self.agent.queue_stats()
    }
}

// ── Remote driver ─────────────────────────────────────────────────

/// Remote driver — speaks protocol over REST/WS to a xylitol server.
///
/// Uses `reqwest` for control commands (prompt, abort, model, export, ...) and
/// `tokio-tungstenite` for WebSocket event streaming.
#[cfg(feature = "server")]
pub struct RemoteDriver {
    base_url: String,
    session_id: String,
    client: reqwest::Client,
    cancel: CancellationToken,
    /// Cached thinking level (server does not expose a getter; tracked locally
    /// so get_state returns something sensible). NOTE: ceiling: server gains a
    /// state endpoint. upgrade: when server exposes GET /state.
    thinking: std::sync::Mutex<ThinkingLevel>,
}

#[cfg(feature = "server")]
impl RemoteDriver {
    /// Create a new RemoteDriver connected to `base_url`.
    ///
    /// `base_url` should be the server root, e.g. `http://127.0.0.1:8080`.
    pub fn new(base_url: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            session_id: session_id.into(),
            client: reqwest::Client::new(),
            cancel: CancellationToken::new(),
            thinking: std::sync::Mutex::new(ThinkingLevel::Medium),
        }
    }

    fn run_url(&self) -> String {
        format!("{}/api/v1/session/{}/run", self.base_url, self.session_id)
    }

    fn cancel_url(&self) -> String {
        format!("{}/api/v1/session/{}", self.base_url, self.session_id)
    }

    fn ws_url(&self) -> String {
        let ws_base = self
            .base_url
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        format!("{ws_base}/api/v1/session/{}/ws", self.session_id)
    }
}

#[cfg(feature = "server")]
#[async_trait]
impl Driver for RemoteDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        let client = self.client.clone();
        let run_url = self.run_url();
        let cancel_url = self.cancel_url();
        let ws_url = self.ws_url();
        let cancel = self.cancel.clone();
        let prompt = prompt.to_string();

        let stream = async_stream::stream! {
            // 1. Submit prompt via REST (triggers agent execution)
            let payload = serde_json::json!({"prompt": prompt});
            match client.post(&run_url).json(&payload).send().await {
                Ok(resp) if !resp.status().is_success() => {
                    let status = resp.status();
                    yield XyEvent::Error(format!("server returned {status}"));
                    return;
                }
                Err(e) => {
                    yield XyEvent::Error(format!("connection failed: {e}"));
                    return;
                }
                _ => {} // success
            }

            // 2. Connect to WS for event streaming
            let ws_stream = match connect_async(&ws_url).await {
                Ok((ws, _)) => ws,
                Err(e) => {
                    yield XyEvent::Error(format!("WS connect failed: {e}"));
                    return;
                }
            };

            let (mut ws_writer, mut ws_reader) = ws_stream.split();

            // 3. Send Subscribe
            let subscribe = serde_json::to_string(&ClientFrame::Subscribe {
                session_id: String::new(), // server knows from URL
                last_seq: 0,
            })
            .unwrap();
            if ws_writer.send(Message::Text(subscribe.into())).await.is_err() {
                yield XyEvent::Error("WS send failed".into());
                return;
            }

            // 4. Read events until AgentEnd or abort
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        let _ = client.delete(&cancel_url).send().await;
                        break;
                    }
                    msg = ws_reader.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(frame) = serde_json::from_str::<ServerFrame>(&text) {
                                    match frame {
                                        ServerFrame::ServerHello { .. } | ServerFrame::Ack { .. } => {
                                            // Handshake frames, ignore
                                        }
                                        ServerFrame::Event { event, .. } => {
                                            if let Ok(agent_event) = XyEvent::try_from(&event) {
                                                let is_end = matches!(agent_event, XyEvent::AgentEnd { .. });
                                                yield agent_event;
                                                if is_end {
                                                    break;
                                                }
                                            }
                                        }
                                        ServerFrame::ResyncRequired { .. } => {
                                            yield XyEvent::Error("journal truncated, resync required".into());
                                            break;
                                        }
                                    }
                                }
                            }
                            Some(Ok(Message::Close(_))) | None => break,
                            _ => {}
                        }
                    }
                }
            }
        };

        Box::pin(stream)
    }

    fn abort(&self) {
        self.cancel.cancel();
        let url = self.cancel_url();
        let client = self.client.clone();
        tokio::spawn(async move {
            let _ = client.delete(&url).send().await;
        });
    }

    // NOTE: the command methods below hit REST endpoints. Several server routes
    // for these commands do not exist yet (the server today exposes only
    // run/cancel/events). Until the server grows the matching routes, these
    // return a not-implemented error rather than fabricating a response. The
    // in-process path is the authoritative implementation; remote parity
    // arrives when the server command surface is built out.
    // ceiling: server lacks command routes. upgrade: add REST routes + wire.

    fn current_model(&self) -> Option<ModelInfo> {
        None
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        Vec::new()
    }

    fn select_model(&mut self, _model_id: &str) -> Result<ModelInfo, String> {
        Err("RemoteDriver command routes not yet implemented".into())
    }

    fn cycle_model(&mut self) -> Result<ModelInfo, String> {
        Err("RemoteDriver command routes not yet implemented".into())
    }

    fn set_thinking_level(&mut self, level: ThinkingLevel) {
        *self.thinking.lock().unwrap() = level;
    }

    fn thinking_level(&self) -> ThinkingLevel {
        *self.thinking.lock().unwrap()
    }

    fn session_id(&self) -> Option<String> {
        Some(self.session_id.clone())
    }

    async fn execute_bash(
        &mut self,
        _command: &str,
        _exclude_from_context: bool,
    ) -> Result<XyBashResult, String> {
        Err("RemoteDriver command routes not yet implemented".into())
    }

    async fn compact(&mut self) -> Result<bool, String> {
        Err("RemoteDriver command routes not yet implemented".into())
    }

    async fn export_html(&mut self, _path: &Path) -> Result<String, String> {
        Err("RemoteDriver command routes not yet implemented".into())
    }

    async fn export_jsonl(&mut self, _path: &Path) -> Result<String, String> {
        Err("RemoteDriver command routes not yet implemented".into())
    }

    async fn import_jsonl(&mut self, _path: &Path) -> Result<String, String> {
        Err("RemoteDriver command routes not yet implemented".into())
    }

    async fn fork_session(&mut self, _entry_id: &str) -> Result<String, String> {
        Err("RemoteDriver command routes not yet implemented".into())
    }

    async fn switch_session(&mut self, _session_id: &str) -> Result<String, String> {
        Err("RemoteDriver command routes not yet implemented".into())
    }

    async fn get_messages(&self) -> Result<Vec<SessionEntry>, String> {
        Err("RemoteDriver command routes not yet implemented".into())
    }

    async fn get_session_stats(&self) -> Result<SessionStats, String> {
        Err("RemoteDriver command routes not yet implemented".into())
    }

    fn get_commands(&self) -> Vec<CommandInfo> {
        Vec::new()
    }

    fn steer(&mut self, _message: &str) -> Result<(), String> {
        Err("RemoteDriver command routes not yet implemented".into())
    }

    fn follow_up(&mut self, _message: &str) -> Result<(), String> {
        Err("RemoteDriver command routes not yet implemented".into())
    }

    fn clear_queue(&mut self, _clear_steer: bool, _clear_follow_up: bool) -> Result<(), String> {
        Err("RemoteDriver command routes not yet implemented".into())
    }

    fn queue_stats(&self) -> (usize, usize) {
        (0, 0)
    }
}
