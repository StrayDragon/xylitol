use std::pin::Pin;

use futures::Stream;
use serde::{Deserialize, Serialize};

use xylitol_plugin_sdk::model::{THINKING_OFF, XyModelMeta, thinking_levels_are_adjustable};
use xylitol_plugin_sdk::session::{SessionEntry, SessionTreeKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReloadStepReport {
    pub step: &'static str,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct RuntimeReloadReport {
    pub steps: Vec<ReloadStepReport>,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTrustMode {
    TrustCwd,
    TrustParent,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTrustPersistReport {
    pub trusted: bool,
    pub saved_path: Option<String>,
    pub message: String,
}

impl ProjectTrustPersistReport {
    pub const RELOAD_HINT: &'static str =
        "Project resources apply after /reload or restart (not auto-reloaded).";
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClipboardCopyOutcome {
    pub pending_osc52: Option<String>,
}

impl RuntimeReloadReport {
    pub fn noop() -> Self {
        Self {
            steps: vec![ReloadStepReport {
                step: "runtime",
                ok: true,
                message: "noop (reload not supported on this driver)".into(),
            }],
            cancelled: false,
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

    pub fn any_step_failed(&self) -> bool {
        self.steps.iter().any(|s| !s.ok)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LoadedResourcesSnapshot {
    pub skill_names: Vec<String>,
    pub mcp_connected: Vec<(String, usize)>,
    pub mcp_configured: usize,
    pub mcp_diag_short: Vec<String>,
    pub mcp_connecting_label: Option<String>,
    pub mcp_servers: Vec<McpServerSnapshot>,
    pub mcp_bootstrap_complete: bool,
    pub tools_table_frozen: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_gate_notice: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obs_diag: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpServerPhase {
    Connecting,
    Connected,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerSnapshot {
    pub id: String,
    pub phase: McpServerPhase,
    pub tools_armed: bool,
    pub tool_count: usize,
}

pub const MCP_PENDING_CUE: &str = "mcp pending (see /mcp)";

impl LoadedResourcesSnapshot {
    pub fn mcp_tools_pending(&self) -> bool {
        if self.mcp_configured == 0 {
            return false;
        }
        if self.mcp_connecting_label.is_some() {
            return true;
        }
        if self
            .mcp_servers
            .iter()
            .any(|s| s.phase == McpServerPhase::Connecting)
        {
            return true;
        }
        if self.tools_table_frozen {
            return false;
        }
        if !self.mcp_bootstrap_complete {
            return true;
        }
        self.mcp_servers
            .iter()
            .any(|s| s.phase == McpServerPhase::Connected && !s.tools_armed)
    }
}

pub use crate::agent::{QueueStats, SessionStats};

pub use xylitol_plugin_sdk::SessionListEntry;

pub fn estimate_from_session_entries(
    entries: &[SessionEntry],
    model_id: Option<String>,
    fixed_context: Option<crate::agent::compaction::FixedRequestContext>,
) -> xylitol_plugin_sdk::model::ContextTokenEstimate {
    use crate::agent::compaction::{EstimateOpts, estimate_from_session_entries as estimate};
    estimate(
        entries,
        &EstimateOpts {
            model_id,
            fixed_context,
            ..Default::default()
        },
    )
}

pub use xylitol_plugin_sdk::lifecycle::XyEvent;

pub type EventStream = Pin<Box<dyn Stream<Item = XyEvent> + Send>>;

#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub session_id: String,
    pub model: Option<ModelInfo>,
    pub thinking_level: String,
    pub leaf_entry_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub thinking: bool,
    pub thinking_levels: Vec<String>,
    pub context_window: u64,
}

impl From<&XyModelMeta> for ModelInfo {
    fn from(m: &XyModelMeta) -> Self {
        let thinking_levels = if !m.thinking || m.thinking_levels.is_empty() {
            vec![THINKING_OFF.into()]
        } else {
            m.thinking_levels.clone()
        };
        Self {
            id: m.id.clone(),
            display_name: m.display_name.clone(),
            thinking: thinking_levels_are_adjustable(&thinking_levels),
            thinking_levels,
            context_window: m.context_window,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DebugSceneLoad {
    pub session_id: String,
    pub entries: Vec<SessionEntry>,
    pub note: String,
    pub model: Option<ModelInfo>,
}

pub(crate) fn session_tree_kind_unimplemented(kind: SessionTreeKind) -> String {
    let name = match kind {
        SessionTreeKind::MessageHistory => "message_history",
        SessionTreeKind::FileBrowser => "file_browser",
    };
    format!("session tree kind '{name}' is not implemented")
}
