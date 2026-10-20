//! Session `protocol::Command` execution for in-process Host and TUI.
//!
//! In-process: [`SessionCommandExecutor`] drives runtime. Attach: remote Driver
//! forwards a unary to occupancy Host.
//!
//! Not dispatched here:
//! - `Prompt` — event stream, caller run loop
//! - `Quit` — caller loop
//! - `Subscribe` / `ApproveTool` / `AnswerQuestion` — Host occupancy, not this table
//!
//! Command `id` is a transport correlation handle; dispatch ignores it.

use async_trait::async_trait;

use crate::driver::{
    CommandInfo, LoadedResourcesSnapshot, ModelInfo, RuntimeReloadReport, SessionListEntry,
    SessionState, XyDriver,
};
pub use crate::driver_error::XyDriverError;
use xylitol_plugin_sdk::XyBashResult;
use xylitol_plugin_sdk::session::{SessionEntry, SessionTreeNode, SessionTreeTravel};
use xylitol_protocol::Command;

/// Executor for session-level Commands (c2710): a `Command` is the SSOT of a
/// session operation. In-process handlers drive the runtime directly; remote
/// forwards via a single typed unary.
///
/// `Prompt`, `Quit`, `Subscribe`, `ApproveTool`, `AnswerQuestion` are NOT
/// handled by executors — callers must match those before dispatching.
#[async_trait]
pub trait SessionCommandExecutor: Send {
    async fn execute_session_command(
        &mut self,
        cmd: Command,
    ) -> Result<DispatchOutcome, XyDriverError>;
}

/// Result of a session Command that this table handles.
///
/// Callers map it to TUI notice text or Host unary JSON.
#[derive(Debug)]
#[allow(dead_code)] // fields read by Host occupancy + TUI slash (feature-gated)
pub enum DispatchOutcome {
    /// `Abort` — whether an active loop was actually cancelled.
    Aborted { cancelled: bool },
    /// `GetState` — session snapshot.
    State(SessionState),
    /// `SetModel` / `CycleModel` — the now-selected model.
    Model(ModelInfo),
    /// `GetAvailableModels` — the registered models.
    Models(Vec<ModelInfo>),
    /// `SetThinkingLevel` — the level now in effect.
    ThinkingLevel(String),
    /// `Bash` — the execution result.
    Bash(XyBashResult),
    /// `Compact` — whether a compaction occurred.
    Compacted(bool),
    /// `GetSessionStats` — serialized stats payload (kept as serde_json::Value
    /// to avoid leaking the agent's SessionStats struct verbatim through the
    /// outcome; callers already serialize it).
    SessionStats(serde_json::Value),
    /// `ExportHtml` / `ExportJsonl` — the path written.
    ExportedPath(String),
    /// `ImportJsonl` / `Fork` — the new session id produced.
    NewSession(String),
    /// `SwitchSession` — the session id now active.
    SwitchedSession(String),
    /// `GetMessages` — the loaded entries.
    Messages {
        session_id: String,
        entries: Vec<SessionEntry>,
    },
    /// `EstimateContext` — the host-side overhead-aware estimate (pa-map5).
    EstimateContext(xylitol_plugin_sdk::model::ContextTokenEstimate),
    /// `SessionTree` — the current session message tree.
    SessionTree(Vec<SessionTreeNode>),
    /// `TravelSessionTree` — the selected tree position and editor prefill.
    SessionTreeTravel(SessionTreeTravel),
    /// `ListSessions` — resumable session rows.
    Sessions(Vec<SessionListEntry>),
    /// `LoadSessionEntries` — raw entries for a named session.
    SessionEntries(Vec<SessionEntry>),
    /// `GetSessionName` / `SetSessionName` — current display name.
    SessionName(Option<String>),
    /// `Reload` — host runtime resource reload report.
    Reload(RuntimeReloadReport),
    /// `LoadedResources` — host resource snapshot.
    LoadedResources(LoadedResourcesSnapshot),
    /// Mutating command with no value payload.
    Empty,
    /// `GetCommands` — the available slash commands.
    Commands(Vec<CommandInfo>),
    /// `Steer` / `FollowUp` / `ClearQueue` — current queue depths.
    QueueStats {
        steer_count: usize,
        follow_up_count: usize,
    },
}

/// Validate a freeform thinking-level request without changing its spelling.
pub fn validate_nonempty_thinking_level(s: &str) -> Result<(), XyDriverError> {
    if s.trim().is_empty() {
        return Err(XyDriverError::invalid_input(
            "thinking level must not be empty",
        ));
    }
    Ok(())
}

/// Variant name for a Command (log `where` / error messages).
fn variant_name(cmd: &Command) -> &'static str {
    cmd.into()
}

/// Dispatch a session Command against `driver`.
///
/// `Prompt`, `Quit`, `Subscribe`, `ApproveTool`, `AnswerQuestion` are not
/// handled here — callers must match those first.
pub async fn dispatch(
    driver: &mut dyn XyDriver,
    cmd: Command,
) -> Result<DispatchOutcome, XyDriverError> {
    // `XyDriver: SessionCommandExecutor` — every driver executes commands;
    // in-process directly, remote over a single typed unary (c2710).
    let where_ = format!("dispatch.{}", variant_name(&cmd));
    match driver.execute_session_command(cmd).await {
        Ok(outcome) => Ok(outcome),
        Err(err) => {
            err.log_failure(&where_);
            Err(err)
        }
    }
}

/// Shared helper: reject transport-owned Command variants inside an executor.
///
/// Executors MUST reach this (or an equivalent error) for the five variants
/// the shared path does not own.
pub fn transport_variant_error(cmd: &Command) -> XyDriverError {
    let name: &'static str = cmd.into();
    XyDriverError::invalid_input(format!(
        "Command variant {name:?} is not handled by shared dispatch; the caller must handle it before calling dispatch()"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::ModelInfo;
    use async_trait::async_trait;

    /// Canned executor stub: dispatch is a thin wrapper (c2710) — the executor
    /// owns the Command semantics, so these tests only pin the wrapper contract
    /// (outcome passthrough, transport-variant rejection, error logging path).
    struct StubDriver {
        thinking: String,
    }

    #[async_trait]
    impl XyDriver for StubDriver {
        async fn run(&mut self, _prompt: &str) -> crate::driver::EventStream {
            unimplemented!()
        }
        fn abort(&self) {}
        fn current_model(&self) -> Option<ModelInfo> {
            Some(ModelInfo {
                id: "m1".into(),
                display_name: "M1".into(),
                thinking: true,
                thinking_levels: Vec::new(),
                context_window: 100,
            })
        }
        fn available_models(&self) -> Vec<ModelInfo> {
            vec![self.current_model().unwrap()]
        }
        fn thinking_level(&self) -> String {
            self.thinking.clone()
        }
        fn session_id(&self) -> Option<String> {
            None
        }
        fn get_commands(&self) -> Vec<CommandInfo> {
            Vec::new()
        }
        async fn estimate_context_tokens(
            &self,
        ) -> Result<xylitol_plugin_sdk::model::ContextTokenEstimate, XyDriverError> {
            Err(XyDriverError::unsupported("stub"))
        }
        fn leaf_entry_id(&self) -> Option<String> {
            None
        }
        async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
            LoadedResourcesSnapshot::default()
        }
    }

    #[async_trait]
    impl SessionCommandExecutor for StubDriver {
        async fn execute_session_command(
            &mut self,
            cmd: Command,
        ) -> Result<DispatchOutcome, XyDriverError> {
            match cmd {
                Command::SetThinkingLevel { level, .. } => {
                    validate_nonempty_thinking_level(&level)?;
                    self.thinking = level.clone();
                    Ok(DispatchOutcome::ThinkingLevel(level))
                }
                Command::SetModel { model_id, .. } => Ok(DispatchOutcome::Model(ModelInfo {
                    id: model_id.clone(),
                    display_name: model_id,
                    thinking: false,
                    thinking_levels: Vec::new(),
                    context_window: 8_000,
                })),
                Command::GetState { .. } => Ok(DispatchOutcome::State(SessionState {
                    session_id: "s".into(),
                    model: self.current_model(),
                    thinking_level: self.thinking.clone(),
                    leaf_entry_id: None,
                })),
                Command::GetAvailableModels { .. } => {
                    Ok(DispatchOutcome::Models(self.available_models()))
                }
                other => Err(transport_variant_error(&other)),
            }
        }
    }

    fn stub() -> StubDriver {
        StubDriver {
            thinking: "off".into(),
        }
    }

    #[tokio::test]
    async fn dispatch_passes_executor_outcome_through() {
        let mut driver = stub();
        let outcome = dispatch(
            &mut driver,
            Command::SetModel {
                provider: String::new(),
                model_id: "m2".into(),
            },
        )
        .await
        .expect("ok");
        assert!(matches!(outcome, DispatchOutcome::Model(m) if m.id == "m2"));
    }

    #[tokio::test]
    async fn set_thinking_level_validates_and_round_trips_via_executor() {
        let mut driver = stub();
        let outcome = dispatch(
            &mut driver,
            Command::SetThinkingLevel {
                level: "high".into(),
            },
        )
        .await
        .expect("ok");
        assert!(matches!(outcome, DispatchOutcome::ThinkingLevel(l) if l == "high"));
        assert_eq!(driver.thinking, "high");
    }

    #[tokio::test]
    async fn blank_thinking_level_rejected() {
        let mut driver = stub();
        let err = dispatch(
            &mut driver,
            Command::SetThinkingLevel { level: "  ".into() },
        )
        .await
        .expect_err("blank rejected");
        assert_eq!(err.kind(), "InvalidInput");
    }

    #[tokio::test]
    async fn transport_variants_rejected() {
        let mut driver = stub();
        for cmd in [
            Command::Prompt {
                message: "hi".into(),
            },
            Command::Quit {},
            Command::Subscribe {
                session_id: "s".into(),
                last_seq: 0,
            },
            Command::ApproveTool {
                call_id: "c".into(),
                approved: true,
            },
            Command::AnswerQuestion {
                call_id: "c".into(),
                answer: "a".into(),
            },
        ] {
            let err = dispatch(&mut driver, cmd).await.expect_err("rejected");
            assert_eq!(err.kind(), "InvalidInput");
        }
    }

    #[test]
    fn thinking_level_validation_rejects_only_blank_input() {
        assert!(validate_nonempty_thinking_level("vendor-max").is_ok());
        assert!(validate_nonempty_thinking_level(" high ").is_ok());
        assert!(validate_nonempty_thinking_level("").is_err());
        assert!(validate_nonempty_thinking_level("   ").is_err());
    }
}
