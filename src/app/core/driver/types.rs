//! Driver seam DTOs, estimates, and stream alias.

use std::pin::Pin;

use futures::Stream;

use crate::protocol::session::{SessionEntry, SessionTreeKind};
use crate::protocol::types::{ThinkingLevel, XyModelMeta};

/// One step in a [`RuntimeReloadReport`] (c1120).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReloadStepReport {
    pub step: &'static str,
    pub ok: bool,
    pub message: String,
}

/// Aggregated runtime reload outcome for `/reload` (c1120).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeReloadReport {
    pub steps: Vec<ReloadStepReport>,
}

/// `/trust` subcommand modes (c1105).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTrustMode {
    /// Persist trust for the current project cwd.
    TrustCwd,
    /// Persist trust for the parent folder (when available).
    TrustParent,
    /// Persist deny for the current project cwd.
    Deny,
}

/// Outcome of [`XyDriver::persist_project_trust`] (c1105).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTrustPersistReport {
    pub trusted: bool,
    pub saved_path: Option<String>,
    pub message: String,
}

impl ProjectTrustPersistReport {
    /// Shared post-write hint — resources apply only after explicit reload/restart.
    pub const RELOAD_HINT: &'static str =
        "Project resources apply after /reload or restart (not auto-reloaded).";
}

/// Outcome of [`XyDriver::copy_text_to_clipboard`] (c1110).
///
/// Native tools run off the UI thread; OSC 52 (when needed) is returned here so
/// the product TUI can write it via `Terminal` on the host thread — never from
/// a blocking worker racing CSI 2026 frames.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClipboardCopyOutcome {
    /// Preformatted OSC 52 sequence for the host to emit (may be `None`).
    pub pending_osc52: Option<String>,
}

impl RuntimeReloadReport {
    /// Scripted / remote drivers: no-op success with a single diagnostic step.
    pub fn noop() -> Self {
        Self {
            steps: vec![ReloadStepReport {
                step: "runtime",
                ok: true,
                message: "noop (reload not supported on this driver)".into(),
            }],
        }
    }

    pub fn format_lines(&self) -> Vec<String> {
        self.steps
            .iter()
            .map(|s| {
                let status = if s.ok { "ok" } else { "failed" };
                format!("{}: {status} — {}", s.step, s.message)
            })
            .collect()
    }
}

/// Read-only skills + MCP summary for the product TUI loaded-resources slot (c1135).
///
/// MUST NOT carry secrets or env values — only server ids, tool counts, and short
/// failure lines safe for dim header display.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoadedResourcesSnapshot {
    pub skill_names: Vec<String>,
    /// `(server_id, tool_count)` for successfully connected MCP servers.
    pub mcp_connected: Vec<(String, usize)>,
    pub mcp_configured: usize,
    /// Short failure lines (`server: message`); no secrets.
    pub mcp_diag_short: Vec<String>,
    /// When MCP bootstrap is in flight: `connecting 1/3 · foo` (c1200).
    pub mcp_connecting_label: Option<String>,
}

impl LoadedResourcesSnapshot {
    /// Whether skills or MCP sections would render (brand line is separate).
    pub fn has_resource_rows(&self) -> bool {
        !self.skill_names.is_empty()
            || self.mcp_configured > 0
            || !self.mcp_connected.is_empty()
            || !self.mcp_diag_short.is_empty()
            || self.mcp_connecting_label.is_some()
    }
}

/// Re-export so XyDriver implementors under surfaces can name the return type
/// without importing `crate::agent::session` directly (layering: surfaces use
/// the XyDriver seam). Surfaces reference this as
/// `crate::app::core::driver::SessionStats`.
pub use crate::agent::session::{QueueStats, SessionStats};

/// Session resume list row (from [`XySessionStore::list_sessions`]).
pub use crate::protocol::ports::SessionListEntry;

/// Build a [`ContextTokenEstimate`] from persisted session entries (XyDriver seam).
///
/// `tokenizer_override` comes from `AppConfig` (`models.*.tokenizer` / `tokenizers:`)
/// when the in-process XyDriver estimates; harness / remote may pass `None`.
pub fn estimate_from_session_entries(
    entries: &[SessionEntry],
    model_id: Option<String>,
    tokenizer_override: Option<xylitol_ai_bridge::registry::TokenizerOverride>,
) -> crate::protocol::types::ContextTokenEstimate {
    use crate::agent::compaction::{EstimateOpts, estimate_from_session_entries as estimate};
    estimate(
        entries,
        &EstimateOpts {
            model_id,
            tokenizer_override,
            allow_local_tokenizer: allow_local_tokenizer_from_app_config(),
            ..Default::default()
        },
    )
}

/// Resolve `models.<alias>.tokenizer` from the layered AppConfig (best-effort).
pub(crate) fn tokenizer_override_from_app_config(
    model_alias: &str,
) -> Option<xylitol_ai_bridge::registry::TokenizerOverride> {
    crate::infra::config::loader::load_app_config(None)
        .ok()?
        .tokenizer_override_for(model_alias)
}

pub(crate) fn allow_local_tokenizer_from_app_config() -> bool {
    crate::infra::config::loader::load_app_config(None)
        .map(|c| c.token_estimate.local_tokenizer.is_on())
        .unwrap_or(false)
}

/// Lifecycle events on [`EventStream`] — surfaces import via the XyDriver seam
/// (not `crate::agent`), keeping app/tui off agent internals.
pub use crate::protocol::lifecycle::XyEvent;

/// A stream of [`XyEvent`] items.
pub type EventStream = Pin<Box<dyn Stream<Item = XyEvent> + Send>>;

/// Minimal info about a slash command (for `GetCommands`), decoupled from the
/// agent's internal `SlashCommandInfo` so the XyDriver trait does not leak
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

/// Minimal model info returned by the XyDriver, decoupled from `XyModelMeta`'s
/// many fields so callers only see what command dispatch needs.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub thinking: bool,
    /// Xylitol thinking levels supported by this model (`off`-only when not adjustable).
    pub thinking_levels: Vec<String>,
    pub context_window: u64,
}

impl From<&XyModelMeta> for ModelInfo {
    fn from(m: &XyModelMeta) -> Self {
        let levels = ThinkingLevel::resolve_configured_levels(
            m.thinking,
            (!m.thinking_levels.is_empty()).then_some(m.thinking_levels.as_slice()),
        )
        .unwrap_or_else(|_| vec![ThinkingLevel::Off]);
        Self {
            id: m.id.clone(),
            display_name: m.display_name.clone(),
            thinking: m.thinking && ThinkingLevel::is_adjustable(&levels),
            thinking_levels: levels.iter().map(|l| l.as_str().to_string()).collect(),
            context_window: m.context_window,
        }
    }
}

/// Outcome of [`XyDriver::load_debug_scene`] (c710).
#[derive(Debug, Clone)]
pub struct DebugSceneLoad {
    pub session_id: String,
    pub entries: Vec<SessionEntry>,
    pub note: String,
    /// Set when the driver also switched to a catalog `fake` model.
    pub model: Option<ModelInfo>,
}

pub(crate) fn session_tree_kind_unimplemented(kind: SessionTreeKind) -> String {
    let name = match kind {
        SessionTreeKind::MessageHistory => "message_history",
        SessionTreeKind::FileBrowser => "file_browser",
    };
    format!("session tree kind '{name}' is not implemented")
}
