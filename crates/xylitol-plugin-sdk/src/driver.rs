use serde::{Deserialize, Serialize};

use crate::model::{THINKING_OFF, XyModelMeta, thinking_levels_are_adjustable};

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
    #[serde(default)]
    pub mcp_details: Vec<String>,
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

impl LoadedResourcesSnapshot {
    pub fn mcp_tools_pending(&self) -> bool {
        if self.mcp_configured == 0 {
            return false;
        }
        if self.mcp_bootstrap_complete && self.tools_table_frozen {
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueueSnapshot {
    pub items: Vec<crate::session::QueueItem>,
}

impl QueueSnapshot {
    pub fn steer_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.kind == crate::session::QueueItemKind::Steer)
            .count()
    }

    pub fn follow_up_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.kind == crate::session::QueueItemKind::FollowUp)
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevokeQueueItemOutcome {
    pub revoked: bool,
    pub items: Vec<crate::session::QueueItem>,
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
