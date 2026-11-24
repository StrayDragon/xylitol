use std::pin::Pin;

use futures::Stream;

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
pub const CAPABILITY_PENDING_CUE: &str = "Next turn: capability overlay";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum McpReadyState {
    NotConfigured,
    Connecting,
    BootstrapPending,
    Frozen,
    Arming,
    Ready,
}

#[allow(dead_code)]
pub trait McpReadyStateView {
    fn mcp_ready_state(&self) -> McpReadyState;
}

impl McpReadyStateView for LoadedResourcesSnapshot {
    fn mcp_ready_state(&self) -> McpReadyState {
        if self.mcp_configured == 0 {
            return McpReadyState::NotConfigured;
        }
        if self.mcp_bootstrap_complete && self.tools_table_frozen {
            return McpReadyState::Frozen;
        }
        if self.mcp_connecting_label.is_some()
            || self
                .mcp_servers
                .iter()
                .any(|s| s.phase == McpServerPhase::Connecting)
        {
            return McpReadyState::Connecting;
        }
        if self.tools_table_frozen {
            return McpReadyState::Frozen;
        }
        if !self.mcp_bootstrap_complete {
            return McpReadyState::BootstrapPending;
        }
        if self
            .mcp_servers
            .iter()
            .any(|s| s.phase == McpServerPhase::Connected && !s.tools_armed)
        {
            return McpReadyState::Arming;
        }
        McpReadyState::Ready
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QueueStats {
    pub steer_count: usize,
    pub follow_up_count: usize,
}

#[derive(Debug, Clone)]
pub struct SessionStats {
    pub session_id: String,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub total_messages: usize,
    pub thinking_level: String,
    pub model: Option<(String, String)>,
}

pub use xylitol_plugin_sdk::SessionListEntry;

pub fn estimate_from_session_entries(
    entries: &[SessionEntry],
    model_id: Option<String>,
    _fixed_context: Option<xylitol_plugin_sdk::FixedRequestContext>,
) -> xylitol_plugin_sdk::model::ContextTokenEstimate {
    xylitol_plugin_sdk::estimate_from_session_entries(
        entries,
        &xylitol_plugin_sdk::EstimateOpts {
            model_id,
            ..Default::default()
        },
    )
}

pub use xylitol_plugin_sdk::lifecycle::XyEvent;

pub type EventStream = Pin<Box<dyn Stream<Item = XyEvent> + Send>>;

#[derive(Debug, Clone)]
pub struct DebugSceneLoad {
    pub session_id: String,
    pub entries: Vec<SessionEntry>,
    pub note: String,
    pub model: Option<ModelInfo>,
}

#[allow(dead_code)]
pub(crate) fn session_tree_kind_unimplemented(kind: SessionTreeKind) -> String {
    let name = match kind {
        SessionTreeKind::MessageHistory => "message_history",
        SessionTreeKind::FileBrowser => "file_browser",
    };
    format!("session tree kind '{name}' is not implemented")
}

#[cfg(test)]
mod mcp_ready_state_tests {
    use super::*;

    #[derive(Clone)]
    struct Case {
        configured: usize,
        bootstrap: bool,
        frozen: bool,
        label: Option<String>,
        any_connecting: bool,
        any_connected_unarmed: bool,
    }

    impl Case {
        fn snapshot(&self) -> LoadedResourcesSnapshot {
            let mut connecting = Vec::new();
            if self.any_connecting {
                connecting.push(McpServerSnapshot {
                    id: "s-connecting".into(),
                    phase: McpServerPhase::Connecting,
                    tools_armed: false,
                    tool_count: 1,
                });
            }
            if self.any_connected_unarmed {
                connecting.push(McpServerSnapshot {
                    id: "s-connected".into(),
                    phase: McpServerPhase::Connected,
                    tools_armed: false,
                    tool_count: 1,
                });
            }
            LoadedResourcesSnapshot {
                mcp_configured: self.configured,
                mcp_bootstrap_complete: self.bootstrap,
                tools_table_frozen: self.frozen,
                mcp_connecting_label: self.label.clone(),
                mcp_servers: connecting,
                ..Default::default()
            }
        }

        fn legacy_pending(&self) -> bool {
            if self.configured == 0 {
                return false;
            }
            if self.bootstrap && self.frozen {
                return false;
            }
            if self.label.is_some() {
                return true;
            }
            if self.any_connecting {
                return true;
            }
            if self.frozen {
                return false;
            }
            if !self.bootstrap {
                return true;
            }
            self.any_connected_unarmed
        }
    }

    #[test]
    fn ready_state_matches_the_legacy_pending_truth_table() {
        for configured in [0usize, 2] {
            for bootstrap in [false, true] {
                for frozen in [false, true] {
                    for label in [None, Some("connecting".to_string())] {
                        for any_connecting in [false, true] {
                            for any_connected_unarmed in [false, true] {
                                let case = Case {
                                    configured,
                                    bootstrap,
                                    frozen,
                                    label: label.clone(),
                                    any_connecting,
                                    any_connected_unarmed,
                                };
                                let snap = case.snapshot();
                                assert_eq!(
                                    snap.mcp_tools_pending(),
                                    case.legacy_pending(),
                                    "configured={configured} bootstrap={bootstrap} frozen={frozen} label={label:?} connecting={any_connecting} unarmed={any_connected_unarmed}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn ready_state_decomposition_covers_all_states() {
        let base = Case {
            configured: 2,
            bootstrap: true,
            frozen: false,
            label: None,
            any_connecting: false,
            any_connected_unarmed: false,
        };
        assert_eq!(base.snapshot().mcp_ready_state(), McpReadyState::Ready);
        let not_configured = Case {
            configured: 0,
            ..base.clone()
        };
        assert_eq!(
            not_configured.snapshot().mcp_ready_state(),
            McpReadyState::NotConfigured
        );
        let connecting = Case {
            any_connecting: true,
            ..base.clone()
        };
        assert_eq!(
            connecting.snapshot().mcp_ready_state(),
            McpReadyState::Connecting
        );
        let bootstrap_pending = Case {
            bootstrap: false,
            ..base.clone()
        };
        assert_eq!(
            bootstrap_pending.snapshot().mcp_ready_state(),
            McpReadyState::BootstrapPending
        );
        let frozen = Case {
            bootstrap: false,
            frozen: true,
            ..base.clone()
        };
        assert_eq!(frozen.snapshot().mcp_ready_state(), McpReadyState::Frozen);
        let arming = Case {
            any_connected_unarmed: true,
            ..base.clone()
        };
        assert_eq!(arming.snapshot().mcp_ready_state(), McpReadyState::Arming);
    }
}
