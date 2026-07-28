//! [`XyDriver`] — shared application driver protocol.

use std::path::Path;

use async_trait::async_trait;

use crate::protocol::ports::XyBashResult;
use crate::protocol::session::{SessionEntry, SessionTreeKind, SessionTreeNode, SessionTreeTravel};
use crate::protocol::types::ThinkingLevel;

use super::XyDriverError;
use super::types::{
    ClipboardCopyOutcome, CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot,
    ModelInfo, ProjectTrustMode, ProjectTrustPersistReport, QueueStats, RuntimeReloadReport,
    SessionListEntry, SessionState, SessionStats,
};

/// XyDriver — interact with the core without knowing its internals.
///
/// [`crate::app::core::driver::XyInProcessDriver`] keeps a cached `AgentRuntime` and is the local
/// (single-process) implementation. The remote HTTP driver speaks the protocol over
/// WS/REST to a xylitol server.
#[async_trait]
pub trait XyDriver: Send {
    /// Submit a prompt and receive a stream of events.
    async fn run(&mut self, prompt: &str) -> EventStream;

    /// Cancel the current turn.
    fn abort(&self);

    /// Currently selected model, if any.
    fn current_model(&self) -> Option<ModelInfo>;

    /// In-flight turn binding `(display_label, thinking, omit_thinking)` while an agent run is active.
    /// `None` when idle / converged (footer uses selected).
    fn active_turn(&self) -> Option<(String, ThinkingLevel, bool)> {
        None
    }

    /// True while ReAct has an active turn binding (agent run in flight).
    fn has_active_turn(&self) -> bool {
        false
    }

    /// All registered models.
    fn available_models(&self) -> Vec<ModelInfo>;

    /// Select a model by id (exact match on `id` or `config.model`).
    /// Returns the selected model on success.
    fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, XyDriverError>;

    /// Cycle to the next model in the registry. Returns the newly-selected model.
    fn cycle_model(&mut self) -> Result<ModelInfo, XyDriverError>;

    /// Set the thinking level.
    ///
    /// Returns `Err` if the level is not in the current model's support set.
    fn set_thinking_level(&mut self, level: ThinkingLevel) -> Result<(), XyDriverError>;

    /// Current thinking level.
    fn thinking_level(&self) -> ThinkingLevel;

    /// Cycle to the next level in the current model's thinking support list.
    ///
    /// Returns the level now in effect. Demo / legacy callers only; product TUI
    /// changes thinking solely via `/model` (ati36).
    fn cycle_thinking_level(&mut self) -> Result<ThinkingLevel, XyDriverError>;

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
    ) -> Result<XyBashResult, XyDriverError>;

    /// Force compact (manual). Optional `instructions` focus the summary (c1670).
    /// Returns whether a compaction occurred.
    async fn compact(&mut self, instructions: Option<String>) -> Result<bool, XyDriverError>;

    /// Export the session to HTML at `path`. Returns the path used.
    async fn export_html(&mut self, path: &Path) -> Result<String, XyDriverError>;

    /// Export the session to JSONL at `path`. Returns the path used.
    async fn export_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError>;

    /// Import a JSONL file. Returns the new session id.
    async fn import_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError>;

    /// Fork the current session at `entry_id`. Returns the new session id.
    async fn fork_session(
        &mut self,
        entry_id: &str,
        position: crate::protocol::session::ForkPosition,
    ) -> Result<String, XyDriverError>;

    /// Switch to an existing session id. Validates existence first.
    async fn switch_session(&mut self, session_id: &str) -> Result<String, XyDriverError>;

    /// Load the message entries of the current session.
    async fn get_messages(&self) -> Result<Vec<SessionEntry>, XyDriverError>;

    /// Load session statistics.
    async fn get_session_stats(&self) -> Result<SessionStats, XyDriverError>;

    /// Read-only context token estimate for the current leaf/path (c1030 / c1035).
    ///
    /// Product footer polls this after travel / turn / compact.
    async fn estimate_context_tokens(
        &self,
    ) -> Result<crate::protocol::types::ContextTokenEstimate, XyDriverError>;

    /// List available slash commands.
    fn get_commands(&self) -> Vec<CommandInfo>;

    /// Enqueue a steering message for the active (or next) run.
    fn steer(&mut self, message: &str) -> Result<(), XyDriverError>;

    /// Enqueue a follow-up message delivered when the run would otherwise stop.
    fn follow_up(&mut self, message: &str) -> Result<(), XyDriverError>;

    /// Clear one or both pending-message queues.
    fn clear_queue(
        &mut self,
        clear_steer: bool,
        clear_follow_up: bool,
    ) -> Result<(), XyDriverError>;

    /// Queue depths for steer / follow-up.
    fn queue_stats(&self) -> QueueStats;

    /// Read a session tree for the given kind.
    ///
    /// XyDriver-only seam (not wired through `protocol::Command`); REST calls this
    /// directly for MessageHistory tree endpoints.
    async fn session_tree(
        &self,
        kind: SessionTreeKind,
    ) -> Result<Vec<SessionTreeNode>, XyDriverError>;

    /// Travel within a session tree kind and update the active leaf.
    ///
    /// XyDriver-only seam (not wired through `protocol::Command`); REST travel
    /// endpoints call this directly.
    async fn travel_session_tree(
        &self,
        kind: SessionTreeKind,
        entry_id: &str,
    ) -> Result<SessionTreeTravel, XyDriverError>;

    /// Persist a tree annotation (`Label` entry) for `target_id` (c690).
    /// `label: None` or empty clears the annotation.
    async fn append_entry_label(
        &mut self,
        target_id: &str,
        label: Option<&str>,
    ) -> Result<(), XyDriverError>;

    /// Active MessageHistory leaf entry id for the current session (c700/c1005 `/session-fork`).
    fn leaf_entry_id(&self) -> Option<String>;

    /// Load a named `/debug <scene>` fixture into a fresh `debug-*` session (c710).
    ///
    /// Returns session id + entries for transcript rebuild. Does not invent a
    /// `protocol::Command` — XyDriver-only like session_tree. Fixtures live in
    /// `app::debug_fixtures` (delete that module to remove).
    async fn load_debug_scene(&mut self, scene: &str) -> Result<DebugSceneLoad, XyDriverError>;

    /// List resumable sessions for `/session-resume` (c1015).
    ///
    /// XyDriver-only seam (not `protocol::Command`); sorted mtime desc by store.
    /// TUI MUST NOT read the sessions directory directly.
    async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XyDriverError>;

    /// Load raw session entries for any session id (c1560 editor history seed).
    ///
    /// TUI MUST NOT read the sessions directory directly.
    async fn load_session_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XyDriverError>;

    /// Create an empty session and make it current (`/session-new`, c1020).
    ///
    /// XyDriver-only seam (not `protocol::Command`).
    async fn new_session(&mut self) -> Result<String, XyDriverError>;

    /// Current session display name (`/session-name`, c1020).
    async fn get_session_name(&self) -> Result<Option<String>, XyDriverError>;

    /// Set current session display name; returns sanitized stored name (c1020).
    async fn set_session_name(&mut self, name: &str) -> Result<String, XyDriverError>;

    /// Set display name for any session (resume panel rename; c1065).
    async fn set_session_name_for(
        &mut self,
        session_id: &str,
        name: &str,
    ) -> Result<String, XyDriverError>;

    /// Delete a persisted session (resume panel; c1065). MUST NOT delete active session.
    async fn delete_session(&mut self, session_id: &str) -> Result<(), XyDriverError>;

    /// `(name, description)` for product `$skill` completion (c1130).
    ///
    /// Default empty (remote / scripted drivers). In-process uses Trust-filtered
    /// loaded skills.
    fn dollar_skill_catalog(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    /// Skills + MCP summary for the product TUI loaded-resources slot (c1135).
    ///
    /// In-process reads Trust-filtered skill names and MCP connected/diagnostics.
    /// Remote / stub drivers return empty (no default body — `dyn XyDriver` + Sync).
    async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot;

    /// Hot-reload skills, MCP, and prompt context (c1120).
    ///
    /// Keybindings and themes are orchestrated by the product TUI host. Default:
    /// no-op report for drivers without reload state.
    async fn reload_runtime(&mut self) -> Result<RuntimeReloadReport, XyDriverError> {
        Ok(RuntimeReloadReport::noop())
    }

    /// Persist a project trust decision for `/trust` (c1105).
    ///
    /// Does **not** reload skills/MCP/context — caller shows
    /// `ProjectTrustPersistReport::RELOAD_HINT`. Default: unsupported.
    fn persist_project_trust(
        &mut self,
        _mode: ProjectTrustMode,
    ) -> Result<ProjectTrustPersistReport, XyDriverError> {
        Err("persist_project_trust not supported on this driver".into())
    }

    /// Copy UTF-8 text to the system clipboard (`/history-copy-last`, c1110).
    ///
    /// Async so platform tools (`xclip` wait / `wl-copy` spawn) do not freeze the
    /// TUI host task. When OSC 52 is required, return it in
    /// `ClipboardCopyOutcome.pending_osc52` for the host to write via
    /// `Terminal` (do not emit from the driver). Default: unsupported.
    async fn copy_text_to_clipboard(
        &mut self,
        _text: &str,
    ) -> Result<ClipboardCopyOutcome, XyDriverError> {
        Err("copy_text_to_clipboard not supported on this driver".into())
    }

    /// Stage a clipboard image to a unique tempfile and return its absolute path (c1155).
    ///
    /// Returns `Ok(None)` when the clipboard has no image. Product TUI inserts the
    /// path as plain text (pi-aligned); MUST NOT put base64 in the editor.
    async fn stage_clipboard_image(&mut self) -> Result<Option<std::path::PathBuf>, XyDriverError> {
        Err("stage_clipboard_image not supported on this driver".into())
    }

    /// Read UTF-8 text from the system clipboard (c1156 / Ctrl+V text fallback).
    ///
    /// Returns `Ok(None)` when empty / no text. Default: unsupported.
    async fn read_clipboard_text(&mut self) -> Result<Option<String>, XyDriverError> {
        Err("read_clipboard_text not supported on this driver".into())
    }
}
