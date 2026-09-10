//! [`XyDriver`] — shared application driver protocol.

use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;

use super::XyDriverError;
use super::types::{
    ClipboardCopyOutcome, CommandInfo, EventStream, LoadedResourcesSnapshot, ModelInfo,
    ProjectTrustMode, ProjectTrustPersistReport, RuntimeReloadReport, SessionState, XyEvent,
};

/// Owned bash-completion receiver (c2790): produced by one `&mut` call to
/// [`XyDriver::bash_run`]; pollable in a select loop **without** borrowing the
/// driver, so interactive bang loops can drain Inline effects mid-bang. Output
/// chunks arrive independently via `set_bash_run_sink`; the wall-clock timeout
/// (host client transport) resolves inside the future.
pub type BashRun = Pin<
    Box<dyn Future<Output = Result<crate::protocol::ports::XyBashResult, XyDriverError>> + Send>,
>;

/// Downlink attachment health (ath42/c2480): drives the fixed-zone grace notice —
/// never transcript error rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkHealth {
    /// Subscribed and streaming.
    Up,
    /// Downlink not currently subscribed (connecting, retrying, or resyncing).
    Down,
}

/// XyDriver — interact with the core without knowing its internals.
///
/// [`crate::app::core::driver::XyInProcessDriver`] keeps a cached `AgentRuntime` and is the local
/// (single-process) implementation. The remote HTTP driver speaks the protocol over
/// WS/REST to a xylitol server.
///
/// # Errors
///
/// `Result` methods fail with [`XyDriverError`], classified by stable `kind`
/// (`NotFound` / `Io` / `Message` / `Unsupported` / …); `detail_kind` keeps the
/// source domain (`Session` / `Export` / `Trust` / …). Store `NotFound` flattens
/// to `kind=NotFound`; `NoActiveSession` flattens to `kind=Message`,
/// `detail_kind=Session`. Per-method docs below list the failing conditions.
#[async_trait]
pub trait XyDriver: crate::app::core::dispatch::SessionCommandExecutor + Send {
    /// Submit a prompt and receive a stream of events.
    async fn run(&mut self, prompt: &str) -> EventStream;

    /// Begin an interactive bash run (c2790): one `&mut` call, owned completion.
    ///
    /// The returned receiver is decoupled from the driver borrow — the caller's
    /// select loop may drain Inline effects (atm18) while the bash runs. Esc
    /// cancel = drop this receiver then call [`XyDriver::abort`] (out-of-band),
    /// exactly like the pre-inversion dispatch-fut drop. Pre-hooks (script
    /// hooks) run eagerly before the receiver is returned.
    async fn bash_run(
        &mut self,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<BashRun, XyDriverError> {
        let _ = (command, exclude_from_context);
        Err(XyDriverError::unsupported("bash_run"))
    }

    /// Product TUI attach: handshake + mux subscribe before the host loop.
    ///
    /// Default no-op (in-process / scripted). Remote MUST open mux and subscribe
    /// here so the first prompt is not the first WebSocket.
    async fn attach_session(&mut self) -> Result<(), XyDriverError> {
        Ok(())
    }

    /// Refresh cached fixed-zone state (model / models / commands) without blocking a tick.
    ///
    /// Default no-op. Remote MUST hit Host unaries here so `/model` and footer
    /// sync do not `block_on` HTTP from sync getters.
    async fn refresh_surface_caches(&mut self) -> Result<(), XyDriverError> {
        Ok(())
    }

    /// Events that arrived on a persistent downlink while no `run` stream is held.
    fn drain_idle_events(&mut self) -> Vec<XyEvent> {
        Vec::new()
    }

    /// Downlink attachment health for fixed-zone grace UX (ath42/c2480).
    ///
    /// Default [`LinkHealth::Up`]: in-process and scripted drivers are their
    /// own host, there is no link to lose. Remote overrides with the mux
    /// subscription state.
    fn link_health(&self) -> LinkHealth {
        LinkHealth::Up
    }

    /// Consume a `session/resync_required` rebuild flag (journal replay).
    fn take_resync_rebuild(&mut self) -> bool {
        false
    }

    /// Cancel the current turn.
    fn abort(&self);

    /// Currently selected model, if any.
    fn current_model(&self) -> Option<ModelInfo>;

    /// In-flight turn binding `(display_label, thinking, omit_thinking)` while an agent run is active.
    /// `None` when idle / converged (footer uses selected).
    fn active_turn(&self) -> Option<(String, String, bool)> {
        None
    }

    /// True while ReAct has an active turn binding (agent run in flight).
    fn has_active_turn(&self) -> bool {
        false
    }

    /// All registered models.
    fn available_models(&self) -> Vec<ModelInfo>;

    /// Current thinking level.
    fn thinking_level(&self) -> String;

    /// Cycle to the next level in the current model's thinking support list.
    ///
    /// Current session id (the id the next `run`/export acts on).
    fn session_id(&self) -> Option<String>;

    /// Snapshot of session state for `GetState`.
    fn get_state(&self) -> SessionState {
        SessionState {
            session_id: self.session_id().unwrap_or_default(),
            model: self.current_model(),
            thinking_level: self.thinking_level(),
            leaf_entry_id: self.leaf_entry_id(),
        }
    }

    /// Inject the interactive bang output-event sink (c2760, surface wiring).
    ///
    /// The caller (TUI) sets it before `Command::Bash` dispatch and clears it
    /// after. Default: no-op (scripted / drivers without live output).
    fn set_bash_run_sink(&mut self, _sink: Option<crate::protocol::ports::BashOutputSink>) {}

    /// Read-only context token estimate for the current leaf/path (c1030 / c1035).
    ///
    /// Prefer consuming [`crate::protocol::lifecycle::XyEvent::ContextTokenSettlement`]
    /// for turn-end / post-compact footer updates (c1860). Use this for leaf travel,
    /// mid-turn throttle, and stream-close **fallback** when no settlement was applied.
    ///
    /// # Errors
    ///
    /// `Err` when no model / tokenizer can estimate the current context.
    async fn estimate_context_tokens(
        &self,
    ) -> Result<crate::protocol::model::ContextTokenEstimate, XyDriverError>;

    /// List available slash commands.
    fn get_commands(&self) -> Vec<CommandInfo>;

    /// Active MessageHistory leaf entry id for the current session (c700/c1005 `/session-fork`).
    fn leaf_entry_id(&self) -> Option<String>;

    /// Load a named `/debug <scene>` fixture into a fresh `debug-*` session (c710).
    ///
    /// Returns session id + entries for transcript rebuild. Does not invent a
    /// `protocol::Command` — XyDriver-only like session_tree. Fixtures live in
    /// `app::debug_fixtures` (delete that module to remove).
    ///
    /// # Errors
    ///
    /// `Err` when the named scene is unknown.
    /// `(name, description)` for product `$skill` completion (c1130).
    ///
    /// Default empty (remote / scripted drivers). In-process uses Trust-filtered
    /// loaded skills.
    fn dollar_skill_catalog(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    /// Cloneable session store for spawn-safe listing (editor ↑/↓ history seed).
    ///
    /// In-process returns the shared store; remote / scripted drivers return `None`
    /// (caller falls back to awaiting a `Command::ListSessions` round-trip on
    /// the host task).
    fn session_store(&self) -> Option<std::sync::Arc<dyn crate::protocol::ports::XySessionStore>> {
        None
    }

    /// Skills + MCP summary for the product TUI loaded-resources slot (c1135).
    ///
    /// In-process reads Trust-filtered skill names and MCP connected/diagnostics.
    /// Remote / stub drivers return empty (no default body — `dyn XyDriver` + Sync).
    async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot;

    /// Last loaded-resources snapshot already held by the driver (**no I/O**).
    ///
    /// After [`Self::poll_mcp_bootstrap`] returns true, product hosts SHOULD apply
    /// this instead of awaiting [`Self::loaded_resources_snapshot`] again: Remote
    /// poll is a dirty flag on the mux cache. In-process returns `None`
    /// (snapshot is local and cheap).
    fn loaded_resources_cached(&self) -> Option<LoadedResourcesSnapshot> {
        None
    }

    /// True while MCP bootstrap is in flight and agent prompt must wait (c1200).
    fn mcp_blocks_agent(&self) -> bool {
        false
    }

    /// True when provider-visible tools are FROZEN (c1900). Default `true` =
    /// no first-turn gate (remote/stub). In-process returns the real freeze phase.
    fn is_tools_frozen(&self) -> bool {
        true
    }

    /// Arm tool freeze gate without blocking (TUI). Default: no-op.
    async fn arm_tool_freeze_gate(&mut self) {}

    /// One-shot notice after MCP gate timeout subset freeze. Default: none.
    fn take_mcp_gate_notice(&mut self) -> Option<String> {
        None
    }

    /// Start background MCP connect when configured (c1200). Idempotent.
    async fn begin_mcp_bootstrap(&mut self) {}

    /// Poll background MCP bootstrap; returns true when loaded-resources should refresh
    /// (connecting label changed, tools applied, or bootstrap phase advanced).
    ///
    /// In-process: observe local boot state — **MUST NOT** report dirty every tick.
    /// Remote: **MUST NOT** unary on the TUI tick. Host pushes `session/resources`
    /// when the writer poll is dirty; this method returns true when that cache
    /// update has not yet been applied to the TUI. Unchanged ticks MUST NOT
    /// refresh TUI catalogs (slash popup / skill list).
    ///
    /// **Contract (sticky cue)**: `Settling → Settled` (deferred system-prompt install
    /// finished) MUST return `true` even when no tool-freeze gate is armed. Hosts only
    /// call `refresh_loaded_resources` when this is true; skipping the signal leaves a
    /// stale snap with `mcp_bootstrap_complete=false` and sticky-restores
    /// `mcp pending (see /mcp)` after the welcome card already shows connected.
    async fn poll_mcp_bootstrap(&mut self) -> bool {
        false
    }

    /// Block until MCP bootstrap settles (print / tests). No-op when idle.
    async fn wait_mcp_bootstrap(&mut self) {
        while self.mcp_blocks_agent() {
            let _ = self.poll_mcp_bootstrap().await;
            tokio::task::yield_now().await;
        }
    }

    /// Hot-reload skills, MCP, and prompt context (c1120 / c1205).
    ///
    /// Keybindings and themes are orchestrated by the product TUI host. Default:
    /// no-op report for drivers without reload state.
    ///
    /// `cancel` is cooperative: implementations MUST put-back reload state on every
    /// exit and MUST NOT leave MCP/tools half-open when cancelled.
    ///
    /// # Errors
    ///
    /// `Err` on reload failure (MCP / tools / prompt assembly). Default: no-op `Ok`.
    async fn reload_runtime(
        &mut self,
        _cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<RuntimeReloadReport, XyDriverError> {
        Ok(RuntimeReloadReport::noop())
    }

    /// Persist a project trust decision for `/trust` (c1105).
    ///
    /// Does **not** reload skills/MCP/context — caller shows
    /// `ProjectTrustPersistReport::RELOAD_HINT`. Default: unsupported.
    ///
    /// # Errors
    ///
    /// `Err` on unsupported driver (default) or IO.
    async fn persist_project_trust(
        &mut self,
        _mode: ProjectTrustMode,
    ) -> Result<ProjectTrustPersistReport, XyDriverError> {
        Err(XyDriverError::unsupported(
            "persist_project_trust not supported on this driver",
        ))
    }

    /// Copy UTF-8 text to the system clipboard (`/history-copy-last`, c1110).
    ///
    /// Async so platform tools (`xclip` wait / `wl-copy` spawn) do not freeze the
    /// TUI host task. When OSC 52 is required, return it in
    /// `ClipboardCopyOutcome.pending_osc52` for the host to write via
    /// `Terminal` (do not emit from the driver). Default: unsupported.
    ///
    /// # Errors
    ///
    /// `Err` on unsupported driver (default) or platform clipboard failure.
    async fn copy_text_to_clipboard(
        &mut self,
        _text: &str,
    ) -> Result<ClipboardCopyOutcome, XyDriverError> {
        Err(XyDriverError::unsupported(
            "copy_text_to_clipboard not supported on this driver",
        ))
    }

    /// Stage a clipboard image to a unique tempfile and return its absolute path (c1155).
    ///
    /// Returns `Ok(None)` when the clipboard has no image. Product TUI inserts the
    /// path as plain text (pi-aligned); MUST NOT put base64 in the editor.
    ///
    /// # Errors
    ///
    /// `Err` on unsupported driver (default) or platform clipboard failure.
    async fn stage_clipboard_image(&mut self) -> Result<Option<std::path::PathBuf>, XyDriverError> {
        Err(XyDriverError::unsupported(
            "stage_clipboard_image not supported on this driver",
        ))
    }

    /// Read UTF-8 text from the system clipboard (c1156 / Ctrl+V text fallback).
    ///
    /// Returns `Ok(None)` when empty / no text. Default: unsupported.
    ///
    /// # Errors
    ///
    /// `Err` on unsupported driver (default) or platform clipboard failure.
    async fn read_clipboard_text(&mut self) -> Result<Option<String>, XyDriverError> {
        Err(XyDriverError::unsupported(
            "read_clipboard_text not supported on this driver",
        ))
    }
}
