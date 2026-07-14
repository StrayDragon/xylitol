//! Driver abstraction — the single dependency of interactive layers.
//!
//! All interactive clients (cli/print/tui) interact with the core through a
//! [`Driver`]; they import agent symbols only from `agent` (mod-level),
//! never reaching into `agent::session`/`agent::runtime` internals or `infra`.
//!
//! - [`crate::app::core::driver::InProcessDriver`]: wraps the local agent module (composition root wires
//!   ports and agent together).
//! - Remote HTTP driver: speaks the protocol over REST/WS to a remote server.
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

use crate::agent::AgentRuntime;
use crate::domain::session_types::{
    SessionEntry, SessionTreeKind, SessionTreeNode, SessionTreeTravel, plan_message_history_travel,
};
use crate::domain::types::{ThinkingLevel, XyModelMeta};
use crate::runtime_protocol::{XyBashResult, XySessionStore};

/// Re-export so Driver implementors under surfaces can name the return type
/// without importing `crate::agent::session` directly (which arch_guard
/// forbids for tui/). Surfaces reference this as
/// `crate::app::core::driver::SessionStats`.
pub use crate::agent::session::{QueueStats, SessionStats};

/// Lifecycle events on [`EventStream`] — surfaces import via the Driver seam
/// (not `crate::agent`), so arch_guard stays green for `app/tui`.
pub use crate::domain::lifecycle::XyEvent;

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
/// [`crate::app::core::driver::InProcessDriver`] keeps a cached `AgentRuntime` and is the local
/// (single-process) implementation. The remote HTTP driver speaks the protocol over
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
    ///
    /// Takes `&self` so the host can `select!` keyboard (Esc → [`Self::abort`])
    /// while bash is in flight (c665). `chunk_tx` uplinks live output bytes for
    /// product TUI streaming (c669); pass `None` for non-streaming callers.
    async fn execute_bash(
        &self,
        command: &str,
        exclude_from_context: bool,
        chunk_tx: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,
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
    async fn fork_session(
        &mut self,
        entry_id: &str,
        position: crate::domain::session_types::ForkPosition,
    ) -> Result<String, String>;

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

    /// Queue depths for steer / follow-up.
    fn queue_stats(&self) -> QueueStats;

    /// Read a session tree for the given kind.
    ///
    /// Driver-only seam (not wired through `protocol::Command`); REST calls this
    /// directly for MessageHistory tree endpoints.
    async fn session_tree(&self, kind: SessionTreeKind) -> Result<Vec<SessionTreeNode>, String>;

    /// Travel within a session tree kind and update the active leaf.
    ///
    /// Driver-only seam (not wired through `protocol::Command`); REST travel
    /// endpoints call this directly.
    async fn travel_session_tree(
        &self,
        kind: SessionTreeKind,
        entry_id: &str,
    ) -> Result<SessionTreeTravel, String>;

    /// Persist a tree annotation (`Label` entry) for `target_id` (c690).
    /// `label: None` or empty clears the annotation.
    async fn append_entry_label(
        &mut self,
        target_id: &str,
        label: Option<&str>,
    ) -> Result<(), String>;

    /// Active MessageHistory leaf entry id for the current session (c700 `/fork`).
    fn leaf_entry_id(&self) -> Option<String>;

    /// Load a named `/debug <scene>` fixture into a fresh `debug-*` session (c710).
    ///
    /// Returns session id + entries for transcript rebuild. Does not invent a
    /// `protocol::Command` — Driver-only like session_tree. Fixtures live in
    /// `app::debug_fixtures` (delete that module to remove).
    async fn load_debug_scene(&mut self, scene: &str) -> Result<DebugSceneLoad, String>;
}

/// Outcome of [`Driver::load_debug_scene`] (c710).
#[derive(Debug, Clone)]
pub struct DebugSceneLoad {
    pub session_id: String,
    pub entries: Vec<SessionEntry>,
    pub note: String,
    /// Set when the driver also switched to a catalog `fake` model.
    pub model: Option<ModelInfo>,
}

// ── In-process driver ─────────────────────────────────────────────

/// In-process driver wrapping the local agent module.
///
/// Constructed at the composition root (`app::cli` via `bootstrap`) which wires
/// ports and agent together. This is the **only** place in the app surfaces
/// that imports `agent`.
pub struct InProcessDriver {
    agent: AgentRuntime,
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
    pub fn new(agent: AgentRuntime, store: Arc<dyn XySessionStore>) -> Self {
        Self { agent, store }
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.agent.cancel_token()
    }

    /// Replace the tool set (next `run`). Used by composition MCP reload.
    pub fn set_tools(&mut self, tools: crate::agent::tools::ToolSet) {
        self.agent.set_tools(tools);
    }

    /// Test/diagnostics: tool names currently registered.
    #[cfg(test)]
    pub(crate) fn tool_names_for_test(&self) -> Vec<String> {
        self.agent
            .inner()
            .tools()
            .iter()
            .map(|t| t.name().to_string())
            .collect()
    }
}

#[async_trait]
impl Driver for InProcessDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        let sid = self
            .agent
            .inner()
            .session_id()
            .map(String::from)
            .unwrap_or_else(|| {
                let id = uuid::Uuid::new_v4().to_string();
                self.agent.inner_mut().set_session(id.clone());
                id
            });
        let stream = self.agent.run_with_id(prompt, &sid).await;
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
        self.agent
            .inner_mut()
            .select_model_with_source(&next_id, "cycle")?;
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
        &self,
        command: &str,
        exclude_from_context: bool,
        chunk_tx: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,
    ) -> Result<XyBashResult, String> {
        self.agent
            .inner()
            .execute_bash(command, exclude_from_context, chunk_tx)
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

    async fn fork_session(
        &mut self,
        entry_id: &str,
        position: crate::domain::session_types::ForkPosition,
    ) -> Result<String, String> {
        self.agent
            .inner_mut()
            .fork_session(entry_id, position)
            .await
    }

    async fn switch_session(&mut self, session_id: &str) -> Result<String, String> {
        if !self.store.exists(session_id).await {
            return Err(format!("session not found: {session_id}"));
        }
        if let Some(bus) = self.agent.inner().hook_bus() {
            crate::agent::session::cancel_hook(
                &bus,
                "session_before_switch",
                "pre",
                serde_json::json!({ "reason": "resume", "target": session_id }),
            )
            .await?;
            crate::agent::session::observe_hook(
                &bus,
                "session_shutdown",
                "",
                serde_json::json!({
                    "reason": "resume",
                    "target": session_id,
                    "previous": self.agent.inner().session_id(),
                }),
            )
            .await;
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

    fn queue_stats(&self) -> crate::agent::session::QueueStats {
        self.agent.queue_stats()
    }

    async fn session_tree(&self, kind: SessionTreeKind) -> Result<Vec<SessionTreeNode>, String> {
        let sid = self.agent.inner().session_id().ok_or("no active session")?;
        if let Some(bus) = self.agent.inner().hook_bus() {
            crate::agent::session::cancel_hook(
                &bus,
                "session_before_tree",
                "pre",
                serde_json::json!({ "kind": format!("{kind:?}") }),
            )
            .await?;
        }
        // Bootstrap may assign a fresh id before any persist; wiped HOME may leave
        // an orphan id. Ensure an empty session so double-Esc opens an empty tree.
        self.agent.inner().ensure_session(sid, None).await?;
        let tree = match kind {
            SessionTreeKind::MessageHistory => self.store.message_history_tree(sid).await?,
            SessionTreeKind::FileBrowser => return Err(session_tree_kind_unimplemented(kind)),
        };
        if let Some(bus) = self.agent.inner().hook_bus() {
            crate::agent::session::observe_hook(
                &bus,
                "session_tree",
                "post",
                serde_json::json!({ "kind": format!("{kind:?}") }),
            )
            .await;
        }
        Ok(tree)
    }

    async fn travel_session_tree(
        &self,
        kind: SessionTreeKind,
        entry_id: &str,
    ) -> Result<SessionTreeTravel, String> {
        let sid = self.agent.inner().session_id().ok_or("no active session")?;
        if let Some(bus) = self.agent.inner().hook_bus() {
            crate::agent::session::cancel_hook(
                &bus,
                "session_before_tree",
                "pre",
                serde_json::json!({
                    "kind": format!("{kind:?}"),
                    "entry_id": entry_id,
                }),
            )
            .await?;
        }
        let travel = match kind {
            SessionTreeKind::MessageHistory => {
                let entries = self.store.load_entries(sid).await?;
                let travel = plan_message_history_travel(&entries, entry_id)?;
                self.store.set_leaf(sid, travel.leaf_id.as_deref());
                travel
            }
            SessionTreeKind::FileBrowser => {
                return Err(session_tree_kind_unimplemented(kind));
            }
        };
        if let Some(bus) = self.agent.inner().hook_bus() {
            crate::agent::session::observe_hook(
                &bus,
                "session_tree",
                "post",
                serde_json::json!({
                    "kind": format!("{kind:?}"),
                    "entry_id": entry_id,
                    "leaf_id": travel.leaf_id,
                }),
            )
            .await;
        }
        Ok(travel)
    }

    async fn append_entry_label(
        &mut self,
        target_id: &str,
        label: Option<&str>,
    ) -> Result<(), String> {
        use crate::domain::session_types::{EntryBase, LabelEntry};

        let sid = self.agent.inner().session_id().ok_or("no active session")?;
        self.agent.inner().ensure_session(sid, None).await?;
        let entries = self.store.load_entries(sid).await?;
        if !entries.iter().any(|e| e.entry_id() == Some(target_id)) {
            return Err(format!("target entry not found: {target_id}"));
        }
        let cleaned = label
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let entry = SessionEntry::Label(LabelEntry {
            base: EntryBase {
                entry_type: "label".into(),
                id: String::new(),
                parent_id: None,
                timestamp: String::new(),
            },
            target_id: target_id.to_string(),
            label: cleaned,
        });
        self.store.append_session_entry(sid, &entry).await
    }

    fn leaf_entry_id(&self) -> Option<String> {
        let sid = self.agent.inner().session_id()?;
        self.store.leaf_id(sid)
    }

    async fn load_debug_scene(&mut self, scene: &str) -> Result<DebugSceneLoad, String> {
        use crate::app::debug_fixtures::{list_note, resolve_scene_id, seed_scene};

        let scene = scene.trim();
        if scene.is_empty() || scene.eq_ignore_ascii_case("list") {
            return Err(list_note());
        }
        let canonical = resolve_scene_id(scene)
            .ok_or_else(|| format!("unknown debug scene: {scene}\n{}", list_note()))?;
        let short = &uuid::Uuid::new_v4().to_string()[..8];
        let session_id = format!("debug-{canonical}-{short}");
        let cwd = std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().into_owned());
        self.store.create(&session_id, cwd.as_deref(), None).await?;
        let canonical = seed_scene(self.store.as_ref(), &session_id, scene).await?;
        self.agent.inner_mut().set_session(session_id.clone());
        let entries = self.store.load_entries(&session_id).await?;
        let mut note = format!("debug scene `{canonical}` → session {session_id}");
        let model = match self.select_model("fake") {
            Ok(m) => {
                note.push_str("; model → fake");
                Some(m)
            }
            Err(_) => {
                note.push_str("; fake not in catalog (tree fixture only)");
                None
            }
        };
        Ok(DebugSceneLoad {
            session_id,
            entries,
            note,
            model,
        })
    }
}

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

    fn api(&self, suffix: &str) -> String {
        format!(
            "{}/api/v1/session/{}/{}",
            self.base_url, self.session_id, suffix
        )
    }

    fn block_on<T>(
        &self,
        fut: impl std::future::Future<Output = Result<T, String>>,
    ) -> Result<T, String> {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
            Err(_) => tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?
                .block_on(fut),
        }
    }

    async fn get_data(&self, suffix: &str) -> Result<serde_json::Value, String> {
        let resp = self
            .client
            .get(self.api(suffix))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Self::parse_envelope(resp).await
    }

    async fn post_data(
        &self,
        suffix: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .client
            .post(self.api(suffix))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Self::parse_envelope(resp).await
    }

    async fn parse_envelope(resp: reqwest::Response) -> Result<serde_json::Value, String> {
        let status = resp.status();
        let env: crate::protocol::Envelope<serde_json::Value> =
            resp.json().await.map_err(|e| e.to_string())?;
        if env.code != crate::protocol::ErrorCode::Ok {
            return Err(env
                .msg
                .unwrap_or_else(|| format!("server error ({status})")));
        }
        Ok(env.data.unwrap_or(serde_json::Value::Null))
    }

    fn model_from_value(v: &serde_json::Value) -> Result<ModelInfo, String> {
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
            thinking: v.get("thinking").and_then(|x| x.as_bool()).unwrap_or(false),
            context_window: v
                .get("context_window")
                .and_then(|x| x.as_u64())
                .unwrap_or(0),
        })
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

    fn current_model(&self) -> Option<ModelInfo> {
        self.block_on(async {
            let data = self.get_data("state").await?;
            match data.get("model") {
                Some(m) if !m.is_null() => Self::model_from_value(m).map(Some),
                _ => Ok(None),
            }
        })
        .ok()
        .flatten()
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        self.block_on(async {
            let data = self.get_data("models").await?;
            let arr = data
                .get("models")
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();
            arr.iter().map(Self::model_from_value).collect()
        })
        .unwrap_or_default()
    }

    fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, String> {
        let model_id = model_id.to_string();
        let url = format!(
            "{}/api/v1/session/{}/model?model_id={}",
            self.base_url,
            self.session_id,
            urlencoding_loose(&model_id)
        );
        self.block_on(async {
            let resp = self
                .client
                .post(&url)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let data = Self::parse_envelope(resp).await?;
            // Endpoint returns { model, display_name }; enrich via list if needed.
            if data.get("id").is_some() {
                Self::model_from_value(&data)
            } else {
                Ok(ModelInfo {
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
                    context_window: 0,
                })
            }
        })
    }

    fn cycle_model(&mut self) -> Result<ModelInfo, String> {
        self.block_on(async {
            let data = self.post_data("model/cycle", serde_json::json!({})).await?;
            Self::model_from_value(&data)
        })
    }

    fn set_thinking_level(&mut self, level: ThinkingLevel) {
        *self.thinking.lock().unwrap() = level;
        let level_str = level.as_str();
        let _ = self.block_on(async {
            self.post_data("thinking", serde_json::json!({ "level": level_str }))
                .await
        });
    }

    fn thinking_level(&self) -> ThinkingLevel {
        *self.thinking.lock().unwrap()
    }

    fn session_id(&self) -> Option<String> {
        Some(self.session_id.clone())
    }

    async fn execute_bash(
        &self,
        command: &str,
        exclude_from_context: bool,
        _chunk_tx: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,
    ) -> Result<XyBashResult, String> {
        // Remote REST bash is request/response — no live chunk uplink.
        let data = self
            .post_data(
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
            truncated: data
                .get("truncated")
                .and_then(|c| c.as_bool())
                .unwrap_or(false),
            full_output_path: None,
        })
    }

    async fn compact(&mut self) -> Result<bool, String> {
        let data = self.post_data("compact", serde_json::json!({})).await?;
        Ok(data
            .get("compacted")
            .and_then(|c| c.as_bool())
            .unwrap_or(false))
    }

    async fn export_html(&mut self, path: &Path) -> Result<String, String> {
        let data = self
            .post_data(
                "export/html",
                serde_json::json!({ "path": path.to_string_lossy() }),
            )
            .await?;
        Ok(data
            .get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string())
    }

    async fn export_jsonl(&mut self, path: &Path) -> Result<String, String> {
        let data = self
            .post_data(
                "export/jsonl",
                serde_json::json!({ "path": path.to_string_lossy() }),
            )
            .await?;
        Ok(data
            .get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string())
    }

    async fn import_jsonl(&mut self, path: &Path) -> Result<String, String> {
        let data = self
            .post_data(
                "import/jsonl",
                serde_json::json!({ "path": path.to_string_lossy() }),
            )
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
        position: crate::domain::session_types::ForkPosition,
    ) -> Result<String, String> {
        let data = self
            .post_data(
                "fork",
                serde_json::json!({
                    "entry_id": entry_id,
                    "position": match position {
                        crate::domain::session_types::ForkPosition::At => "at",
                        crate::domain::session_types::ForkPosition::Before => "before",
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

    async fn switch_session(&mut self, session_id: &str) -> Result<String, String> {
        let data = self
            .post_data("switch", serde_json::json!({ "session_id": session_id }))
            .await?;
        let id = data
            .get("session_id")
            .and_then(|p| p.as_str())
            .unwrap_or(session_id)
            .to_string();
        self.session_id = id.clone();
        Ok(id)
    }

    async fn get_messages(&self) -> Result<Vec<SessionEntry>, String> {
        let data = self.get_data("messages").await?;
        let entries = data
            .get("entries")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        serde_json::from_value(entries).map_err(|e| e.to_string())
    }

    async fn get_session_stats(&self) -> Result<SessionStats, String> {
        let data = self.get_data("stats").await?;
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

    fn get_commands(&self) -> Vec<CommandInfo> {
        self.block_on(async {
            let data = self.get_data("commands").await?;
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
        .unwrap_or_default()
    }

    fn steer(&mut self, message: &str) -> Result<(), String> {
        self.block_on(async {
            self.post_data("steer", serde_json::json!({ "message": message }))
                .await?;
            Ok(())
        })
    }

    fn follow_up(&mut self, message: &str) -> Result<(), String> {
        self.block_on(async {
            self.post_data("follow-up", serde_json::json!({ "message": message }))
                .await?;
            Ok(())
        })
    }

    fn clear_queue(&mut self, clear_steer: bool, clear_follow_up: bool) -> Result<(), String> {
        self.block_on(async {
            self.post_data(
                "queue/clear",
                serde_json::json!({
                    "clear_steer": clear_steer,
                    "clear_follow_up": clear_follow_up,
                }),
            )
            .await?;
            Ok(())
        })
    }

    fn queue_stats(&self) -> crate::agent::session::QueueStats {
        self.block_on(async {
            let data = self.get_data("queue").await?;
            Ok(crate::agent::session::QueueStats {
                steer_count: data
                    .get("steer_count")
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as usize,
                follow_up_count: data
                    .get("follow_up_count")
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as usize,
            })
        })
        .unwrap_or_default()
    }

    async fn session_tree(&self, kind: SessionTreeKind) -> Result<Vec<SessionTreeNode>, String> {
        match kind {
            SessionTreeKind::MessageHistory => {
                let data = self.get_data("trees/message-history").await?;
                let tree = data.get("tree").cloned().unwrap_or(serde_json::Value::Null);
                serde_json::from_value(tree).map_err(|e| e.to_string())
            }
            SessionTreeKind::FileBrowser => Err(session_tree_kind_unimplemented(kind)),
        }
    }

    async fn travel_session_tree(
        &self,
        kind: SessionTreeKind,
        entry_id: &str,
    ) -> Result<SessionTreeTravel, String> {
        match kind {
            SessionTreeKind::MessageHistory => {
                let data = self
                    .post_data(
                        "trees/message-history/travel",
                        serde_json::json!({ "entry_id": entry_id }),
                    )
                    .await?;
                serde_json::from_value(data).map_err(|e| e.to_string())
            }
            SessionTreeKind::FileBrowser => Err(session_tree_kind_unimplemented(kind)),
        }
    }

    async fn append_entry_label(
        &mut self,
        _target_id: &str,
        _label: Option<&str>,
    ) -> Result<(), String> {
        Err("remote: append_entry_label not implemented".into())
    }

    fn leaf_entry_id(&self) -> Option<String> {
        None
    }

    async fn load_debug_scene(&mut self, _scene: &str) -> Result<DebugSceneLoad, String> {
        Err("remote: load_debug_scene not implemented".into())
    }
}

fn session_tree_kind_unimplemented(kind: SessionTreeKind) -> String {
    let name = match kind {
        SessionTreeKind::MessageHistory => "message_history",
        SessionTreeKind::FileBrowser => "file_browser",
    };
    format!("session tree kind '{name}' is not implemented")
}

fn urlencoding_loose(s: &str) -> String {
    s.replace(' ', "%20")
}

#[cfg(test)]
mod driver_session_tree_tests {
    use std::sync::Arc;

    use super::*;
    use crate::agent::AgentBuilder;
    use crate::agent::tools::ToolSet;
    use crate::domain::model::XyModelConfig;
    use crate::domain::session_types::{EntryBase, MessageEntry, SessionEntry, SessionTreeKind};
    use crate::infra::bash_exec::InfraBashExecutor;
    use crate::infra::config::value::InfraSecretResolver;
    use crate::infra::event::EventBus;
    use crate::infra::export::StdExportIo;
    use crate::infra::permission;
    use crate::infra::session::SessionManager;
    use crate::runtime_protocol::{
        XyBashExecutor, XyEventSink, XyExportIo, XyModel, XySessionStore,
    };

    type ModelBuilderFn =
        Arc<dyn Fn(&XyModelConfig) -> Result<Arc<dyn XyModel>, String> + Send + Sync>;

    fn msg_entry(id: &str, parent: Option<&str>, role: &str, text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: parent.map(str::to_string),
                timestamp: format!("2026-01-01T00:00:{id}Z"),
            },
            message: crate::domain::session_types::fixture_message_json(role, text),
        })
    }

    async fn build_test_driver(store: Arc<SessionManager>) -> InProcessDriver {
        let store_trait: Arc<dyn XySessionStore> = store.clone();
        let mut agent = AgentBuilder::new(
            crate::agent::model::registry::ModelRegistry::new(Arc::new(InfraSecretResolver::new())),
            Arc::new(crate::infra::provider::factory::build_provider),
            store_trait.clone(),
            Arc::new(EventBus::new()) as Arc<dyn XyEventSink>,
            permission::allow_all_permission(),
        )
        .cwd(".")
        .tools(ToolSet::from_iter(crate::infra::tools::default_tools()))
        .bash(Arc::new(InfraBashExecutor::new()) as Arc<dyn XyBashExecutor>)
        .export_io(Arc::new(StdExportIo::new()) as Arc<dyn XyExportIo>)
        .build()
        .expect("build agent");
        let sid = uuid::Uuid::new_v4().to_string();
        store_trait
            .create(&sid, Some("."), None)
            .await
            .expect("create session");
        agent.inner_mut().set_session(sid);
        InProcessDriver::new(agent, store)
    }

    #[tokio::test]
    async fn in_process_session_tree_ensures_missing_session() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let store_trait: Arc<dyn XySessionStore> = store.clone();
        let mut agent = AgentBuilder::new(
            crate::agent::model::registry::ModelRegistry::new(Arc::new(InfraSecretResolver::new())),
            Arc::new(crate::infra::provider::factory::build_provider),
            store_trait.clone(),
            Arc::new(EventBus::new()) as Arc<dyn XyEventSink>,
            permission::allow_all_permission(),
        )
        .cwd(".")
        .tools(ToolSet::from_iter(crate::infra::tools::default_tools()))
        .bash(Arc::new(InfraBashExecutor::new()) as Arc<dyn XyBashExecutor>)
        .export_io(Arc::new(StdExportIo::new()) as Arc<dyn XyExportIo>)
        .build()
        .expect("build agent");
        // Orphan id: set on agent but never created on disk (wipe / pre-persist).
        let orphan = uuid::Uuid::new_v4().to_string();
        agent.inner_mut().set_session(orphan.clone());
        let driver = InProcessDriver::new(agent, store);
        let tree = driver
            .session_tree(SessionTreeKind::MessageHistory)
            .await
            .expect("empty tree after ensure");
        assert!(tree.is_empty(), "expected empty tree, got {tree:?}");
        assert!(
            store_trait.exists(&orphan).await,
            "ensure_session must create orphan session"
        );
    }

    #[tokio::test]
    async fn in_process_session_tree_returns_parent_child() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let driver = build_test_driver(store.clone()).await;
        let sid = driver.session_id().expect("session");

        store
            .append_with_id(&sid, &msg_entry("u1", None, "user", "hello"))
            .await
            .expect("append u1");
        store
            .append_with_id(&sid, &msg_entry("a1", Some("u1"), "assistant", "hi"))
            .await
            .expect("append a1");

        let tree = driver
            .session_tree(SessionTreeKind::MessageHistory)
            .await
            .expect("tree");
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].entry.entry_id(), Some("u1"));
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].entry.entry_id(), Some("a1"));
    }

    #[tokio::test]
    async fn in_process_travel_user_sets_parent_leaf_and_editor_text() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let driver = build_test_driver(store.clone()).await;
        let sid = driver.session_id().expect("session");

        store
            .append_with_id(&sid, &msg_entry("u1", None, "user", "edit me"))
            .await
            .unwrap();
        store
            .append_with_id(&sid, &msg_entry("a1", Some("u1"), "assistant", "reply"))
            .await
            .unwrap();
        XySessionStore::set_leaf(store.as_ref(), &sid, Some("a1"));

        let travel = driver
            .travel_session_tree(SessionTreeKind::MessageHistory, "u1")
            .await
            .expect("travel");
        assert_eq!(travel.leaf_id, None);
        assert_eq!(travel.editor_text.as_deref(), Some("edit me"));
        assert_eq!(XySessionStore::leaf_id(store.as_ref(), &sid), None);
    }

    #[tokio::test]
    async fn in_process_travel_non_user_sets_leaf_without_editor_text() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let driver = build_test_driver(store.clone()).await;
        let sid = driver.session_id().expect("session");

        store
            .append_with_id(&sid, &msg_entry("u1", None, "user", "hello"))
            .await
            .unwrap();
        store
            .append_with_id(&sid, &msg_entry("a1", Some("u1"), "assistant", "reply"))
            .await
            .unwrap();

        let travel = driver
            .travel_session_tree(SessionTreeKind::MessageHistory, "a1")
            .await
            .expect("travel");
        assert_eq!(travel.leaf_id.as_deref(), Some("a1"));
        assert!(travel.editor_text.is_none());
        assert_eq!(
            XySessionStore::leaf_id(store.as_ref(), &sid).as_deref(),
            Some("a1")
        );
    }

    #[tokio::test]
    async fn unsupported_tree_kind_returns_err_without_changing_leaf() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let driver = build_test_driver(store.clone()).await;
        let sid = driver.session_id().expect("session");
        XySessionStore::set_leaf(store.as_ref(), &sid, Some("keep"));

        let err = driver
            .session_tree(SessionTreeKind::FileBrowser)
            .await
            .expect_err("file_browser");
        assert!(err.contains("file_browser"));
        assert_eq!(
            XySessionStore::leaf_id(store.as_ref(), &sid).as_deref(),
            Some("keep")
        );

        let err = driver
            .travel_session_tree(SessionTreeKind::FileBrowser, "x")
            .await
            .expect_err("travel file_browser");
        assert!(err.contains("file_browser"));
        assert_eq!(
            XySessionStore::leaf_id(store.as_ref(), &sid).as_deref(),
            Some("keep")
        );
    }

    #[tokio::test]
    async fn after_run_session_tree_reflects_persisted_turn() {
        use std::pin::Pin;

        use async_trait::async_trait;
        use futures::StreamExt;

        use crate::domain::error::XyError;
        use crate::domain::message::AgentMessage;
        use crate::domain::message::XyStopReason;
        use crate::domain::model::XyModelConfig;
        use crate::domain::types::{XyChunk, XyModelMeta, XyToolSchema};
        use crate::runtime_protocol::{XyModel, XyStream};

        struct TextMockModel;
        #[async_trait]
        impl XyModel for TextMockModel {
            fn name(&self) -> &str {
                "text-mock"
            }

            async fn generate_stream(
                &self,
                _messages: Vec<AgentMessage>,
                _tools: &[XyToolSchema],
                _stream: bool,
            ) -> Result<XyStream, XyError> {
                let chunks = vec![
                    Ok(XyChunk::TextDelta("reply".into())),
                    Ok(XyChunk::Done {
                        finish_reason: XyStopReason::Stop,
                        usage: None,
                    }),
                ];
                Ok(Box::pin(futures::stream::iter(chunks))
                    as Pin<
                        Box<dyn futures::Stream<Item = Result<XyChunk, XyError>> + Send>,
                    >)
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let store_trait: Arc<dyn XySessionStore> = store.clone();
        let mut reg =
            crate::agent::model::registry::ModelRegistry::new(Arc::new(InfraSecretResolver::new()));
        reg.register(XyModelMeta {
            id: "mock".into(),
            config: XyModelConfig {
                kind: crate::domain::model::XyModelKind::Fake,
                api_key: String::new(),
                model: "mock".into(),
                base_url: None,
                api: None,
            },
            display_name: "Mock".into(),
            thinking: false,
            context_window: 128000,
            api: String::new(),
            provider: String::new(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels: Vec::new(),
        });
        let builder: ModelBuilderFn = Arc::new(|_| Ok(Arc::new(TextMockModel) as Arc<dyn XyModel>));
        let mut agent = AgentBuilder::new(
            reg,
            builder,
            store_trait.clone(),
            Arc::new(EventBus::new()) as Arc<dyn crate::runtime_protocol::XyEventSink>,
            permission::allow_all_permission(),
        )
        .cwd(".")
        .tools(ToolSet::empty())
        .bash(Arc::new(InfraBashExecutor::new()) as Arc<dyn XyBashExecutor>)
        .export_io(Arc::new(StdExportIo::new()) as Arc<dyn XyExportIo>)
        .build()
        .expect("build agent");
        let sid = uuid::Uuid::new_v4().to_string();
        agent.inner_mut().set_session(sid);
        let mut driver = InProcessDriver::new(agent, store_trait);

        let mut stream = driver.run("hello tree").await;
        while stream.next().await.is_some() {}

        let tree = driver
            .session_tree(SessionTreeKind::MessageHistory)
            .await
            .expect("tree");
        assert!(
            !tree.is_empty(),
            "session tree should reflect persisted messages"
        );
    }
}
