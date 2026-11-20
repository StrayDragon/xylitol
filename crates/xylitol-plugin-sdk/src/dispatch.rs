use async_trait::async_trait;

use crate::driver::{
    CommandInfo, LoadedResourcesSnapshot, ModelInfo, RuntimeReloadReport, SessionState,
};
use crate::error::XyError;
use crate::session::{CapabilitySnapshot, SessionEntry, SessionTreeNode, SessionTreeTravel};
use crate::{SessionListEntry, XyBashResult};
use xylitol_protocol::Command;

#[derive(Debug)]
pub enum DispatchRejection {
    NotFound {
        message: String,
        source_kind: Option<&'static str>,
    },

    Unsupported {
        message: String,
        source_kind: Option<&'static str>,
    },

    InvalidInput {
        message: String,
        source_kind: Option<&'static str>,
    },

    Io {
        message: String,
        source_kind: Option<&'static str>,
    },

    Remote {
        message: String,
        source_kind: Option<&'static str>,
    },

    Agent(XyError),

    Message {
        message: String,
        source_kind: Option<&'static str>,
    },
}

impl DispatchRejection {
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: msg.into(),
            source_kind: None,
        }
    }
}

#[async_trait]
pub trait SessionCommandExecutor: Send {
    async fn execute_session_command(
        &mut self,
        cmd: Command,
    ) -> Result<DispatchOutcome, DispatchRejection>;
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum DispatchOutcome {
    Aborted {
        cancelled: bool,
    },
    State(SessionState),
    Model(ModelInfo),
    Models(Vec<ModelInfo>),
    ThinkingLevel(String),
    Bash(XyBashResult),
    Compacted(bool),
    SessionStats(serde_json::Value),
    ExportedPath(String),
    NewSession(String),
    SwitchedSession(String),
    Messages {
        session_id: String,
        entries: Vec<SessionEntry>,
        has_more: bool,
    },
    EstimateContext(crate::model::ContextTokenEstimate),
    SessionTree(Vec<SessionTreeNode>),
    SessionTreeTravel(SessionTreeTravel),
    Sessions(Vec<SessionListEntry>),
    SessionEntries(Vec<SessionEntry>),
    SessionName(Option<String>),
    Reload(RuntimeReloadReport),
    LoadedResources(LoadedResourcesSnapshot),
    Capabilities(CapabilitySnapshot),
    Empty,
    Commands(Vec<CommandInfo>),
    PluginCommandOutput {
        plugin: String,
        text: String,
    },
    QueueStats {
        steer_count: usize,
        follow_up_count: usize,
    },
}

pub fn validate_nonempty_thinking_level(s: &str) -> Result<(), DispatchRejection> {
    if s.trim().is_empty() {
        return Err(DispatchRejection::invalid_input(
            "thinking level must not be empty",
        ));
    }
    Ok(())
}

pub fn variant_name(cmd: &Command) -> &'static str {
    cmd.into()
}

pub fn transport_variant_error(cmd: &Command) -> DispatchRejection {
    let name: &'static str = cmd.into();
    DispatchRejection::invalid_input(format!(
        "Command variant {name:?} is not handled by shared dispatch; the caller must handle it before calling dispatch()"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinking_level_validation_rejects_only_blank_input() {
        assert!(validate_nonempty_thinking_level("vendor-max").is_ok());
        assert!(validate_nonempty_thinking_level(" high ").is_ok());
        assert!(matches!(
            validate_nonempty_thinking_level(""),
            Err(DispatchRejection::InvalidInput { message, .. })
                if message == "thinking level must not be empty"
        ));
        assert!(validate_nonempty_thinking_level("   ").is_err());
    }

    #[test]
    fn transport_variant_error_names_the_variant() {
        let rejection = transport_variant_error(&Command::Quit {});
        assert!(matches!(
            rejection,
            DispatchRejection::InvalidInput { message, .. }
                if message.contains("Command variant \"Quit\" is not handled by shared dispatch")
        ));
    }
}
