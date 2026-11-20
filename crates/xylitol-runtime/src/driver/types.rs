use std::pin::Pin;

use futures::Stream;

use xylitol_plugin_sdk::message::{AgentPart, XyLlmMessage};
use xylitol_plugin_sdk::session::{SessionEntry, SessionTreeKind};

pub use xylitol_plugin_sdk::driver::{
    CommandInfo, LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot, ModelInfo,
    ReloadStepReport, RuntimeReloadReport, SessionState,
};

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

pub const MCP_PENDING_CUE: &str = "mcp pending (see /mcp)";

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

#[non_exhaustive]
#[derive(Debug, Clone, Default)]
pub struct TurnRequest {
    pub parts: Vec<AgentPart>,
    pub context: Vec<XyLlmMessage>,
    pub system_prompt: Option<String>,
}

impl TurnRequest {
    pub fn text(prompt: impl Into<String>) -> Self {
        Self::new(vec![AgentPart::text(prompt)])
    }

    pub fn new(parts: Vec<AgentPart>) -> Self {
        Self {
            parts,
            context: Vec::new(),
            system_prompt: None,
        }
    }

    pub fn with_context(mut self, context: Vec<XyLlmMessage>) -> Self {
        self.context = context;
        self
    }

    pub fn with_system_prompt(mut self, system_prompt: Option<String>) -> Self {
        self.system_prompt = system_prompt;
        self
    }

    pub fn prompt_text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|part| match part {
                AgentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_request_round_trips_prompt() {
        let request = TurnRequest::text("hello");
        assert_eq!(request.prompt_text(), "hello");
        assert!(request.context.is_empty());
        assert!(request.system_prompt.is_none());
    }

    #[test]
    fn prompt_text_joins_text_parts_only() {
        let request = TurnRequest::new(vec![
            AgentPart::text("first"),
            AgentPart::thinking("hidden"),
            AgentPart::text("second"),
        ]);
        assert_eq!(request.prompt_text(), "first\nsecond");
    }
}

pub type EventStream = Pin<Box<dyn Stream<Item = XyEvent> + Send>>;

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
